use bytes::Bytes;
use stringer_core::FileAsset;
use stringer_pex::{
    PexDebugFunctionInfo, PexDebugFunctionType, PexDebugInfo, PexError, PexFile, PexFunction,
    PexHeader, PexInstruction, PexLocal, PexObject, PexOpcode, PexProperty, PexState, PexStringId,
    PexValue, parse_pex_file, write_pex_file,
};

fn header() -> PexHeader {
    PexHeader::new_skyrim(0, "Example.psc", "tester", "builder")
}

fn literal_file() -> PexFile {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let local = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let hello = file.intern("hello").unwrap();

    file.objects.push(PexObject {
        name: object,
        parent_class_name: empty,
        documentation_string: empty,
        user_flags: 0,
        auto_state_name: empty,
        variables: Vec::new(),
        properties: Vec::new(),
        states: vec![PexState {
            name: empty,
            functions: vec![PexFunction {
                name: function,
                return_type_name: none,
                documentation_string: empty,
                user_flags: 0,
                is_global: false,
                is_native: false,
                parameters: Vec::new(),
                locals: vec![PexLocal {
                    name: local,
                    type_name: string_type,
                }],
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::Assign,
                        vec![PexValue::Identifier(local), PexValue::String(hello)],
                    )
                    .unwrap(),
                    PexInstruction::new(PexOpcode::Return, vec![PexValue::None]).unwrap(),
                ],
            }],
        }],
    });
    file
}

#[test]
fn reads_written_skyrim_pex_and_reemits_identically() {
    let file = literal_file();
    let bytes = file.write_to_vec().unwrap();

    let read = PexFile::read_from_slice(&bytes).unwrap();

    assert_eq!(read, file);
    assert_eq!(read.write_to_vec().unwrap(), bytes);
    assert_eq!(read.string(PexStringId::new(6)).unwrap(), "hello");
}

#[test]
fn rejects_malformed_pex_inputs() {
    let error = PexFile::read_from_slice(&[0, 0, 0, 0]).unwrap_err();
    assert!(matches!(error, PexError::InvalidMagic { .. }));

    let error = PexFile::read_from_slice(&[0xFA, 0x57, 0xC0, 0xDE, 0x03]).unwrap_err();
    assert!(matches!(error, PexError::Truncated { .. }));

    let mut bytes = literal_file().write_to_vec().unwrap();
    let return_opcode = bytes
        .iter()
        .position(|byte| *byte == PexOpcode::Return as u8)
        .unwrap();
    bytes[return_opcode] = 200;
    let error = PexFile::read_from_slice(&bytes).unwrap_err();
    assert!(matches!(error, PexError::UnknownOpcode { opcode: 200, .. }));
}

#[test]
fn rejects_oversized_debug_line_maps_and_accessor_tables_before_writing() {
    let mut debug_file = literal_file();
    let object = debug_file.objects[0].name;
    let state = debug_file.objects[0].states[0].name;
    let function = debug_file.objects[0].states[0].functions[0].name;
    debug_file.debug_info = Some(PexDebugInfo {
        modification_time: 1,
        functions: vec![PexDebugFunctionInfo {
            object_name: object,
            state_name: state,
            function_name: function,
            function_type: PexDebugFunctionType::Normal,
            instruction_line_map: vec![1; u16::MAX as usize + 1],
        }],
    });

    let error = debug_file.write_to_vec().unwrap_err();
    assert!(matches!(
        error,
        PexError::CountTooLarge {
            what: "debug line map",
            ..
        }
    ));

    let mut accessor_file = literal_file();
    let empty = accessor_file.intern("").unwrap();
    let property_name = accessor_file.intern("Title").unwrap();
    let string_type = accessor_file.intern("String").unwrap();
    let getter = PexFunction {
        name: property_name,
        return_type_name: string_type,
        documentation_string: empty,
        user_flags: 0,
        is_global: false,
        is_native: false,
        parameters: Vec::new(),
        locals: vec![
            PexLocal {
                name: empty,
                type_name: string_type,
            };
            u16::MAX as usize + 1
        ],
        instructions: Vec::new(),
    };
    accessor_file.objects[0].properties.push(PexProperty {
        name: property_name,
        type_name: string_type,
        documentation_string: empty,
        user_flags: 0,
        is_readable: true,
        is_writable: false,
        is_auto: false,
        auto_var: None,
        read_function: Some(getter),
        write_function: None,
    });

    let error = accessor_file.write_to_vec().unwrap_err();
    assert!(matches!(
        error,
        PexError::CountTooLarge {
            what: "local table",
            ..
        }
    ));
}

#[test]
fn preserves_unmodified_parsed_pex_asset_bytes() {
    let bytes = literal_file().write_to_vec().unwrap();
    let asset = FileAsset::new("Data/Scripts/Example.pex", Bytes::from(bytes.clone()));
    let parsed = parse_pex_file(&asset).unwrap();

    let written = write_pex_file(&parsed).unwrap();

    assert_eq!(written.path().as_str(), "Data/Scripts/Example.pex");
    assert_eq!(written.bytes().as_ref(), bytes.as_slice());
}
