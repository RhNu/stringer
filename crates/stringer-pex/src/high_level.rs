use bytes::Bytes;
use stringer_core::{
    FileAsset, FileRole, PexCallContext, PexConcatMetadata, PexConcatPart, PexFunctionKind,
    PexOperandPath, PexStringMetadata, StringEntry, StringEntryBundle, StringEntryContext,
    StringEntrySource,
};
use stringer_extraction_filter::ExtractionFilterSet;
use tracing::{debug, instrument, trace};

use crate::filter::{PexStringFilter, PexStringFilterInput};
use crate::{PexError, PexFile, PexFunction, PexInstruction, PexOpcode, PexStringId, PexValue};

#[derive(Debug, Clone, Default)]
pub struct ReadPexOptions {
    extraction_filters: ExtractionFilterSet,
}

impl ReadPexOptions {
    pub fn with_extraction_filters(mut self, filters: ExtractionFilterSet) -> Self {
        self.extraction_filters = filters;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ParsedPex {
    original: FileAsset,
    file: PexFile,
    dirty: bool,
}

impl ParsedPex {
    pub fn file(&self) -> &PexFile {
        &self.file
    }

    pub fn file_mut(&mut self) -> &mut PexFile {
        self.dirty = true;
        &mut self.file
    }

    pub fn path(&self) -> &str {
        self.original.path().as_str()
    }
}

#[derive(Debug, Clone)]
pub struct PexStringBundle {
    parsed: ParsedPex,
    entries: Vec<StringEntry>,
    bindings: Vec<PexStringBinding>,
}

impl PexStringBundle {
    pub fn entries(&self) -> &[StringEntry] {
        &self.entries
    }

    pub fn entries_mut(&mut self) -> &mut [StringEntry] {
        &mut self.entries
    }
}

impl StringEntryBundle for PexStringBundle {
    type Entry = StringEntry;

    fn string_entries(&self) -> &[StringEntry] {
        &self.entries
    }

    fn string_entries_mut(&mut self) -> &mut [StringEntry] {
        &mut self.entries
    }
}

#[derive(Debug, Clone)]
struct PexStringBinding {
    entry_id: String,
    string_id: PexStringId,
    slot: FunctionSlot,
    instruction_index: usize,
    operand: PexOperandPath,
}

#[derive(Debug, Clone)]
enum FunctionSlot {
    State {
        object_index: usize,
        state_index: usize,
        function_index: usize,
    },
    PropertyGetter {
        object_index: usize,
        property_index: usize,
    },
    PropertySetter {
        object_index: usize,
        property_index: usize,
    },
}

#[derive(Debug, Default)]
struct FunctionExtractionState {
    next_concat_group: usize,
    active_concat: Option<ActiveConcat>,
    concat_groups: Vec<ConcatGroup>,
}

#[derive(Debug, Clone)]
struct ActiveConcat {
    destination: Option<String>,
    group_index: usize,
}

#[derive(Debug, Default)]
struct ConcatGroup {
    entry_indexes: Vec<usize>,
    parts: Vec<PexConcatPart>,
}

#[instrument(skip(asset), fields(path = %asset.path()), err)]
pub fn parse_pex_file(asset: &FileAsset) -> Result<ParsedPex, PexError> {
    if asset.role() != FileRole::Pex {
        return Err(PexError::unsupported_file(
            asset.path().to_string(),
            "expected .pex script",
        ));
    }
    let file = PexFile::read_from_slice(asset.bytes())?;
    debug!(
        strings = file.string_table().len(),
        objects = file.objects.len(),
        "parsed pex file"
    );
    Ok(ParsedPex {
        original: asset.clone(),
        file,
        dirty: false,
    })
}

#[instrument(skip(parsed), fields(path = parsed.path()), err)]
pub fn write_pex_file(parsed: &ParsedPex) -> Result<FileAsset, PexError> {
    if !parsed.dirty {
        trace!("preserving unmodified pex bytes");
        return Ok(parsed.original.clone());
    }
    let bytes = parsed.file.write_to_vec()?;
    debug!(bytes = bytes.len(), "wrote pex file");
    Ok(FileAsset::new(
        parsed.original.path().to_owned(),
        Bytes::from(bytes),
    ))
}

#[instrument(skip(asset), fields(path = %asset.path()), err)]
pub fn read_pex_strings(
    asset: FileAsset,
    options: ReadPexOptions,
) -> Result<PexStringBundle, PexError> {
    let parsed = parse_pex_file(&asset)?;
    let mut entries = Vec::new();
    let mut bindings = Vec::new();
    extract_entries(
        parsed.path(),
        parsed.file(),
        &mut entries,
        &mut bindings,
        &PexStringFilter::with_rules(options.extraction_filters),
    );
    debug!(
        entries = entries.len(),
        bindings = bindings.len(),
        "read pex strings"
    );
    Ok(PexStringBundle {
        parsed,
        entries,
        bindings,
    })
}

#[instrument(skip(bundle), err)]
pub fn write_pex_strings(mut bundle: PexStringBundle) -> Result<FileAsset, PexError> {
    if !bundle.entries.iter().any(StringEntry::is_dirty) {
        trace!("preserving unmodified pex string bundle");
        return write_pex_file(&bundle.parsed);
    }

    let dirty_entries = bundle
        .entries
        .iter()
        .filter(|entry| entry.is_dirty())
        .count();
    for entry in bundle.entries.iter_mut().filter(|entry| entry.is_dirty()) {
        let binding = bundle
            .bindings
            .iter()
            .find(|binding| binding.entry_id == entry.id())
            .cloned()
            .ok_or_else(|| PexError::InvalidStringEntryBinding {
                entry_id: entry.id().to_string(),
            })?;
        let replacement = entry.text().to_string();
        let reference_count = count_string_references(bundle.parsed.file(), binding.string_id);
        if reference_count <= 1 {
            bundle
                .parsed
                .file_mut()
                .replace_string(binding.string_id, replacement)?;
            trace!(
                string_id = binding.string_id.index(),
                "replaced unique pex string"
            );
        } else {
            let new_id = bundle.parsed.file_mut().intern(&replacement)?;
            let operand =
                instruction_operand_mut(bundle.parsed.file_mut(), &binding).ok_or_else(|| {
                    PexError::InvalidStringEntryBinding {
                        entry_id: binding.entry_id.clone(),
                    }
                })?;
            *operand = PexValue::String(new_id);
            if let StringEntrySource::Pex(metadata) = entry.source_mut() {
                metadata.string_id = new_id.index();
            }
            trace!(
                old_string_id = binding.string_id.index(),
                new_string_id = new_id.index(),
                reference_count,
                "interned replacement for shared pex string"
            );
        }
    }

    debug!(dirty_entries, "wrote pex string edits");
    write_pex_file(&bundle.parsed)
}

fn extract_entries(
    path: &str,
    file: &PexFile,
    entries: &mut Vec<StringEntry>,
    bindings: &mut Vec<PexStringBinding>,
    filter: &PexStringFilter,
) {
    for (object_index, object) in file.objects.iter().enumerate() {
        for (property_index, property) in object.properties.iter().enumerate() {
            if let Some(function) = &property.read_function {
                extract_function(
                    ExtractFunctionInput {
                        path,
                        file,
                        slot: FunctionSlot::PropertyGetter {
                            object_index,
                            property_index,
                        },
                        object_name: string(file, object.name),
                        state_name: "",
                        function_name: string(file, property.name),
                        function_kind: PexFunctionKind::Getter,
                    },
                    function,
                    entries,
                    bindings,
                    filter,
                );
            }
            if let Some(function) = &property.write_function {
                extract_function(
                    ExtractFunctionInput {
                        path,
                        file,
                        slot: FunctionSlot::PropertySetter {
                            object_index,
                            property_index,
                        },
                        object_name: string(file, object.name),
                        state_name: "",
                        function_name: string(file, property.name),
                        function_kind: PexFunctionKind::Setter,
                    },
                    function,
                    entries,
                    bindings,
                    filter,
                );
            }
        }
        for (state_index, state) in object.states.iter().enumerate() {
            for (function_index, function) in state.functions.iter().enumerate() {
                extract_function(
                    ExtractFunctionInput {
                        path,
                        file,
                        slot: FunctionSlot::State {
                            object_index,
                            state_index,
                            function_index,
                        },
                        object_name: string(file, object.name),
                        state_name: string(file, state.name),
                        function_name: string(file, function.name),
                        function_kind: PexFunctionKind::Normal,
                    },
                    function,
                    entries,
                    bindings,
                    filter,
                );
            }
        }
    }
}

struct ExtractFunctionInput<'a> {
    path: &'a str,
    file: &'a PexFile,
    slot: FunctionSlot,
    object_name: &'a str,
    state_name: &'a str,
    function_name: &'a str,
    function_kind: PexFunctionKind,
}

fn extract_function(
    input: ExtractFunctionInput<'_>,
    function: &PexFunction,
    entries: &mut Vec<StringEntry>,
    bindings: &mut Vec<PexStringBinding>,
    filter: &PexStringFilter,
) {
    let mut state = FunctionExtractionState::default();
    for (instruction_index, instruction) in function.instructions.iter().enumerate() {
        let concat_group = update_concat_state(&mut state, input.file, instruction);
        for (argument_index, value) in instruction.arguments.iter().enumerate() {
            let operand = PexOperandPath::Fixed(argument_index);
            extract_value(
                ExtractValueInput {
                    function_input: &input,
                    instruction,
                    instruction_index,
                    operand,
                    concat_group,
                },
                *value,
                entries,
                bindings,
                &mut state,
                filter,
            );
        }
        for (argument_index, value) in instruction.variadic_arguments.iter().enumerate() {
            extract_value(
                ExtractValueInput {
                    function_input: &input,
                    instruction,
                    instruction_index,
                    operand: PexOperandPath::Variadic(argument_index),
                    concat_group,
                },
                *value,
                entries,
                bindings,
                &mut state,
                filter,
            );
        }
    }
    finalize_concat_groups(entries, &state);
}

struct ExtractValueInput<'a, 'b> {
    function_input: &'a ExtractFunctionInput<'b>,
    instruction: &'a PexInstruction,
    instruction_index: usize,
    operand: PexOperandPath,
    concat_group: Option<usize>,
}

fn extract_value(
    input: ExtractValueInput<'_, '_>,
    value: PexValue,
    entries: &mut Vec<StringEntry>,
    bindings: &mut Vec<PexStringBinding>,
    state: &mut FunctionExtractionState,
    filter: &PexStringFilter,
) {
    let PexValue::String(string_id) = value else {
        add_concat_operand_part(input, value, state);
        return;
    };
    if is_skipped_symbol_position(input.instruction.opcode, input.operand) {
        add_concat_operand_part(input, value, state);
        return;
    }

    let text = string(input.function_input.file, string_id).to_string();
    let call_context = call_context(input.function_input.file, input.instruction);
    let filter_input = PexStringFilterInput {
        text: &text,
        path: input.function_input.path,
        object_name: input.function_input.object_name,
        state_name: input.function_input.state_name,
        function_name: input.function_input.function_name,
        function_kind: input.function_input.function_kind,
        opcode: input.instruction.opcode,
        operand: input.operand,
        string_id,
        call_context: call_context.as_ref(),
        in_concat: input.concat_group.is_some(),
    };
    if let Some(reason) = filter.evaluate(&filter_input) {
        add_concat_operand_part(input, value, state);
        trace!(
            ?reason,
            string_id = string_id.index(),
            "filtered pex string"
        );
        return;
    }
    let entry_id = pex_entry_id(
        input.function_input.path,
        input.function_input.object_name,
        input.function_input.state_name,
        input.function_input.function_name,
        input.function_input.function_kind,
        input.instruction_index,
        input.operand,
    );
    let entry_index = entries.len();
    let concat = input.concat_group.map(|group_index| {
        state.concat_groups[group_index]
            .entry_indexes
            .push(entry_index);
        state.concat_groups[group_index]
            .parts
            .push(PexConcatPart::Entry {
                id: entry_id.clone(),
                text: text.clone(),
            });
        PexConcatMetadata {
            group_id: concat_group_id(input.function_input, group_index),
            part_index: 0,
            ambiguous: false,
            parts: Vec::new(),
        }
    });
    entries.push(StringEntry::new(
        entry_id.clone(),
        text,
        StringEntrySource::Pex(PexStringMetadata {
            path: input.function_input.path.into(),
            object: input.function_input.object_name.to_string(),
            state: input.function_input.state_name.to_string(),
            function: input.function_input.function_name.to_string(),
            function_kind: input.function_input.function_kind,
            instruction_index: input.instruction_index,
            opcode: input.instruction.opcode.name().to_string(),
            operand: input.operand,
            string_id: string_id.index(),
            call_context,
            concat,
        }),
        StringEntryContext::default(),
    ));
    bindings.push(PexStringBinding {
        entry_id,
        string_id,
        slot: input.function_input.slot.clone(),
        instruction_index: input.instruction_index,
        operand: input.operand,
    });
}

fn is_skipped_symbol_position(opcode: PexOpcode, operand: PexOperandPath) -> bool {
    let PexOperandPath::Fixed(index) = operand else {
        return false;
    };
    match opcode {
        PexOpcode::CallStatic => index <= 2,
        PexOpcode::CallMethod => index <= 2,
        PexOpcode::CallParent => index <= 1,
        PexOpcode::PropGet => index == 0 || index == 2,
        PexOpcode::PropSet => index == 0,
        PexOpcode::StrCat => index == 0,
        PexOpcode::Assign
        | PexOpcode::Cast
        | PexOpcode::Not
        | PexOpcode::INeg
        | PexOpcode::FNeg
        | PexOpcode::IAdd
        | PexOpcode::FAdd
        | PexOpcode::ISub
        | PexOpcode::FSub
        | PexOpcode::IMul
        | PexOpcode::FMul
        | PexOpcode::IDiv
        | PexOpcode::FDiv
        | PexOpcode::IMod
        | PexOpcode::CmpEq
        | PexOpcode::CmpLt
        | PexOpcode::CmpLte
        | PexOpcode::CmpGt
        | PexOpcode::CmpGte
        | PexOpcode::ArrayCreate
        | PexOpcode::ArrayLength
        | PexOpcode::ArrayGetElement
        | PexOpcode::ArraySetElement
        | PexOpcode::ArrayFindElement
        | PexOpcode::ArrayRFindElement => index == 0,
        PexOpcode::Nop | PexOpcode::Jmp | PexOpcode::JmpT | PexOpcode::JmpF | PexOpcode::Return => {
            false
        }
    }
}

fn update_concat_state(
    state: &mut FunctionExtractionState,
    file: &PexFile,
    instruction: &PexInstruction,
) -> Option<usize> {
    if instruction.opcode != PexOpcode::StrCat || instruction.arguments.len() != 3 {
        state.active_concat = None;
        return None;
    }
    let destination = value_label(file, instruction.arguments[0]);
    let continues = state.active_concat.as_ref().is_some_and(|active| {
        destination == active.destination
            || instruction.arguments[1..]
                .iter()
                .any(|value| value_label(file, *value) == active.destination)
    });
    if continues {
        state
            .active_concat
            .as_ref()
            .map(|active| active.group_index)
    } else {
        let group_index = state.concat_groups.len();
        state.next_concat_group += 1;
        state.concat_groups.push(ConcatGroup::default());
        state.active_concat = Some(ActiveConcat {
            destination,
            group_index,
        });
        Some(group_index)
    }
}

fn add_concat_operand_part(
    input: ExtractValueInput<'_, '_>,
    value: PexValue,
    state: &mut FunctionExtractionState,
) {
    let Some(group_index) = input.concat_group else {
        return;
    };
    if input.instruction.opcode != PexOpcode::StrCat
        || matches!(input.operand, PexOperandPath::Fixed(0))
    {
        return;
    }
    if let Some(label) = value_label(input.function_input.file, value) {
        state.concat_groups[group_index]
            .parts
            .push(PexConcatPart::Operand { label });
    }
}

fn finalize_concat_groups(entries: &mut [StringEntry], state: &FunctionExtractionState) {
    for group in &state.concat_groups {
        for (part_index, entry_index) in group.entry_indexes.iter().enumerate() {
            let StringEntrySource::Pex(metadata) = entries[*entry_index].source_mut() else {
                continue;
            };
            if let Some(concat) = &mut metadata.concat {
                concat.part_index = part_index;
                concat.parts = group.parts.clone();
            }
        }
    }
}

fn call_context(file: &PexFile, instruction: &PexInstruction) -> Option<PexCallContext> {
    let (target, member) = match instruction.opcode {
        PexOpcode::CallStatic if instruction.arguments.len() >= 2 => (
            value_label(file, instruction.arguments[0]),
            value_label(file, instruction.arguments[1]),
        ),
        PexOpcode::CallMethod if instruction.arguments.len() >= 2 => (
            value_label(file, instruction.arguments[1]),
            value_label(file, instruction.arguments[0]),
        ),
        PexOpcode::CallParent if !instruction.arguments.is_empty() => {
            (None, value_label(file, instruction.arguments[0]))
        }
        _ => return None,
    };
    Some(PexCallContext {
        opcode: instruction.opcode.name().to_string(),
        target,
        member,
    })
}

fn pex_entry_id(
    path: &str,
    object: &str,
    state: &str,
    function: &str,
    function_kind: PexFunctionKind,
    instruction_index: usize,
    operand: PexOperandPath,
) -> String {
    let scope = pex_scope_label(path, object, state, function, function_kind);
    let operand = pex_operand_id(operand);
    format!("pex:{path}:{scope}:{instruction_index}:{operand}")
}

fn pex_scope_label(
    path: &str,
    object: &str,
    state: &str,
    function: &str,
    function_kind: PexFunctionKind,
) -> String {
    let script_stem = path_stem(path);
    let mut segments = Vec::new();
    if !object.is_empty() && !object.eq_ignore_ascii_case(script_stem) {
        segments.push(object.to_string());
    }
    if !state.is_empty() {
        segments.push(state.to_string());
    }
    segments.push(match function_kind {
        PexFunctionKind::Normal => function.to_string(),
        PexFunctionKind::Getter => format!("{function}.get"),
        PexFunctionKind::Setter => format!("{function}.set"),
    });
    segments.join("/")
}

fn pex_operand_id(operand: PexOperandPath) -> String {
    match operand {
        PexOperandPath::Fixed(index) => format!("f{index}"),
        PexOperandPath::Variadic(index) => format!("v{index}"),
    }
}

fn path_stem(path: &str) -> &str {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name)
}

