use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::Utf8PathBuf;
use stringer_core::FileRole;
use stringer_reader::{FileSourceKind, FileSourceState, ReadModOptions, read_mod_root};

#[test]
fn maps_loose_files_from_bare_mod_root_into_data_logical_paths() {
    let root = temp_root("bare-root");
    write(root.join("Scripts/QuestScript.pex"), b"pex");
    write(root.join("Strings/MyMod_English.STRINGS"), b"strings");

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(
        result
            .files
            .get("Data/Scripts/QuestScript.pex")
            .unwrap()
            .bytes()
            .as_ref(),
        b"pex"
    );
    assert_eq!(
        result
            .files
            .get("data/strings/mymod_english.strings")
            .unwrap()
            .role(),
        FileRole::Strings
    );
}

#[test]
fn maps_loose_files_inside_data_child_without_double_prefixing_data() {
    let root = temp_root("data-child");
    write(
        root.join("Data/Interface/Translations/MyMod_English.txt"),
        b"$A\tB",
    );

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert!(
        result
            .files
            .get("Data/Interface/Translations/MyMod_English.txt")
            .is_some()
    );
    assert!(
        result
            .files
            .get("Data/Data/Interface/Translations/MyMod_English.txt")
            .is_none()
    );
}

#[test]
fn ignores_unknown_large_asset_paths() {
    let root = temp_root("filters-assets");
    write(root.join("Textures/Armor/Iron.dds"), b"texture");
    write(root.join("Meshes/Armor/Iron.nif"), b"mesh");
    write(root.join("Scripts/QuestScript.pex"), b"pex");

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(result.files.files().count(), 1);
    assert_eq!(result.sources.len(), 1);
}

#[test]
fn loose_file_overrides_same_logical_path_case_insensitively() {
    let root = temp_root("loose-overrides-loose");
    write(root.join("Data/Scripts/QuestScript.pex"), b"from-data");
    write(root.join("scripts/questscript.PEX"), b"from-root");

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(
        result
            .files
            .get("Data/Scripts/QuestScript.pex")
            .unwrap()
            .bytes()
            .as_ref(),
        b"from-root"
    );
    assert!(
        result
            .sources
            .iter()
            .any(|source| source.state == FileSourceState::Shadowed)
    );
    assert!(
        result
            .sources
            .iter()
            .any(|source| matches!(source.kind, FileSourceKind::Loose { .. })
                && source.state == FileSourceState::Included)
    );
}

#[test]
fn reads_known_assets_from_bsa_and_marks_archive_source_shadowed_by_loose_file() {
    let root = temp_root("bsa-override");
    write_tes4_bsa(
        &root.join("MyMod.bsa"),
        &[
            ("Scripts", "QuestScript.pex", b"from-archive".as_slice()),
            ("Textures", "Ignored.dds", b"texture".as_slice()),
        ],
    );
    write(root.join("Data/Scripts/QuestScript.pex"), b"from-loose");

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(
        result
            .files
            .get("Data/Scripts/QuestScript.pex")
            .unwrap()
            .bytes()
            .as_ref(),
        b"from-loose"
    );
    assert!(result.files.get("Data/Textures/Ignored.dds").is_none());
    assert!(result.sources.iter().any(|source| matches!(
        &source.kind,
        FileSourceKind::Archive { entry_path, .. }
            if entry_path.as_str().eq_ignore_ascii_case("Scripts/QuestScript.pex")
                && source.state == FileSourceState::Shadowed
    )));
}

#[test]
fn later_sorted_archive_overrides_earlier_archive_for_same_logical_path() {
    let root = temp_root("archive-precedence");
    write_tes4_bsa(
        &root.join("A.bsa"),
        &[("Scripts", "QuestScript.pex", b"from-a".as_slice())],
    );
    write_tes4_bsa(
        &root.join("Z.bsa"),
        &[("Scripts", "QuestScript.pex", b"from-z".as_slice())],
    );

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(
        result
            .files
            .get("Data/Scripts/QuestScript.pex")
            .unwrap()
            .bytes()
            .as_ref(),
        b"from-z"
    );
    assert!(result.sources.iter().any(|source| matches!(
        &source.kind,
        FileSourceKind::Archive { archive_path, .. }
            if archive_path.file_name() == Some("A.bsa")
                && source.state == FileSourceState::Shadowed
    )));
    assert!(result.sources.iter().any(|source| matches!(
        &source.kind,
        FileSourceKind::Archive { archive_path, .. }
            if archive_path.file_name() == Some("Z.bsa")
                && source.state == FileSourceState::Included
    )));
}

