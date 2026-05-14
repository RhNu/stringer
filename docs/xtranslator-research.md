# xTranslator 源码结构调研

调研日期：2026-05-14

## 概览

xTranslator 是面向 Bethesda 游戏 Mod 的桌面翻译工具，源码主体是 Delphi/VCL 项目。README 把它描述为同一套工具在不同游戏工作区下运行：Skyrim、Skyrim Special Edition、Fallout 4、Fallout New Vegas、Fallout 76 和 Starfield 共用核心功能，只是按游戏切换数据目录、编码、记录定义、归档规则和默认路径。

源码快照位于 `main` 分支，提交 `9aa38d6`。项目入口是 `xTranslator.dpr`，工程文件是 `xTranslator.dproj`。代码没有按多项目拆分，而是大量 Pascal unit 直接组成一个 VCL 应用；核心逻辑集中在主窗体、加载器、ESP 结构解析、字符串表处理、词典处理、在线翻译 API 和批处理模块中。

## 顶层结构

| 路径或文件 | 作用 |
| --- | --- |
| `xTranslator.dpr` / `xTranslator.dproj` | Delphi/VCL 应用入口和工程配置 |
| `TESVT_main.pas` / `TESVT_main.dfm` | 主窗体、菜单事件、加载/保存流程、批量操作调度、UI 状态管理 |
| `TESVT_MainLoader.pas` | 单个已加载文件的状态容器和核心加载器，协调 ESP、Strings、MCM、PEX、VMAD、SST 应用 |
| `TESVT_typedef.pas` | 翻译条目、状态位、ESP 引用、词典文件夹和树节点等核心类型 |
| `TESVT_espDefinition.pas` | ESP/ESM 二进制记录、字段、GRUP、压缩记录、保存和快速索引 |
| `TESVT_StringsFunc.pas` | `.STRINGS`、`.DLSTRINGS`、`.ILSTRINGS` 读取、匹配和写回 |
| `TESVT_SSTFunc.pas` | xTranslator 自有 SST 词典格式的保存和加载 |
| `TESVT_TranslateFunc.pas` | 自动匹配、启发式匹配、派生字符串、繁简转换和 RTL 处理 |
| `TESVT_TranslatorApi.pas` | Microsoft、Google、DeepL、OpenAI、Baidu、Youdao 等在线翻译 API 封装 |
| `TESVT_scriptPex.pas` | Papyrus PEX 读取、字符串授权判定、反编译反馈和写回 |
| `TESVT_bsa.pas` | BSA/BA2 归档读取、浏览和提取 |
| `TESVT_XMLFunc.pas` | XML 导入导出 |
| `TESVT_Batcher.pas` | 规则和命令式批处理支持 |
| `Data/<Game>` | 各游戏记录定义、编码、词汇表、PEX 排除规则、对话子类型等配置 |
| `Res/<Language>` | UI 本地化资源、手册和教程 |
| `Misc` | API 配置、custom txt 定义、繁简字符表、旧对话格式模板 |
| `Batch` | 批处理示例 |

## 工作模式

xTranslator 的功能不是围绕一种文件格式，而是围绕“把各种来源归一成翻译条目列表”来组织。README 列出的编辑模式包括：

- `Esp mode`：直接加载 ESP/ESM，解析记录字段并在保存时写回插件。
- `Strings mode`：直接编辑 localized 插件配套的三类 strings 文件；README 标注此模式已被 Hybrid Mode 取代。
- `Hybrid Mode`：用 localized ESP 提供 records/fields 结构，用旁边的 strings 文件提供真实文本。保存时写 strings，不直接改 ESP。
- `MCM/Translate`：处理 SkyUI MCM 和其他自定义文本文件，也支持可配置的 custom txt 解析定义。
- `PapyrusPex`：读取 Papyrus `.pex`，暴露可翻译字符串，内部变量或函数名会锁定为不可编辑。
- `BSA/BA2`：归档浏览、提取，并可从归档中加载 strings、MCM/custom txt、PEX 等文件。

主程序通过 `CurrentTESVmode` 标识当前模式，枚举值包含 SST 编辑、Strings、ESP、Hybrid、MCM、PEX。保存逻辑也按模式分派：ESP 调 `FinalizeEsp`，Hybrid/Strings 调 `FinalizeStrings`，MCM 调 `FinalizeMCM`，PEX 调 `FinalizePex`。

## 核心数据模型

`TESVT_typedef.pas` 中的 `tSkyStr` 是工具层的核心抽象。它表示一个可翻译条目，字段包括：

