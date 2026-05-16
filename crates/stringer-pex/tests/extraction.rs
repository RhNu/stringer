use bytes::Bytes;
use stringer_core::{
    FileAsset, PexOperandPath, PexStringMetadata, StringEntryBundle, StringEntrySource,
};
use stringer_extraction_filter::{ExtractionFilterConfig, ExtractionFilterSet};
use stringer_pex::{
    PexDebugFunctionInfo, PexDebugFunctionType, PexDebugInfo, PexFile, PexFunction, PexHeader,
    PexInstruction, PexLocal, PexObject, PexOpcode, PexState, PexUserFlag, PexValue,
    ReadPexOptions, read_pex_strings, write_pex_strings,
};

fn header() -> PexHeader {
    PexHeader::new_skyrim(0, "Example.psc", "tester", "builder")
}

fn extractable_file() -> PexFile {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let local = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let hello = file.intern("Hello world").unwrap();
    let debug = file.intern("Debug").unwrap();
    let notification = file.intern("Notification").unwrap();
    let shown = file.intern("Quest updated").unwrap();
    let prop = file.intern("Title").unwrap();

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
                    PexInstruction::new_variadic(
                        PexOpcode::CallStatic,
                        vec![
                            PexValue::String(debug),
                            PexValue::String(notification),
                            PexValue::None,
                        ],
                        vec![PexValue::String(shown)],
                    )
                    .unwrap(),
                    PexInstruction::new(
                        PexOpcode::PropGet,
                        vec![
                            PexValue::String(prop),
                            PexValue::Identifier(object),
                            PexValue::Identifier(local),
                        ],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    file
}

#[test]
fn extracts_literals_but_skips_call_and_property_symbol_positions() {
    let bytes = extractable_file().write_to_vec().unwrap();
    let asset = FileAsset::new("Data/Scripts/Example.pex", Bytes::from(bytes));

    let bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();

    let texts = bundle
        .string_entries()
        .iter()
        .map(|entry| entry.text())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Hello world", "Quest updated"]);
    let StringEntrySource::Pex(PexStringMetadata {
        object,
        state,
        function,
        instruction_index,
        opcode,
        operand,
        call_context,
        ..
    }) = bundle.string_entries()[1].source()
    else {
        panic!("expected pex metadata");
    };
    assert_eq!(object, "Example");
    assert_eq!(state, "");
    assert_eq!(function, "Run");
    assert_eq!(*instruction_index, 1);
    assert_eq!(opcode, "CALLSTATIC");
    assert_eq!(*operand, PexOperandPath::Variadic(0));
    assert_eq!(
        call_context.as_ref().unwrap().target.as_deref(),
        Some("Debug")
    );
    assert_eq!(
        call_context.as_ref().unwrap().member.as_deref(),
        Some("Notification")
    );
}

#[test]
fn groups_traceable_string_concatenation_literals() {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let name = file.intern("name").unwrap();
    let string_type = file.intern("String").unwrap();
    let first = file.intern("Hello ").unwrap();
    let second = file.intern("wide world").unwrap();
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
                    name: tmp,
                    type_name: string_type,
                }],
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::StrCat,
                        vec![
                            PexValue::Identifier(tmp),
                            PexValue::String(first),
                            PexValue::Identifier(name),
                        ],
                    )
                    .unwrap(),
                    PexInstruction::new(
                        PexOpcode::StrCat,
                        vec![
                            PexValue::Identifier(tmp),
                            PexValue::Identifier(tmp),
                            PexValue::String(second),
                        ],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );

    let bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();

    assert_eq!(bundle.string_entries().len(), 2);
    let first_concat = pex_metadata(&bundle.string_entries()[0])
        .concat
        .as_ref()
        .unwrap();
    let second_concat = pex_metadata(&bundle.string_entries()[1])
        .concat
        .as_ref()
        .unwrap();
    assert_eq!(first_concat.group_id, second_concat.group_id);
    assert_eq!(first_concat.part_index, 0);
    assert_eq!(second_concat.part_index, 1);
    assert!(!first_concat.ambiguous);
}

