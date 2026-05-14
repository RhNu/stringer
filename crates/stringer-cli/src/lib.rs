#![forbid(unsafe_code)]

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use stringer_workspace::{
    AnnotateTranslationsOptions, BuildKnowledgeIndexOptions, ExportTranslationsOptions,
    ImportTranslationsOptions, KnowledgeLayerOverrides, LoadWorkspaceSettingsOptions,
    LookupKnowledgeOptions, PipelineEntryKind, ValidateTranslationsOptions, WorkspaceError,
    WorkspaceSettingsOverrides, WriteTarget, annotate_translations, build_knowledge_index,
    export_translations, import_translations, load_workspace_settings, lookup_knowledge,
    parse_game_release_name, parse_language_name, validate_translations,
};

const ROOT_LONG_ABOUT: &str = r#"Stringer 是 Bethesda 模组本地化工作流的命令行入口。

它把模组根目录里的可翻译内容导出为翻译包，让人工或 Agent 编辑 JSONL；然后把译文写回覆盖目录。知识库子命令可以把术语、翻译记忆、规则诊断写进翻译包，也可以按单条文本查询上下文。

推荐 Agent 使用方式：
  1. 先运行 `stringer --help` 看全局流程。
  2. 再运行具体子命令的 `--help`，例如 `stringer export --help`。
  3. 优先显式传入 game/language/locale 参数，减少对本机默认配置的依赖。"#;

const ROOT_AFTER_LONG_HELP: &str = r#"典型流程:
  stringer export --root <MOD_ROOT> --out <TRANSLATIONS> --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans
  stringer knowledge annotate --root <MOD_ROOT> --translations <TRANSLATIONS>
  # 编辑 <TRANSLATIONS>/entries/**/*.jsonl 里的 translation 字段
  stringer knowledge validate --root <MOD_ROOT> --translations <TRANSLATIONS>
  stringer import --root <MOD_ROOT> --translations <TRANSLATIONS> --override-root <OVERRIDE_ROOT>

知识库默认位置:
  <MOD_ROOT>/knowledge/terms/*.toml
  <MOD_ROOT>/knowledge/memory/*.jsonl
  <MOD_ROOT>/knowledge/rules/*.toml

更多项目说明见 README.md。"#;

const SETTINGS_LONG_HELP: &str = r#"这些设置决定如何解释 Bethesda 本地化资产和翻译包 locale。

如果不传，命令会尝试从默认配置文件读取。为了让 Agent 调用可复现，建议在 export、lookup、index rebuild 中显式传入:
  --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans"#;

const KNOWLEDGE_ROOTS_LONG_HELP: &str = r#"知识层加载顺序为 built-in < global < library < project < override。

project 层固定为 <MOD_ROOT>/knowledge。global 层来自默认配置、项目 stringer.toml 或 --global-knowledge-root。library 层位于 global/libraries/<GameRelease>/<target_locale>。override 层只在传入 --override-knowledge-root 时使用，适合临时覆盖术语或记忆。"#;

const EXPORT_LONG_ABOUT: &str = r#"扫描模组根目录，导出 Agent 可编辑的翻译包。

导出的翻译包是一个目录，包含 manifest.json 和 entries/**/*.jsonl。每个 JSONL 行通常包含 id、source、translation、context、hints、diagnostics。刚导出时 translation 通常为空，后续由人工或 Agent 填写。

export 当前读取默认配置和命令行覆盖参数，不读取 <MOD_ROOT>/stringer.toml。"#;

const EXPORT_AFTER_LONG_HELP: &str = r#"示例:
  stringer export --root ./MyMod --out ./translations --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

输出结构:
  <TRANSLATIONS>/manifest.json
  <TRANSLATIONS>/entries/plugin/<asset>/<record_type>.jsonl
  <TRANSLATIONS>/entries/pex/<asset>.jsonl
  <TRANSLATIONS>/entries/scaleform/<asset>.jsonl

下一步通常运行:
  stringer knowledge annotate --root ./MyMod --translations ./translations"#;

const IMPORT_LONG_ABOUT: &str = r#"读取翻译包里的 id 和 translation，把译文应用回源模组资产，并把发生变化的文件写到覆盖目录。

import 会忽略 hints、diagnostics 和其他扩展字段。没有 translation 的记录会跳过。覆盖目录不能位于源模组目录内部，避免误覆盖输入。"#;

const IMPORT_AFTER_LONG_HELP: &str = r#"示例:
  stringer import --root ./MyMod --translations ./translations --override-root ./StringerOverride

