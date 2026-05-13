use crate::binary::{BinaryReader, BinaryWriter};
use crate::model::{PexVersion, ensure_u16, validate_counted_str};
use crate::{
    PEX_MAGIC, PexDebugFunctionInfo, PexDebugFunctionType, PexDebugInfo, PexError, PexFile,
    PexFunction, PexHeader, PexInstruction, PexLocal, PexObject, PexOpcode, PexParameter,
    PexProperty, PexState, PexStringId, PexUserFlag, PexValue, PexVariable,
};

const SKYRIM_GAME_ID: u16 = 1;

impl PexFile {
    pub fn read_from_slice(bytes: &[u8]) -> Result<Self, PexError> {
        let mut reader = BinaryReader::new(bytes);
        let magic = reader.read_magic()?;
        if magic != PEX_MAGIC {
            return Err(PexError::InvalidMagic { value: magic });
        }

        let major = reader.read_u8("major version")?;
        let minor = reader.read_u8("minor version")?;
        if !matches!((major, minor), (3, 1) | (3, 2)) {
            return Err(PexError::UnsupportedVersion { major, minor });
        }
        let game_id = reader.read_u16("game id")?;
        if game_id != SKYRIM_GAME_ID {
            return Err(PexError::UnsupportedGame { game_id });
        }

        let compilation_time = reader.read_u64("compilation time")?;
        let source_file_name = reader.read_counted_str("source file name")?;
        let user_name = reader.read_counted_str("user name")?;
        let computer_name = reader.read_counted_str("computer name")?;
        let string_count = usize::from(reader.read_u16("string table count")?);
        let mut strings = Vec::with_capacity(string_count);
        for _ in 0..string_count {
            strings.push(reader.read_counted_str("string table entry")?);
        }

        let debug_info = (reader.read_u8("debug info flag")? != 0)
            .then(|| read_debug_info(&mut reader, strings.len()))
            .transpose()?;
        let user_flag_count = usize::from(reader.read_u16("user flag table count")?);
        let mut user_flags = Vec::with_capacity(user_flag_count);
        for _ in 0..user_flag_count {
            user_flags.push(PexUserFlag {
                name: reader.read_string_id(strings.len(), "user flag name")?,
                bit_index: reader.read_u8("user flag bit index")?,
            });
        }

        let object_count = usize::from(reader.read_u16("object table count")?);
        let mut objects = Vec::with_capacity(object_count);
        for _ in 0..object_count {
            objects.push(read_object(&mut reader, strings.len())?);
        }
        if reader.remaining() != 0 {
            return Err(PexError::TrailingBytes {
                offset: reader.offset(),
                len: reader.len(),
            });
        }

        Ok(Self::from_parts(
            PexHeader::read(
                PexVersion::new(major, minor),
                compilation_time,
                source_file_name,
                user_name,
                computer_name,
            ),
            strings,
            debug_info,
            user_flags,
            objects,
        ))
    }

    pub fn write_to_vec(&self) -> Result<Vec<u8>, PexError> {
        validate_for_write(self)?;
        let mut writer = BinaryWriter::big_endian();
        writer.write_u32(PEX_MAGIC);
        writer.write_u8(self.header().pex_version().major());
        writer.write_u8(self.header().pex_version().minor());
        writer.write_u16(SKYRIM_GAME_ID);
        writer.write_u64(self.header().compilation_time());
        writer.write_counted_str("source file name", self.header().source_file_name())?;
        writer.write_counted_str("user name", self.header().user_name())?;
        writer.write_counted_str("computer name", self.header().computer_name())?;

        writer.write_u16(self.string_table().len() as u16);
        for string in self.string_table() {
            writer.write_counted_str("string table entry", string)?;
        }

        if let Some(debug_info) = &self.debug_info {
            writer.write_u8(1);
            write_debug_info(&mut writer, debug_info, self.string_table().len())?;
        } else {
            writer.write_u8(0);
        }

        writer.write_u16(self.user_flags.len() as u16);
        for user_flag in &self.user_flags {
            writer.write_string_id(user_flag.name, self.string_table().len())?;
            writer.write_u8(user_flag.bit_index);
        }

        writer.write_u16(self.objects.len() as u16);
        for object in &self.objects {
            write_object(&mut writer, object, self.string_table().len())?;
        }
        Ok(writer.into_bytes())
    }
}