#[test]
fn filters_empty_identifier_like_and_tag_list_sources() {
    let file = filter_fixture([
        "",
        "SomeCamelCase",
        "someCamelCase",
        "some_id",
        "SOME_ID",
        "Namespace.Member",
        "queststage01",
        "tag,tag,tag",
        "foo_bar,baz-1",
        "Open Door",
        "Hello world",
    ]);
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );

    let bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();

    let texts = bundle
        .string_entries()
        .iter()
        .map(|entry| entry.text())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Open Door", "Hello world"]);
}

#[test]
fn custom_filter_config_can_disable_builtin_identifier_rule() {
    let file = filter_fixture(["SomeIdentifier", "Open Door"]);
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "pex.identifier_like_source"
enabled = false
"#,
    )
    .unwrap();
    let filters = ExtractionFilterSet::from_config(config).unwrap();

    let bundle = read_pex_strings(
        asset,
        ReadPexOptions::default().with_extraction_filters(filters),
    )
    .unwrap();

    let texts = bundle
        .string_entries()
        .iter()
        .map(|entry| entry.text())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["SomeIdentifier", "Open Door"]);
}

#[test]
fn custom_filter_config_can_filter_pex_call_context() {
    let bytes = extractable_file().write_to_vec().unwrap();
    let asset = FileAsset::new("Data/Scripts/Example.pex", Bytes::from(bytes));
    let config: ExtractionFilterConfig = toml::from_str(
        r#"
[[rules]]
id = "user.skip_debug_notifications"
when = { all = [
  { field = "kind", op = "eq", value = "pex" },
  { field = "call_member", op = "eq", value = "Notification" },
] }
"#,
    )
    .unwrap();
    let filters = ExtractionFilterSet::from_config(config).unwrap();

    let bundle = read_pex_strings(
        asset,
        ReadPexOptions::default().with_extraction_filters(filters),
    )
    .unwrap();

    let texts = bundle
        .string_entries()
        .iter()
        .map(|entry| entry.text())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["Hello world"]);
}

#[test]
fn keeps_filtered_concat_sources_as_context_operands() {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let name = file.intern("name").unwrap();
    let string_type = file.intern("String").unwrap();
    let filtered = file.intern("InternalName").unwrap();
    let translatable = file.intern(" opened").unwrap();
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
                    name: tmp,
                    type_name: string_type,
                }],
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::StrCat,
                        vec![
                            PexValue::Identifier(tmp),
                            PexValue::String(filtered),
                            PexValue::Identifier(name),
                        ],
                    )
                    .unwrap(),
                    PexInstruction::new(
                        PexOpcode::StrCat,
                        vec![
                            PexValue::Identifier(tmp),
                            PexValue::Identifier(tmp),
                            PexValue::String(translatable),
                        ],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );

    let bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();

    assert_eq!(bundle.string_entries().len(), 1);
    assert_eq!(bundle.string_entries()[0].text(), " opened");
    let concat = pex_metadata(&bundle.string_entries()[0])
        .concat
        .as_ref()
        .unwrap();
    assert_eq!(concat.part_index, 0);
    assert!(concat.parts.iter().any(|part| {
        matches!(
            part,
            stringer_core::PexConcatPart::Operand { label } if label == "InternalName"
        )
    }));
}

#[test]
fn editing_shared_literal_interns_replacement_without_renaming_metadata_strings() {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let shared = file.intern("Same text").unwrap();
    file.objects.push(PexObject {
        name: object,
        parent_class_name: empty,
        documentation_string: shared,
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
                    name: tmp,
                    type_name: string_type,
                }],
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::Assign,
                        vec![PexValue::Identifier(tmp), PexValue::String(shared)],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );
    let mut bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();
    bundle.string_entries_mut()[0].set_text("changed");

    let written = write_pex_strings(bundle).unwrap();
    let reparsed = PexFile::read_from_slice(written.bytes()).unwrap();
    let object = &reparsed.objects[0];
    let instruction = &object.states[0].functions[0].instructions[0];

    assert_eq!(
        reparsed.string(object.documentation_string).unwrap(),
        "Same text"
    );
    assert_eq!(string_operand(&reparsed, instruction, 1), "changed");
}

