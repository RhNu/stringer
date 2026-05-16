use super::*;
use stringer_core::Language;
use stringer_plugin::GameRelease;

#[test]
fn workspace_scope_build_settings_do_not_resolve_global_defaults() {
    let settings =
        settings_for_build_scope_with(test_settings(), KnowledgeIndexBuildScope::Workspace, || {
            panic!("workspace scoped index rebuild should not read global defaults")
        })
        .unwrap();

    assert_eq!(settings.global_knowledge_root, None);
}

#[test]
fn all_scope_build_settings_resolve_global_defaults() {
    let settings =
        settings_for_build_scope_with(test_settings(), KnowledgeIndexBuildScope::All, || {
            Ok(Some(Utf8PathBuf::from("global-knowledge")))
        })
        .unwrap();

    assert_eq!(
        settings.global_knowledge_root,
        Some(Utf8PathBuf::from("global-knowledge"))
    );
}

fn test_settings() -> WorkspaceSettings {
    WorkspaceSettings {
        game_release: GameRelease::SkyrimSe,
        asset_language: Language::English,
        source_locale: "en".to_string(),
        target_locale: "zh-Hans".to_string(),
        global_knowledge_root: None,
    }
}