建议:
  先运行 `stringer knowledge validate`，再 import。
  将 override-root 指向新的空目录，再交给模组管理器加载。"#;

const KNOWLEDGE_LONG_ABOUT: &str = r#"知识库子命令用于给翻译包和单条查询提供上下文。

知识来源包括术语 TOML、翻译记忆 JSONL、替换规则 TOML，以及可重建的 .stringer/indexes/knowledge.sqlite。普通 annotate、validate 和 lookup 会优先使用新鲜索引；索引缺失或过期时回退到文件知识库，并报告 knowledge.index_stale。"#;

const ANNOTATE_LONG_ABOUT: &str = r#"给翻译包写入 hints，并可选使用高置信翻译记忆自动填充 translation。

annotate 会读取翻译包、加载知识库、清理 Stringer 内建处理器写入的旧 hints/diagnostics，然后重新写入术语提示、记忆候选和相关诊断。默认不会覆盖 Agent 已写好的 translation；只有传入 --auto-fill-memory 时，才允许高置信翻译记忆填充空译文。"#;

const ANNOTATE_AFTER_LONG_HELP: &str = r#"示例:
  stringer knowledge annotate --root ./MyMod --translations ./translations
  stringer knowledge annotate --root ./MyMod --translations ./translations --auto-fill-memory

Agent 编辑建议:
  读取 entries/**/*.jsonl 时优先看 source、context、hints 和 diagnostics。
  写译文时只改 translation 字段；保留 id 和 source。"#;

const VALIDATE_LONG_ABOUT: &str = r#"重新计算翻译包 diagnostics，用于导入前检查风险。

validate 不信任包里已有的旧 diagnostics，会用当前知识库重新计算。它只报告问题并写回 diagnostics，不会阻止后续 import。"#;

const VALIDATE_AFTER_LONG_HELP: &str = r#"示例:
  stringer knowledge validate --root ./MyMod --translations ./translations

常见 diagnostic:
  term.preferred_missing  推荐术语未使用
  term.forbidden_used     使用了禁用译法
  placeholder.mismatch    占位符不一致
  scaleform.newline       Scaleform 换行风险
  translation.empty       译文为空
  memory.conflict         与翻译记忆冲突"#;

const LOOKUP_LONG_ABOUT: &str = r#"按单条文本查询知识库提示，适合 Agent 在翻译某个 source 前即时查上下文。

lookup 会构造一个临时条目，使用 text、kind、record-type、subrecord、game/language/locale 等信息匹配术语和记忆。加 --json 可输出结构化 hints 和 diagnostics。"#;

const LOOKUP_AFTER_LONG_HELP: &str = r#"示例:
  stringer knowledge lookup --root ./MyMod --text "Iron Sword" --kind plugin --record-type WEAP --subrecord FULL --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans --json

kind 可用值:
  plugin
  strings
  scaleform
  pex

提示:
  plugin 条目通常传 record-type 和 subrecord。
  scaleform 条目通常只需要 kind=scaleform 和 text。
  pex 第一阶段不会进入知识增强主流程，lookup 仍允许传 kind=pex 以保持格式统一。"#;

const INDEX_LONG_ABOUT: &str = r#"知识索引维护命令。

索引是派生缓存，不是权威数据源。可以随时删除并通过 rebuild 重建。"#;

const INDEX_REBUILD_LONG_ABOUT: &str = r#"重建 <MOD_ROOT>/.stringer/indexes/knowledge.sqlite。

rebuild 会读取当前知识层文件，写入 terms、memory、rules 和 diagnostics 的派生索引。后续 annotate、validate、lookup 会在索引新鲜时优先使用它。"#;

const INDEX_REBUILD_AFTER_LONG_HELP: &str = r#"示例:
  stringer knowledge index rebuild --root ./MyMod --game-release SkyrimSe --asset-language English --source-locale en --target-locale zh-Hans

如果知识文件频繁变化，Agent 可以在批量 annotate 或 lookup 前先 rebuild。"#;

#[derive(Debug, Parser)]
#[command(
    name = "stringer",
    version,
    about = "Bethesda 模组本地化导入、导出和知识库工具",
    long_about = ROOT_LONG_ABOUT,
    after_long_help = ROOT_AFTER_LONG_HELP,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(
        about = "从模组根目录导出翻译包",
        long_about = EXPORT_LONG_ABOUT,
        after_long_help = EXPORT_AFTER_LONG_HELP
    )]
    Export(ExportCommand),
    #[command(
        about = "把翻译包写回覆盖目录",
        long_about = IMPORT_LONG_ABOUT,
        after_long_help = IMPORT_AFTER_LONG_HELP
    )]
    Import(ImportCommand),
    #[command(
        about = "术语、翻译记忆、规则和诊断工具",
        long_about = KNOWLEDGE_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
}