fn concat_group_id(input: &ExtractFunctionInput<'_>, group_index: usize) -> String {
    let scope = pex_scope_label(
        input.path,
        input.object_name,
        input.state_name,
        input.function_name,
        input.function_kind,
    );
    format!("pex-concat:{}:{}:{group_index}", input.path, scope)
}

fn instruction_operand_mut<'a>(
    file: &'a mut PexFile,
    binding: &PexStringBinding,
) -> Option<&'a mut PexValue> {
    let function = match binding.slot {
        FunctionSlot::State {
            object_index,
            state_index,
            function_index,
        } => {
            &mut file
                .objects
                .get_mut(object_index)?
                .states
                .get_mut(state_index)?
                .functions[function_index]
        }
        FunctionSlot::PropertyGetter {
            object_index,
            property_index,
        } => file
            .objects
            .get_mut(object_index)?
            .properties
            .get_mut(property_index)?
            .read_function
            .as_mut()?,
        FunctionSlot::PropertySetter {
            object_index,
            property_index,
        } => file
            .objects
            .get_mut(object_index)?
            .properties
            .get_mut(property_index)?
            .write_function
            .as_mut()?,
    };
    let instruction = function.instructions.get_mut(binding.instruction_index)?;
    match binding.operand {
        PexOperandPath::Fixed(index) => instruction.arguments.get_mut(index),
        PexOperandPath::Variadic(index) => instruction.variadic_arguments.get_mut(index),
    }
}