#[test]
fn editing_literal_shared_with_debug_info_or_user_flag_does_not_rename_metadata() {
    let mut file = shared_metadata_file();
    let shared = file
        .string_table()
        .iter()
        .position(|text| text == "Same text")
        .map(|index| stringer_pex::PexStringId::new(index as u16))
        .unwrap();
    file.debug_info = Some(PexDebugInfo {
        modification_time: 99,
        functions: vec![PexDebugFunctionInfo {
            object_name: shared,
            state_name: shared,
            function_name: shared,
            function_type: PexDebugFunctionType::Normal,
            instruction_line_map: Vec::new(),
        }],
    });
    file.user_flags.push(PexUserFlag {
        name: shared,
        bit_index: 3,
    });
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );
    let mut bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();
    bundle.string_entries_mut()[0].set_text("changed");

    let written = write_pex_strings(bundle).unwrap();
    let reparsed = PexFile::read_from_slice(written.bytes()).unwrap();
    let debug = reparsed.debug_info.as_ref().unwrap();
    let instruction = &reparsed.objects[0].states[0].functions[0].instructions[0];

    assert_eq!(
        reparsed.string(debug.functions[0].function_name),
        Some("Same text")
    );
    assert_eq!(
        reparsed.string(reparsed.user_flags[0].name),
        Some("Same text")
    );
    assert_eq!(string_operand(&reparsed, instruction, 1), "changed");
}

#[test]
fn writes_reordered_pex_entries_to_their_original_instruction_operands() {
    let mut file = shared_metadata_file();
    let second = file.intern("Second text").unwrap();
    let local_name = file.objects[0].states[0].functions[0].locals[0].name;
    file.objects[0].states[0].functions[0].instructions.push(
        PexInstruction::new(
            PexOpcode::Assign,
            vec![PexValue::Identifier(local_name), PexValue::String(second)],
        )
        .unwrap(),
    );
    let asset = FileAsset::new(
        "Data/Scripts/Example.pex",
        Bytes::from(file.write_to_vec().unwrap()),
    );
    let mut bundle = read_pex_strings(asset, ReadPexOptions::default()).unwrap();

    bundle
        .string_entries_mut()
        .sort_by(|left, right| right.id().cmp(left.id()));
    let second_entry = bundle
        .string_entries_mut()
        .iter_mut()
        .find(|entry| entry.text() == "Second text")
        .expect("second entry should exist");
    second_entry.set_text("changed-second");

    let written = write_pex_strings(bundle).unwrap();
    let reparsed = PexFile::read_from_slice(written.bytes()).unwrap();
    let instructions = &reparsed.objects[0].states[0].functions[0].instructions;

    assert_eq!(string_operand(&reparsed, &instructions[0], 1), "Same text");
    assert_eq!(
        string_operand(&reparsed, &instructions[1], 1),
        "changed-second"
    );
}

fn shared_metadata_file() -> PexFile {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let shared = file.intern("Same text").unwrap();
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
                    name: tmp,
                    type_name: string_type,
                }],
                instructions: vec![
                    PexInstruction::new(
                        PexOpcode::Assign,
                        vec![PexValue::Identifier(tmp), PexValue::String(shared)],
                    )
                    .unwrap(),
                ],
            }],
        }],
    });
    file
}

fn filter_fixture<const N: usize>(texts: [&str; N]) -> PexFile {
    let mut file = PexFile::new(header());
    let empty = file.intern("").unwrap();
    let object = file.intern("Example").unwrap();
    let function = file.intern("Run").unwrap();
    let none = file.intern("None").unwrap();
    let tmp = file.intern("tmp").unwrap();
    let string_type = file.intern("String").unwrap();
    let instructions = texts
        .into_iter()
        .map(|text| {
            let id = file.intern(text).unwrap();
            PexInstruction::new(
                PexOpcode::Assign,
                vec![PexValue::Identifier(tmp), PexValue::String(id)],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
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
                    name: tmp,
                    type_name: string_type,
                }],
                instructions,
            }],
        }],
    });
    file
}

fn pex_metadata(entry: &stringer_core::StringEntry) -> &PexStringMetadata {
    let StringEntrySource::Pex(metadata) = entry.source() else {
        panic!("expected pex metadata");
    };
    metadata
}

fn string_operand<'a>(
    file: &'a PexFile,
    instruction: &stringer_pex::PexInstruction,
    index: usize,
) -> &'a str {
    let PexValue::String(id) = instruction.arguments[index] else {
        panic!("expected string operand");
    };
    file.string(id).unwrap()
}