#[derive(Debug, Parser)]
pub struct ExportCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。Stringer 会递归读取其中的 plugin、STRINGS、PEX 和 Scaleform 翻译表资产。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "翻译包输出目录",
        long_help = "翻译包输出目录。命令会创建 manifest.json 和 entries/**/*.jsonl；目录已存在时会按当前导出结果重写相关文件。"
    )]
    pub out: Utf8PathBuf,
    #[arg(
        long,
        value_name = "GAME",
        help = "游戏版本，例如 SkyrimSe",
        long_help = SETTINGS_LONG_HELP
    )]
    pub game_release: Option<String>,
    #[arg(
        long,
        value_name = "LANGUAGE",
        help = "Bethesda 资产语言，例如 English",
        long_help = SETTINGS_LONG_HELP
    )]
    pub asset_language: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "源语言 locale，例如 en",
        long_help = SETTINGS_LONG_HELP
    )]
    pub source_locale: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "目标语言 locale，例如 zh-Hans",
        long_help = SETTINGS_LONG_HELP
    )]
    pub target_locale: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ImportCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。导入时会重新读取这些原始资产，再把 translation 应用到对应条目。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "翻译包目录",
        long_help = "翻译包目录，必须包含 manifest.json 和 entries/**/*.jsonl。import 只读取每行的 id 和 translation。"
    )]
    pub translations: Utf8PathBuf,
    #[arg(
        long,
        value_name = "OVERRIDE_ROOT",
        help = "覆盖目录输出位置",
        long_help = "覆盖目录输出位置。Stringer 只写入发生变化的资产，并要求该目录不能位于源模组根目录内部。"
    )]
    pub override_root: Utf8PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeCommand {
    #[command(
        about = "给翻译包写入知识提示",
        long_about = ANNOTATE_LONG_ABOUT,
        after_long_help = ANNOTATE_AFTER_LONG_HELP
    )]
    Annotate(KnowledgeAnnotateCommand),
    #[command(
        about = "校验翻译包并写入 diagnostics",
        long_about = VALIDATE_LONG_ABOUT,
        after_long_help = VALIDATE_AFTER_LONG_HELP
    )]
    Validate(KnowledgeValidateCommand),
    #[command(
        about = "查询单条文本的术语和记忆提示",
        long_about = LOOKUP_LONG_ABOUT,
        after_long_help = LOOKUP_AFTER_LONG_HELP
    )]
    Lookup(KnowledgeLookupCommand),
    #[command(
        about = "维护知识库派生索引",
        long_about = INDEX_LONG_ABOUT,
        arg_required_else_help = true
    )]
    Index {
        #[command(subcommand)]
        command: KnowledgeIndexCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum KnowledgeIndexCommand {
    #[command(
        about = "重建知识库 SQLite 索引",
        long_about = INDEX_REBUILD_LONG_ABOUT,
        after_long_help = INDEX_REBUILD_AFTER_LONG_HELP
    )]
    Rebuild(KnowledgeIndexRebuildCommand),
}

#[derive(Debug, Parser)]
pub struct KnowledgeAnnotateCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。用于定位项目知识库 <MOD_ROOT>/knowledge，并在需要时读取 <MOD_ROOT>/stringer.toml 的 knowledge.global_root。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "翻译包目录",
        long_help = "翻译包目录。annotate 会原地更新 entries/**/*.jsonl，写入 hints、diagnostics，并可能在允许时填充 translation。"
    )]
    pub translations: Utf8PathBuf,
    #[arg(
        long,
        help = "允许高置信翻译记忆自动填充空译文",
        long_help = "允许高置信翻译记忆自动填充 translation。默认关闭；开启后只会填充满足阈值的空译文，不会覆盖已有 Agent 译文。"
    )]
    pub auto_fill_memory: bool,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "覆盖全局知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "追加最高优先级知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeValidateCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。用于定位项目知识库 <MOD_ROOT>/knowledge，并在需要时读取 <MOD_ROOT>/stringer.toml 的 knowledge.global_root。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TRANSLATIONS",
        help = "翻译包目录",
        long_help = "翻译包目录。validate 会原地更新 entries/**/*.jsonl，重算每条记录的 diagnostics。"
    )]
    pub translations: Utf8PathBuf,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "覆盖全局知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "追加最高优先级知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