fn count_string_references(file: &PexFile, target: PexStringId) -> usize {
    let mut count = 0;
    if let Some(debug_info) = &file.debug_info {
        for function in &debug_info.functions {
            count_id(&mut count, function.object_name, target);
            count_id(&mut count, function.state_name, target);
            count_id(&mut count, function.function_name, target);
        }
    }
    for user_flag in &file.user_flags {
        count_id(&mut count, user_flag.name, target);
    }
    for object in &file.objects {
        count_id(&mut count, object.name, target);
        count_id(&mut count, object.parent_class_name, target);
        count_id(&mut count, object.documentation_string, target);
        count_id(&mut count, object.auto_state_name, target);
        for variable in &object.variables {
            count_id(&mut count, variable.name, target);
            count_id(&mut count, variable.type_name, target);
            count_value(&mut count, variable.default_value, target);
        }
        for property in &object.properties {
            count_id(&mut count, property.name, target);
            count_id(&mut count, property.type_name, target);
            count_id(&mut count, property.documentation_string, target);
            if let Some(auto_var) = property.auto_var {
                count_id(&mut count, auto_var, target);
            }
            if let Some(function) = &property.read_function {
                count_function(&mut count, function, target);
            }
            if let Some(function) = &property.write_function {
                count_function(&mut count, function, target);
            }
        }
        for state in &object.states {
            count_id(&mut count, state.name, target);
            for function in &state.functions {
                count_function(&mut count, function, target);
            }
        }
    }
    count
}