fn read_debug_info(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexDebugInfo, PexError> {
    let modification_time = reader.read_u64("debug modification time")?;
    let count = usize::from(reader.read_u16("debug function count")?);
    let mut functions = Vec::with_capacity(count);
    for _ in 0..count {
        let object_name = reader.read_string_id(string_table_len, "debug object name")?;
        let state_name = reader.read_string_id(string_table_len, "debug state name")?;
        let function_name = reader.read_string_id(string_table_len, "debug function name")?;
        let offset = reader.offset();
        let function_type = match reader.read_u8("debug function type")? {
            0 => PexDebugFunctionType::Normal,
            1 => PexDebugFunctionType::Getter,
            2 => PexDebugFunctionType::Setter,
            tag => return Err(PexError::InvalidDebugFunctionType { offset, tag }),
        };
        let line_count = usize::from(reader.read_u16("debug line map count")?);
        let mut instruction_line_map = Vec::with_capacity(line_count);
        for _ in 0..line_count {
            instruction_line_map.push(reader.read_u16("debug line")?);
        }
        functions.push(PexDebugFunctionInfo {
            object_name,
            state_name,
            function_name,
            function_type,
            instruction_line_map,
        });
    }
    Ok(PexDebugInfo {
        modification_time,
        functions,
    })
}

fn read_object(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexObject, PexError> {
    let object_offset = reader.offset();
    let name = reader.read_string_id(string_table_len, "object name")?;
    let raw_body_len = reader.read_u32("object body length")? as usize;
    let body_start = reader.offset();
    let mut body_end =
        body_start
            .checked_add(raw_body_len)
            .ok_or(PexError::ObjectSizeMismatch {
                offset: object_offset,
                expected_end: usize::MAX,
                actual_end: body_start,
            })?;
    if body_end > reader.len()
        && raw_body_len >= 4
        && let Some(ck_body_end) = body_start.checked_add(raw_body_len - 4)
        && ck_body_end <= reader.len()
    {
        body_end = ck_body_end;
    }
    if reader.len() < body_end {
        return Err(PexError::Truncated {
            offset: reader.offset(),
            needed: body_end - reader.offset(),
            remaining: reader.remaining(),
            what: "object body",
        });
    }

    let parent_class_name = reader.read_string_id(string_table_len, "object parent class")?;
    let documentation_string = reader.read_string_id(string_table_len, "object documentation")?;
    let user_flags = reader.read_u32("object user flags")?;
    let auto_state_name = reader.read_string_id(string_table_len, "object auto state")?;

    let variable_count = usize::from(reader.read_u16("variable count")?);
    let mut variables = Vec::with_capacity(variable_count);
    for _ in 0..variable_count {
        variables.push(read_variable(reader, string_table_len)?);
    }

    let property_count = usize::from(reader.read_u16("property count")?);
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        properties.push(read_property(reader, string_table_len)?);
    }

    let state_count = usize::from(reader.read_u16("state count")?);
    let mut states = Vec::with_capacity(state_count);
    for _ in 0..state_count {
        states.push(read_state(reader, string_table_len)?);
    }

    if reader.offset() != body_end {
        return Err(PexError::ObjectSizeMismatch {
            offset: object_offset,
            expected_end: body_end,
            actual_end: reader.offset(),
        });
    }

    Ok(PexObject {
        name,
        parent_class_name,
        documentation_string,
        user_flags,
        auto_state_name,
        variables,
        properties,
        states,
    })
}

fn read_variable(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexVariable, PexError> {
    Ok(PexVariable {
        name: reader.read_string_id(string_table_len, "variable name")?,
        type_name: reader.read_string_id(string_table_len, "variable type")?,
        user_flags: reader.read_u32("variable user flags")?,
        default_value: read_value(reader, string_table_len)?,
    })
}

fn read_property(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexProperty, PexError> {
    let name = reader.read_string_id(string_table_len, "property name")?;
    let type_name = reader.read_string_id(string_table_len, "property type")?;
    let documentation_string = reader.read_string_id(string_table_len, "property documentation")?;
    let user_flags = reader.read_u32("property user flags")?;
    let flags = reader.read_u8("property flags")?;
    let is_readable = flags & 0x01 != 0;
    let is_writable = flags & 0x02 != 0;
    let is_auto = flags & 0x04 != 0;
    let (auto_var, read_function, write_function) = if is_auto {
        (
            Some(reader.read_string_id(string_table_len, "property auto variable")?),
            None,
            None,
        )
    } else {
        let getter = is_readable
            .then(|| read_function(reader, string_table_len, false, Some(name)))
            .transpose()?;
        let setter = is_writable
            .then(|| read_function(reader, string_table_len, false, Some(name)))
            .transpose()?;
        (None, getter, setter)
    };

    Ok(PexProperty {
        name,
        type_name,
        documentation_string,
        user_flags,
        is_readable,
        is_writable,
        is_auto,
        auto_var,
        read_function,
        write_function,
    })
}

fn read_state(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexState, PexError> {
    let name = reader.read_string_id(string_table_len, "state name")?;
    let count = usize::from(reader.read_u16("state function count")?);
    let mut functions = Vec::with_capacity(count);
    for _ in 0..count {
        functions.push(read_function(reader, string_table_len, true, None)?);
    }
    Ok(PexState { name, functions })
}

fn read_function(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
    has_name: bool,
    property_name: Option<PexStringId>,
) -> Result<PexFunction, PexError> {
    let name = if has_name {
        reader.read_string_id(string_table_len, "function name")?
    } else {
        property_name.unwrap_or(PexStringId::new(0))
    };
    let return_type_name = reader.read_string_id(string_table_len, "function return type")?;
    let documentation_string = reader.read_string_id(string_table_len, "function documentation")?;
    let user_flags = reader.read_u32("function user flags")?;
    let flags = reader.read_u8("function flags")?;
    let is_global = flags & 0x01 != 0;
    let is_native = flags & 0x02 != 0;
    let parameter_count = usize::from(reader.read_u16("parameter count")?);
    let mut parameters = Vec::with_capacity(parameter_count);
    for _ in 0..parameter_count {
        parameters.push(PexParameter {
            name: reader.read_string_id(string_table_len, "parameter name")?,
            type_name: reader.read_string_id(string_table_len, "parameter type")?,
        });
    }
    let local_count = usize::from(reader.read_u16("local count")?);
    let mut locals = Vec::with_capacity(local_count);
    for _ in 0..local_count {
        locals.push(PexLocal {
            name: reader.read_string_id(string_table_len, "local name")?,
            type_name: reader.read_string_id(string_table_len, "local type")?,
        });
    }
    let instruction_count = usize::from(reader.read_u16("instruction count")?);
    let mut instructions = Vec::with_capacity(instruction_count);
    for _ in 0..instruction_count {
        instructions.push(read_instruction(reader, string_table_len)?);
    }
    Ok(PexFunction {
        name,
        return_type_name,
        documentation_string,
        user_flags,
        is_global,
        is_native,
        parameters,
        locals,
        instructions,
    })
}

fn read_instruction(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexInstruction, PexError> {
    let offset = reader.offset();
    let opcode_byte = reader.read_u8("instruction opcode")?;
    let opcode = PexOpcode::from_byte(opcode_byte).ok_or(PexError::UnknownOpcode {
        offset,
        opcode: opcode_byte,
    })?;
    let mut arguments = Vec::with_capacity(opcode.fixed_arg_count());
    for _ in 0..opcode.fixed_arg_count() {
        arguments.push(read_value(reader, string_table_len)?);
    }
    let mut variadic_arguments = Vec::new();
    if opcode.has_variadic_arguments() {
        let offset = reader.offset();
        let count = read_value(reader, string_table_len)?;
        let PexValue::Integer(count) = count else {
            return Err(PexError::MalformedVariadicCount { offset, opcode });
        };
        let count = usize::try_from(count)
            .map_err(|_| PexError::MalformedVariadicCount { offset, opcode })?;
        if count > reader.remaining() {
            return Err(PexError::MalformedVariadicCount { offset, opcode });
        }
        for _ in 0..count {
            variadic_arguments.push(read_value(reader, string_table_len)?);
        }
    }
    Ok(PexInstruction {
        opcode,
        arguments,
        variadic_arguments,
    })
}

fn read_value(
    reader: &mut BinaryReader<'_>,
    string_table_len: usize,
) -> Result<PexValue, PexError> {
    let offset = reader.offset();
    match reader.read_u8("value type")? {
        0 => Ok(PexValue::None),
        1 => Ok(PexValue::Identifier(
            reader.read_string_id(string_table_len, "identifier value")?,
        )),
        2 => Ok(PexValue::String(
            reader.read_string_id(string_table_len, "string value")?,
        )),
        3 => Ok(PexValue::Integer(reader.read_i32("integer value")?)),
        4 => Ok(PexValue::Float(reader.read_f32("float value")?)),
        5 => Ok(PexValue::Bool(reader.read_u8("bool value")? != 0)),
        tag => Err(PexError::UnknownValueType { offset, tag }),
    }
}

fn write_debug_info(
    writer: &mut BinaryWriter,
    debug_info: &PexDebugInfo,
    string_table_len: usize,
) -> Result<(), PexError> {
    writer.write_u64(debug_info.modification_time);
    writer.write_u16(debug_info.functions.len() as u16);
    for function in &debug_info.functions {
        writer.write_string_id(function.object_name, string_table_len)?;
        writer.write_string_id(function.state_name, string_table_len)?;
        writer.write_string_id(function.function_name, string_table_len)?;
        writer.write_u8(function.function_type as u8);
        writer.write_u16(function.instruction_line_map.len() as u16);
        for line in &function.instruction_line_map {
            writer.write_u16(*line);
        }
    }
    Ok(())
}

fn write_object(
    writer: &mut BinaryWriter,
    object: &PexObject,
    string_table_len: usize,
) -> Result<(), PexError> {
    writer.write_string_id(object.name, string_table_len)?;
    let mut body = BinaryWriter::big_endian();
    body.write_string_id(object.parent_class_name, string_table_len)?;
    body.write_string_id(object.documentation_string, string_table_len)?;
    body.write_u32(object.user_flags);
    body.write_string_id(object.auto_state_name, string_table_len)?;
    body.write_u16(object.variables.len() as u16);
    for variable in &object.variables {
        write_variable(&mut body, variable, string_table_len)?;
    }
    body.write_u16(object.properties.len() as u16);
    for property in &object.properties {
        write_property(&mut body, property, string_table_len)?;
    }
    body.write_u16(object.states.len() as u16);
    for state in &object.states {
        write_state(&mut body, state, string_table_len)?;
    }
    let body = body.into_bytes();
    if body.len() > u32::MAX as usize {
        return Err(PexError::ObjectTooLarge { len: body.len() });
    }
    writer.write_u32(body.len() as u32);
    writer.extend(body);
    Ok(())
}

fn write_variable(
    writer: &mut BinaryWriter,
    variable: &PexVariable,
    string_table_len: usize,
) -> Result<(), PexError> {
    writer.write_string_id(variable.name, string_table_len)?;
    writer.write_string_id(variable.type_name, string_table_len)?;
    writer.write_u32(variable.user_flags);
    write_value(writer, variable.default_value, string_table_len)
}

fn write_property(
    writer: &mut BinaryWriter,
    property: &PexProperty,
    string_table_len: usize,
) -> Result<(), PexError> {
    writer.write_string_id(property.name, string_table_len)?;
    writer.write_string_id(property.type_name, string_table_len)?;
    writer.write_string_id(property.documentation_string, string_table_len)?;
    writer.write_u32(property.user_flags);
    let mut flags = 0u8;
    if property.is_readable {
        flags |= 0x01;
    }
    if property.is_writable {
        flags |= 0x02;
    }
    if property.is_auto {
        flags |= 0x04;
    }
    writer.write_u8(flags);
    if property.is_auto {
        writer.write_string_id(
            property
                .auto_var
                .ok_or(PexError::AutoPropertyMissingAutoVar)?,
            string_table_len,
        )?;
    } else {
        if property.is_readable {
            write_function(
                writer,
                property
                    .read_function
                    .as_ref()
                    .ok_or(PexError::ReadablePropertyMissingGetter)?,
                string_table_len,
                false,
            )?;
        }
        if property.is_writable {
            write_function(
                writer,
                property
                    .write_function
                    .as_ref()
                    .ok_or(PexError::WritablePropertyMissingSetter)?,
                string_table_len,
                false,
            )?;
        }
    }
    Ok(())
}

fn write_state(
    writer: &mut BinaryWriter,
    state: &PexState,
    string_table_len: usize,
) -> Result<(), PexError> {
    writer.write_string_id(state.name, string_table_len)?;
    writer.write_u16(state.functions.len() as u16);
    for function in &state.functions {
        write_function(writer, function, string_table_len, true)?;
    }
    Ok(())
}

fn write_function(
    writer: &mut BinaryWriter,
    function: &PexFunction,
    string_table_len: usize,
    include_name: bool,
) -> Result<(), PexError> {
    if include_name {
        writer.write_string_id(function.name, string_table_len)?;
    }
    writer.write_string_id(function.return_type_name, string_table_len)?;
    writer.write_string_id(function.documentation_string, string_table_len)?;
    writer.write_u32(function.user_flags);
    let mut flags = 0u8;
    if function.is_global {
        flags |= 0x01;
    }
    if function.is_native {
        flags |= 0x02;
    }
    writer.write_u8(flags);
    writer.write_u16(function.parameters.len() as u16);
    for parameter in &function.parameters {
        writer.write_string_id(parameter.name, string_table_len)?;
        writer.write_string_id(parameter.type_name, string_table_len)?;
    }
    writer.write_u16(function.locals.len() as u16);
    for local in &function.locals {
        writer.write_string_id(local.name, string_table_len)?;
        writer.write_string_id(local.type_name, string_table_len)?;
    }
    writer.write_u16(function.instructions.len() as u16);
    for instruction in &function.instructions {
        write_instruction(writer, instruction, string_table_len)?;
    }
    Ok(())
}

fn write_instruction(
    writer: &mut BinaryWriter,
    instruction: &PexInstruction,
    string_table_len: usize,
) -> Result<(), PexError> {
    if instruction.arguments.len() != instruction.opcode.fixed_arg_count() {
        return Err(PexError::InvalidInstructionArity {
            opcode: instruction.opcode,
            expected: instruction.opcode.fixed_arg_count(),
            actual: instruction.arguments.len(),
        });
    }
    writer.write_u8(instruction.opcode as u8);
    for argument in &instruction.arguments {
        write_value(writer, *argument, string_table_len)?;
    }
    if instruction.opcode.has_variadic_arguments() {
        writer.write_u8(3);
        writer.write_i32(instruction.variadic_arguments.len() as i32);
        for argument in &instruction.variadic_arguments {
            write_value(writer, *argument, string_table_len)?;
        }
    } else if !instruction.variadic_arguments.is_empty() {
        return Err(PexError::UnexpectedVariadicArguments {
            opcode: instruction.opcode,
        });
    }
    Ok(())
}

fn write_value(
    writer: &mut BinaryWriter,
    value: PexValue,
    string_table_len: usize,
) -> Result<(), PexError> {
    match value {
        PexValue::None => writer.write_u8(0),
        PexValue::Identifier(id) => {
            writer.write_u8(1);
            writer.write_string_id(id, string_table_len)?;
        }
        PexValue::String(id) => {
            writer.write_u8(2);
            writer.write_string_id(id, string_table_len)?;
        }
        PexValue::Integer(value) => {
            writer.write_u8(3);
            writer.write_i32(value);
        }
        PexValue::Float(value) => {
            writer.write_u8(4);
            writer.write_f32(value);
        }
        PexValue::Bool(value) => {
            writer.write_u8(5);
            writer.write_u8(u8::from(value));
        }
    }
    Ok(())
}

fn validate_for_write(file: &PexFile) -> Result<(), PexError> {
    let version = file.header().pex_version();
    if !matches!((version.major(), version.minor()), (3, 1) | (3, 2)) {
        return Err(PexError::UnsupportedVersion {
            major: version.major(),
            minor: version.minor(),
        });
    }
    validate_counted_str("source file name", file.header().source_file_name())?;
    validate_counted_str("user name", file.header().user_name())?;
    validate_counted_str("computer name", file.header().computer_name())?;
    ensure_u16("string table", file.string_table().len())?;
    for string in file.string_table() {
        validate_counted_str("string table entry", string)?;
    }
    ensure_u16("user flag table", file.user_flags.len())?;
    for user_flag in &file.user_flags {
        validate_string_id(user_flag.name, file.string_table().len())?;
    }
    ensure_u16("object table", file.objects.len())?;
    for object in &file.objects {
        validate_object(object, file.string_table().len())?;
    }
    if let Some(debug_info) = &file.debug_info {
        ensure_u16("debug function table", debug_info.functions.len())?;
        for function in &debug_info.functions {
            validate_string_id(function.object_name, file.string_table().len())?;
            validate_string_id(function.state_name, file.string_table().len())?;
            validate_string_id(function.function_name, file.string_table().len())?;
            ensure_u16("debug line map", function.instruction_line_map.len())?;
        }
    }
    Ok(())
}

fn validate_object(object: &PexObject, string_table_len: usize) -> Result<(), PexError> {
    validate_string_id(object.name, string_table_len)?;
    validate_string_id(object.parent_class_name, string_table_len)?;
    validate_string_id(object.documentation_string, string_table_len)?;
    validate_string_id(object.auto_state_name, string_table_len)?;
    ensure_u16("object variable table", object.variables.len())?;
    ensure_u16("property table", object.properties.len())?;
    ensure_u16("state table", object.states.len())?;
    for variable in &object.variables {
        validate_string_id(variable.name, string_table_len)?;
        validate_string_id(variable.type_name, string_table_len)?;
        validate_value(variable.default_value, string_table_len)?;
    }
    for property in &object.properties {
        validate_string_id(property.name, string_table_len)?;
        validate_string_id(property.type_name, string_table_len)?;
        validate_string_id(property.documentation_string, string_table_len)?;
        if let Some(auto_var) = property.auto_var {
            validate_string_id(auto_var, string_table_len)?;
        }
        if let Some(function) = &property.read_function {
            validate_function(function, string_table_len)?;
        }
        if let Some(function) = &property.write_function {
            validate_function(function, string_table_len)?;
        }
    }
    for state in &object.states {
        validate_string_id(state.name, string_table_len)?;
        ensure_u16("function table", state.functions.len())?;
        for function in &state.functions {
            validate_function(function, string_table_len)?;
        }
    }
    Ok(())
}

fn validate_function(function: &PexFunction, string_table_len: usize) -> Result<(), PexError> {
    validate_string_id(function.name, string_table_len)?;
    validate_string_id(function.return_type_name, string_table_len)?;
    validate_string_id(function.documentation_string, string_table_len)?;
    ensure_u16("parameter table", function.parameters.len())?;
    for parameter in &function.parameters {
        validate_string_id(parameter.name, string_table_len)?;
        validate_string_id(parameter.type_name, string_table_len)?;
    }
    ensure_u16("local table", function.locals.len())?;
    for local in &function.locals {
        validate_string_id(local.name, string_table_len)?;
        validate_string_id(local.type_name, string_table_len)?;
    }
    ensure_u16("instruction table", function.instructions.len())?;
    for instruction in &function.instructions {
        if instruction.arguments.len() != instruction.opcode.fixed_arg_count() {
            return Err(PexError::InvalidInstructionArity {
                opcode: instruction.opcode,
                expected: instruction.opcode.fixed_arg_count(),
                actual: instruction.arguments.len(),
            });
        }
        if instruction.variadic_arguments.len() > i32::MAX as usize {
            return Err(PexError::VariadicArgumentCountTooLarge {
                len: instruction.variadic_arguments.len(),
            });
        }
        for value in &instruction.arguments {
            validate_value(*value, string_table_len)?;
        }
        for value in &instruction.variadic_arguments {
            validate_value(*value, string_table_len)?;
        }
    }
    Ok(())
}

fn validate_value(value: PexValue, string_table_len: usize) -> Result<(), PexError> {
    match value {
        PexValue::Identifier(id) | PexValue::String(id) => validate_string_id(id, string_table_len),
        PexValue::None | PexValue::Integer(_) | PexValue::Float(_) | PexValue::Bool(_) => Ok(()),
    }
}

fn validate_string_id(id: PexStringId, table_len: usize) -> Result<(), PexError> {
    if id.index() as usize >= table_len {
        return Err(PexError::WriteStringIdOutOfRange { id, table_len });
    }
    Ok(())
}