#[derive(Debug, Parser)]
pub struct KnowledgeLookupCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。lookup 会用它定位项目知识库、知识索引和可选 stringer.toml。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "TEXT",
        help = "要查询的源文本",
        long_help = "要查询的源文本。lookup 会把它作为临时 PipelineEntry 的 source_text，匹配术语和翻译记忆。"
    )]
    pub text: String,
    #[arg(
        long,
        default_value = "plugin",
        value_name = "KIND",
        help = "条目类型：plugin、strings、scaleform、pex",
        long_help = "条目类型。可用值：plugin、strings、scaleform、pex。知识增强主要覆盖 plugin、strings 和 scaleform；pex 目前保留统一格式。"
    )]
    pub kind: String,
    #[arg(
        long,
        value_name = "RECORD_TYPE",
        help = "Plugin 记录类型，例如 WEAP",
        long_help = "Plugin 记录类型，例如 WEAP、ARMO、NPC_。术语 scope 可用它提高匹配精度。"
    )]
    pub record_type: Option<String>,
    #[arg(
        long,
        value_name = "SUBRECORD",
        help = "Plugin 子记录，例如 FULL",
        long_help = "Plugin 子记录，例如 FULL、DESC。术语和翻译记忆可以用它限定上下文。"
    )]
    pub subrecord: Option<String>,
    #[arg(
        long,
        value_name = "GAME",
        help = "游戏版本，例如 SkyrimSe",
        long_help = SETTINGS_LONG_HELP
    )]
    pub game_release: Option<String>,
    #[arg(
        long,
        value_name = "LANGUAGE",
        help = "Bethesda 资产语言，例如 English",
        long_help = SETTINGS_LONG_HELP
    )]
    pub asset_language: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "源语言 locale，例如 en",
        long_help = SETTINGS_LONG_HELP
    )]
    pub source_locale: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "目标语言 locale，例如 zh-Hans",
        long_help = SETTINGS_LONG_HELP
    )]
    pub target_locale: Option<String>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "覆盖全局知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "追加最高优先级知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        help = "输出结构化 JSON",
        long_help = "输出结构化 JSON，包含 index_used、hints 和 diagnostics。Agent 查询时建议开启。"
    )]
    pub json: bool,
}

#[derive(Debug, Parser)]
pub struct KnowledgeIndexRebuildCommand {
    #[arg(
        long,
        value_name = "MOD_ROOT",
        help = "源模组根目录",
        long_help = "源模组根目录。索引会写到 <MOD_ROOT>/.stringer/indexes/knowledge.sqlite。"
    )]
    pub root: Utf8PathBuf,
    #[arg(
        long,
        value_name = "GAME",
        help = "游戏版本，例如 SkyrimSe",
        long_help = SETTINGS_LONG_HELP
    )]
    pub game_release: Option<String>,
    #[arg(
        long,
        value_name = "LANGUAGE",
        help = "Bethesda 资产语言，例如 English",
        long_help = SETTINGS_LONG_HELP
    )]
    pub asset_language: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "源语言 locale，例如 en",
        long_help = SETTINGS_LONG_HELP
    )]
    pub source_locale: Option<String>,
    #[arg(
        long,
        value_name = "LOCALE",
        help = "目标语言 locale，例如 zh-Hans",
        long_help = SETTINGS_LONG_HELP
    )]
    pub target_locale: Option<String>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "覆盖全局知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub global_knowledge_root: Option<Utf8PathBuf>,
    #[arg(
        long,
        value_name = "KNOWLEDGE_ROOT",
        help = "追加最高优先级知识库根目录",
        long_help = KNOWLEDGE_ROOTS_LONG_HELP
    )]
    pub override_knowledge_root: Option<Utf8PathBuf>,
}

pub async fn run(cli: Cli) -> Result<(), WorkspaceError> {
    match cli.command {
        Command::Export(command) => {
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path: None,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = export_translations(ExportTranslationsOptions {
                root: command.root,
                out: command.out,
                settings,
            })
            .await?;
            println!("exported {} entries", summary.entries);
            Ok(())
        }
        Command::Import(command) => {
            let summary = import_translations(ImportTranslationsOptions {
                root: command.root,
                translations: command.translations,
                target: WriteTarget::OverrideDirectory {
                    root: command.override_root,
                },
            })
            .await?;
            println!(
                "applied {} entries and wrote {} files",
                summary.applied_entries, summary.written_files
            );
            Ok(())
        }
        Command::Knowledge { command } => run_knowledge(command).await,
    }
}