fn count_function(count: &mut usize, function: &PexFunction, target: PexStringId) {
    count_id(count, function.name, target);
    count_id(count, function.return_type_name, target);
    count_id(count, function.documentation_string, target);
    for parameter in &function.parameters {
        count_id(count, parameter.name, target);
        count_id(count, parameter.type_name, target);
    }
    for local in &function.locals {
        count_id(count, local.name, target);
        count_id(count, local.type_name, target);
    }
    for instruction in &function.instructions {
        for value in &instruction.arguments {
            count_value(count, *value, target);
        }
        for value in &instruction.variadic_arguments {
            count_value(count, *value, target);
        }
    }
}

fn count_value(count: &mut usize, value: PexValue, target: PexStringId) {
    if let PexValue::String(id) | PexValue::Identifier(id) = value {
        count_id(count, id, target);
    }
}

fn count_id(count: &mut usize, id: PexStringId, target: PexStringId) {
    if id == target {
        *count += 1;
    }
}

fn value_label(file: &PexFile, value: PexValue) -> Option<String> {
    match value {
        PexValue::Identifier(id) | PexValue::String(id) => Some(string(file, id).to_string()),
        PexValue::None => Some("None".to_string()),
        PexValue::Integer(value) => Some(value.to_string()),
        PexValue::Float(value) => Some(value.to_string()),
        PexValue::Bool(value) => Some(value.to_string()),
    }
}

fn string(file: &PexFile, id: PexStringId) -> &str {
    file.string(id).unwrap_or("")
}