- 原文 `s` 和译文 `sTrans`。
- 源文 hash、译文 hash、词 hash 列表、归一化文本 hash。
- `listIndex`，对应 strings、dlstrings、ilstrings 三个列表。
- `sparams`，保存翻译状态：translated、lockedTrans、incompleteTrans、validated、oldData、pending 等。
- `sInternalparams`，保存运行时状态：aliasError、isOrphean、isLookUpFailed、pexNoTrans、isVMADString、OnTranslationApiArray、isNormalized 等。
- `esp: rEspPointer`，把条目关联到 ESP 记录、字段、formID、record signature、field signature、字符串 ID 和字段序号。
- `VMAD`，把条目关联到脚本属性或 fragment 中的 VMAD 字符串。

这个模型使不同输入格式可以进入同一套编辑、匹配、词典、搜索、状态标记和 API 翻译流程。`tTranslatorLoader` 则是单个文件的工作上下文，持有：

- `listArray[0..2]`：三类字符串列表。
- `EspLoader`：ESP/ESM 结构树。
- `PexDecompiler`：PEX 结构和字符串表。
- `McMData`：custom txt/MCM 的解析与保存状态。
- undo、词典应用状态、shared string id 列表、collab 标签、任务和对话分析缓存等。

## ESP 与记录定义

ESP/ESM 处理在 `TESVT_espDefinition.pas`。它定义通用 record、field、GRUP header 和 loader，不为每个游戏生成强类型记录，而是使用 record signature 与 field signature 进行轻量解析。`tField` 存字段 header、buffer、参数位、最大字符串长度、是否允许换行、所在 strings 列表等；`trecord` 存 headerData、字段列表、EDID、压缩状态和原始 buffer。

可翻译字段主要来自 `Data/<Game>/_recorddefs.txt`。定义行格式类似：

```text
Def_:NAM1=INFO=2*
Def_:CNAM=QUST=1
Def_:FULL=****=0
Def_:DESC=****=1
```

其中第三列指定列表：`0` 为 STRINGS，`1` 为 DLSTRINGS，`2` 为 ILSTRINGS。`*` 表示不能为空；`proc1`、`proc2` 等会绑定到额外判定函数，例如 GMST 或 PERK 的特殊字段处理。定义文件末尾保留 `FULL=****` 和 `DESC=****` 作为跨记录 fallback。

加载 ESP 时，`trecord.getFieldfromBuffer` 逐字段解析 subrecord，处理 `XXXX` 扩展长度、`EDID`、`VMAD`、`NAME` 引用、压缩记录解压，并调用字段定义判断是否为字符串字段。直接 ESP 模式会把字段 buffer 解码成 `tSkyStr`；localized ESP 会先把字段中的 string id 作为占位，后续再通过 strings 文件补齐文本。

保存 ESP 时，`tTranslatorLoader.updateAllRecords` 把每个 `tSkyStr.sTrans` 写回关联字段 buffer，VMAD 字符串另走 `rebuildVMADBuffers`。`tEspLoader.saveEsp` 重新计算 record 和 GRUP 尺寸，保留 raw record 或重写已解析字段，必要时恢复压缩数据。

## Strings 与 Hybrid

`TESVT_StringsFunc.pas` 负责 Bethesda strings 文件。解析流程读取 count、data size、id/offset 表，再按列表类型读取正文：

- STRINGS 使用 null-terminated 字符串。
- DLSTRINGS/ILSTRINGS 在正文前带长度整数。

读取时通过 `getcodepage` 选择编码并转成 Delphi string。`parseOpt` 控制读取目标：可以直接创建条目、按 id 匹配 source、按 id 写入 translation、或用于构建词典。Hybrid Mode 的关键点是先从 localized ESP 建立 `tSkyStr` 和 ESP 字段引用，再加载对应语言的 strings 文件，用 string id 匹配并补齐文本。没有被 ESP 引用的 strings 可以标记为 orphan。

写回 strings 时，`saveStringFile` 会按译文 hash 去重，多个 string id 可以共享同一个正文 offset。输出文件名使用目标语言后缀，按三类列表分别保存。

## SST 词典

SST 是 xTranslator 自有二进制词典格式，由 `TESVT_SSTFunc.pas` 处理。当前保存 header 是 `VocabUserHeader8`，格式保存的信息包括：

- 翻译状态位。
- source、dest、string id、list index、LDResult。
- 轻量 ESP 引用 `rEspPointerLite`。
- collab id 和 collab label。
- master list。

保存前 `prepareSSTFile` 会过滤无意义或风险较高的条目：空字符串、锁定的 PEX/VMAD 字符串、异常字段、未确认翻译等通常不会进入词典。加载时可进入两条路径：基础 source hash 匹配，或带 EDID/record/field 引用的精确匹配。

应用 SST 时，`doApplySst` 先读词典，再根据用户选择的匹配策略调用：

- `findStrMatchEx`：按 source hash/source 文本匹配。
- `findEdidMatchEx`：按 EDID、record/field、string id、VMAD 等上下文匹配。