async fn run_knowledge(command: KnowledgeCommand) -> Result<(), WorkspaceError> {
    match command {
        KnowledgeCommand::Annotate(command) => {
            let summary = annotate_translations(AnnotateTranslationsOptions {
                root: command.root,
                translations: command.translations,
                allow_memory_auto_fill: command.auto_fill_memory,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "annotated {} entries, added {} hints, wrote {} diagnostics, auto-filled {} entries",
                summary.entries, summary.annotations, summary.diagnostics, summary.auto_filled
            );
            Ok(())
        }
        KnowledgeCommand::Validate(command) => {
            let summary = validate_translations(ValidateTranslationsOptions {
                root: command.root,
                translations: command.translations,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "validated {} entries and wrote {} diagnostics",
                summary.entries, summary.diagnostics
            );
            Ok(())
        }
        KnowledgeCommand::Lookup(command) => {
            let config_path = project_config_path(&command.root);
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let lookup = lookup_knowledge(LookupKnowledgeOptions {
                root: command.root,
                settings,
                text: command.text,
                kind: parse_pipeline_kind(command.kind)?,
                context: lookup_context(command.record_type, command.subrecord),
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            if command.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "index_used": lookup.index_used,
                        "hints": lookup.annotations,
                        "diagnostics": lookup.diagnostics,
                    }))
                    .map_err(|source| WorkspaceError::Json {
                        path: Utf8PathBuf::from("<stdout>"),
                        source,
                    })?
                );
            } else {
                println!(
                    "found {} hints and {} diagnostics",
                    lookup.annotations.len(),
                    lookup.diagnostics.len()
                );
            }
            Ok(())
        }
        KnowledgeCommand::Index { command } => run_knowledge_index(command).await,
    }
}

async fn run_knowledge_index(command: KnowledgeIndexCommand) -> Result<(), WorkspaceError> {
    match command {
        KnowledgeIndexCommand::Rebuild(command) => {
            let config_path = project_config_path(&command.root);
            let settings = load_workspace_settings(LoadWorkspaceSettingsOptions {
                config_path,
                overrides: overrides(
                    command.game_release,
                    command.asset_language,
                    command.source_locale,
                    command.target_locale,
                )?,
            })?;
            let summary = build_knowledge_index(BuildKnowledgeIndexOptions {
                root: command.root,
                settings,
                knowledge: knowledge_overrides(
                    command.global_knowledge_root,
                    command.override_knowledge_root,
                ),
            })?;
            println!(
                "indexed {} files, {} terms, {} memory entries, {} rules, {} diagnostics",
                summary.files, summary.terms, summary.memory, summary.rules, summary.diagnostics
            );
            Ok(())
        }
    }
}

fn parse_pipeline_kind(value: String) -> Result<PipelineEntryKind, WorkspaceError> {
    PipelineEntryKind::from_package_kind(&value).ok_or(WorkspaceError::InvalidSetting {
        name: "kind",
        value,
    })
}

fn lookup_context(record_type: Option<String>, subrecord: Option<String>) -> Vec<(String, String)> {
    let mut context = Vec::new();
    if let Some(record_type) = record_type {
        context.push(("record_type".to_string(), record_type));
    }
    if let Some(subrecord) = subrecord {
        context.push(("subrecord".to_string(), subrecord));
    }
    context
}

fn knowledge_overrides(
    global_root: Option<Utf8PathBuf>,
    override_root: Option<Utf8PathBuf>,
) -> KnowledgeLayerOverrides {
    KnowledgeLayerOverrides {
        global_root,
        override_root,
    }
}

fn project_config_path(root: &Utf8PathBuf) -> Option<Utf8PathBuf> {
    let path = root.join("stringer.toml");
    path.exists().then_some(path)
}

fn overrides(
    game_release: Option<String>,
    asset_language: Option<String>,
    source_locale: Option<String>,
    target_locale: Option<String>,
) -> Result<WorkspaceSettingsOverrides, WorkspaceError> {
    Ok(WorkspaceSettingsOverrides {
        game_release: game_release
            .as_deref()
            .map(parse_game_release_name)
            .transpose()?,
        asset_language: asset_language
            .as_deref()
            .map(parse_language_name)
            .transpose()?,
        source_locale,
        target_locale,
    })
}