#[test]
fn reads_known_assets_from_ba2_string_table_entries() {
    let root = temp_root("ba2");
    write_fo4_ba2(
        &root.join("Data/MyMod.ba2"),
        &[
            (
                "Interface/Translations/MyMod_English.txt",
                b"$A\tB".as_slice(),
            ),
            ("Meshes/Ignored.nif", b"mesh".as_slice()),
        ],
    );

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert!(
        result
            .files
            .get("Data/Interface/Translations/MyMod_English.txt")
            .is_some()
    );
    assert!(result.files.get("Data/Meshes/Ignored.nif").is_none());
    assert_eq!(result.sources.len(), 1);
}

#[test]
fn skips_archive_entries_with_non_normal_virtual_path_components() {
    let root = temp_root("invalid-archive-paths");
    write_fo4_ba2(
        &root.join("Invalid.ba2"),
        &[
            ("Scripts/../Scripts/Traversal.pex", b"bad-parent".as_slice()),
            ("./Scripts/Current.pex", b"bad-current".as_slice()),
            ("C:/Scripts/Absolute.pex", b"bad-absolute".as_slice()),
            ("Scripts/Valid.pex", b"valid".as_slice()),
        ],
    );

    let result = read_mod_root(&root, ReadModOptions::default()).unwrap();

    assert_eq!(result.files.files().count(), 1);
    assert_eq!(
        result
            .files
            .get("Data/Scripts/Valid.pex")
            .unwrap()
            .bytes()
            .as_ref(),
        b"valid"
    );
    assert!(result.files.get("Data/Scripts/Traversal.pex").is_none());
    assert!(result.files.get("Data/Scripts/Current.pex").is_none());
    assert!(result.files.get("Data/Scripts/Absolute.pex").is_none());
    assert_eq!(result.sources.len(), 1);
}

struct TempRoot {
    path: Utf8PathBuf,
}

impl AsRef<camino::Utf8Path> for TempRoot {
    fn as_ref(&self) -> &camino::Utf8Path {
        &self.path
    }
}

impl std::ops::Deref for TempRoot {
    type Target = camino::Utf8Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        _ = fs::remove_dir_all(&self.path);
    }
}

fn temp_root(label: &str) -> TempRoot {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "stringer-reader-{label}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    TempRoot {
        path: Utf8PathBuf::from_path_buf(root).unwrap(),
    }
}

fn write(path: impl AsRef<Path>, bytes: &[u8]) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn write_tes4_bsa(path: &camino::Utf8Path, entries: &[(&str, &str, &[u8])]) {
    use ba2::prelude::*;
    use ba2::tes4::{
        Archive, ArchiveKey, ArchiveOptions, ArchiveTypes, DirectoryKey, File, Version,
    };

    let mut archive = Archive::new();
    for (directory_name, file_name, bytes) in entries {
        let mut directory = archive
            .remove(&ArchiveKey::from(*directory_name))
            .unwrap_or_default();
        directory.insert(
            DirectoryKey::from(*file_name),
            File::from_decompressed(*bytes),
        );
        archive.insert(ArchiveKey::from(*directory_name), directory);
    }

    let mut output = Vec::new();
    let options = ArchiveOptions::builder()
        .types(ArchiveTypes::MISC)
        .version(Version::SSE)
        .build();
    archive.write(&mut output, &options).unwrap();
    write(path, &output);
}

fn write_fo4_ba2(path: &camino::Utf8Path, entries: &[(&str, &[u8])]) {
    use ba2::fo4::{Archive, ArchiveKey, ArchiveOptions, Chunk, File};
    use ba2::prelude::*;

    let archive: Archive = entries
        .iter()
        .map(|(entry_path, bytes)| {
            let file: File = [Chunk::from_decompressed(*bytes)].into_iter().collect();
            (ArchiveKey::from(*entry_path), file)
        })
        .collect();
    let mut output = Vec::new();
    let options = ArchiveOptions::builder().strings(true).build();
    archive.write(&mut output, &options).unwrap();
    write(path, &output);
}