命中后会复制译文和状态，并处理 incomplete、validated、locked、pending、oldData、derived 等状态。对 VMAD 字符串可单独启用匹配流程。

## 自动匹配与启发式搜索

自动翻译不只靠完全相同文本。`TESVT_TranslateFunc.pas` 还构建 `HeuristicList`，对词典条目做词 hash 预处理，并按词数排序。启发式搜索会先用词数量和词 hash 快速筛掉明显不相关项，再计算近似距离：

- 完全 hash 命中给很低距离。
- 大小写差异有单独权重。
- 单词条目可走最长公共子串逻辑。
- 多词条目走 word distance，并用 source/translation proxy 调整排序。

`tSkyStr` 初始化时会提取 words、alias tag hash、数字和 normalized source。API 翻译和启发式匹配都利用这些缓存，减少重复计算。

工具还支持 derived strings：根据正则和模板从已有词典生成派生译文。README 历史记录中提到“Generate Derived strings”快捷键，源码中对应 `generateDerivedStringData` 和 `generateDerivedStringData(bforceDerived)` 一类逻辑。

## 标签、数字和完整性检查

翻译工具层有大量围绕“译文可安全写回”的检查：

- `checkStringIntegrity` 检查译文是否超过字段最大长度，是否在不允许换行的字段中包含换行。
- `InitTransEx` 通过 alias hash 比对源文和译文里的 `<...>` 标签，必要时做深度标签检查并标记 `aliasError`。
- API 翻译前可把 alias 和数字归一化，减少重复翻译和费用；译文返回后用源文提取的 alias/number 恢复。
- PEX 中被判定为函数、变量或内部标识符的字符串会标记 `pexNoTrans` 并锁定。
- 同语言模式会把源文直接复制为译文，避免误触发跨语言状态。

这些机制说明 xTranslator 的设计重点不是单纯文本替换，而是保留 Bethesda 文件里的结构占位符、脚本标识符、字段约束和上下文一致性。

## 在线翻译 API

`TESVT_TranslatorApi.pas` 封装了多个翻译服务。`MaxApiCount = 7`，API 名称数组包含 Microsoft Translator、Yandex、Baidu、Youdao、freeApi、Google、DeepL、OpenAI。`Misc/ApiTranslator.txt` 提供默认开关、语言名映射、字符限制、数组大小、请求间隔和 OpenAI 默认 prompt。

在线翻译分单条和数组两类：

- 单条函数形如 `OpenAITranslation(var Text, apiId)`。
- 数组函数形如 `OpenAITranslationArray(l: TStringList)`。
- Microsoft、DeepL 有真实数组接口；Google 和 OpenAI 使用“virtual array”，把多条文本用换行拼成一次请求，再拆回列表。

`StartApiTranslationArray` 先统计待翻译条目和字符数，允许用户选择 API、是否自动标记不需翻译文本、是否启用归一化。`StartApiTranslationArrayEx` 再按 API 字符上限、数组上限和每分钟字符配额分批调用。返回后会：

- 恢复 CRLF。
- 恢复归一化的 alias 和数字。
- 将条目标为 `incompleteTrans`。
- 对相同 normalized source 的其他条目复用结果。
- 对 virtual array 拆分失败、归一化恢复失败、限速等待等情况做重试或降级。

OpenAI 封装使用 Chat Completions 格式，读取可配置 URL、model 和 prompt，把 source/dest 语言替换到 prompt 中，再将文本追加为用户消息。

## PEX 支持

`TESVT_scriptPex.pas` 实现 Papyrus PEX 读取和写回。它读取 PEX header、字符串表、debug 信息、user flags、objects、variables、properties、states、functions 和 instructions。字符串表中的每个字符串都有 `auth` 和 `warn` 状态，后续加载到 `tSkyStr` 时：

- `auth=false` 或满足 `$` 开头等规则的字符串会标记 locked。
- 可疑但可编辑的字符串标记 warning。
- 可翻译字符串默认 source 和 translation 相同，进入普通编辑列表。

保存 PEX 时，工具保留原始 header buffer 和 data buffer，只重写字符串表。这种设计把 PEX 当作“字符串表可编辑，其他结构只用于判断哪些字符串可编辑”的格式，而不是完整脚本重编译器。

## MCM 与 Custom Txt

MCM/custom txt 由 `tMcMData` 处理。README 在 1.5.5 记录中提到 custom txt 导入已重写，支持单行字符串定义的自定义文本类型，解析定义位于 `Misc/customTxtDefinition.txt`。`tMcMData` 存原始内存流、导出流、header 列表、compare header 列表和 normalized 列表，提供 `parseMCM`、`saveMCM`、`doCompareMCM`。

和 ESP/Strings 一样，MCM/custom txt 最终也变成 `tSkyStr` 列表，之后可以使用同一套搜索、替换、词典应用、API 翻译和保存流程。

## XML 导入导出

`TESVT_XMLFunc.pas` 使用 OmniXML。导出的根节点是 `SSTXMLRessources`，包含：

- `Params`：addon、source、dest、version。
- `Content/String`：ID、EDID、REC、Source、Dest。
- 可选 Fuz 信息。

导入支持 xTranslator 自身 XML，也支持旧 ESP-ESM Translator 风格的 XML。导入匹配策略和 SST 类似，可以按 REC/EDID/context 精确匹配，也可以回退到普通 source 匹配。

## 对话、FUZ 和上下文数据

工具对对话有专门支持。`tSkyStr.isDialog` 把 `INFO:NAM1` 视为对话行，`isDialogEditing` 包含 INFO 和 DIAL。源码中还有：

- `TESVT_Fuz.pas`：FUZ 映射和播放器。
- `TESVT_NPCMap.pas`：NPC map 和对话说话人相关分析。
- `buildQuestsList`、`assignDialogNAM`、`ImportDialData` 等主窗体逻辑，用于组织对话、任务和旧对话显示。

这让翻译界面不只是表格，还能围绕 DIAL/INFO/QUST 提供更接近游戏语境的编辑视图。

## 搜索、替换和批处理

搜索能力分多个层次：

- 直接搜索和高级搜索。
- 正则搜索和替换。
- 批量 search/replace。
- Heuristic search。
- 字典树拖放和词典排序。

`TESVT_Batcher.pas` 定义规则对象，支持按 record/field、关键字、component、fallback、regex、header template 等条件处理文本。主窗体的 `runCommands` 又提供命令式批处理，支持：

- `loadfile`
- `apitranslation`
- `loadmasters`
- `finalize`
- `closefile` / `closeall`
- `savedictionary`
- `generatedictionaries`
- `applysst`
- `importsst`
- `importxml`

批处理文件可以设置全局词典目录、导入目录、导出目录、source/dest 语言和导出子目录。README 历史记录也说明后续版本把 API translation 和 save dictionary 纳入批处理命令。

## 多语言和编码

项目有两层多语言：

1. 处理游戏文本的 source/dest language。
2. 工具 UI 自身的资源语言。

游戏文本层由 `Data/<Game>/codepage.txt`、全局 codepage 配置和 `getcodepage` 决定读取/写入编码。不同游戏目录还包含 `vocabulary.txt`、`pexNoTransProc.txt`、`DialSubType.txt`、`ctdaFunc.txt`、`fieldSizeRef.txt` 等。Starfield 和 Fallout 76 还包含 HeaderProcessor 默认规则。

UI 本地化放在 `Res/<Language>`，每种语言有 `Res.ini`、`manual.htm`、`Tutorial.htm` 和 `header.tpl`。README 提到 UI 字符串可以通过新增资源语言目录进行翻译，扩展字符集语言通常以 UTF-8 with BOM 保存。

## 归档支持

BSA/BA2 支持贯穿加载和导出。工具可以浏览归档、提取 strings、MCM/custom txt、PEX，也可以在保存时准备注入归档。归档查找依赖默认 BSA 定义和 alias 列表；`loadAddonStrings` 会按选项决定优先 loose file 还是 archive file。README 和源码注释都注明 BSA/BA2/Stream loading 有部分代码来自 xEdit。

## 实现风格

xTranslator 的功能设计偏“工作台”而不是库：

- 大量全局偏好、当前游戏、当前语言和 UI 状态集中在 `TESVT_Const.pas` 与主窗体。
- 文件加载后进入 `tTranslatorLoader`，再统一暴露为 `tSkyStr` 列表。
- UI、状态位、文件格式、词典匹配和保存流程耦合较紧，但功能边界在 unit 层面仍能看出分工。
- 游戏差异主要通过配置文件、默认常量和少量代码分支处理，而不是强类型记录模型。
- 重要操作都服务于翻译员工作流：加载、上下文识别、词典命中、人工编辑、状态标记、批量替换、在线翻译、检查、最终写回。

## 参考资料

- [MGuffin/xTranslator - GitHub](https://github.com/MGuffin/xTranslator)
- [xTranslator README](https://github.com/MGuffin/xTranslator/blob/main/README.md)
- [Nexus Mods xTranslator 页面](https://www.nexusmods.com/skyrimspecialedition/mods/134)
- `README.md`
- `TESVT_main.pas`
- `TESVT_MainLoader.pas`
- `TESVT_typedef.pas`
- `TESVT_espDefinition.pas`
- `TESVT_StringsFunc.pas`
- `TESVT_SSTFunc.pas`
- `TESVT_TranslateFunc.pas`
- `TESVT_TranslatorApi.pas`
- `TESVT_scriptPex.pas`
- `TESVT_XMLFunc.pas`
- `TESVT_Batcher.pas`
