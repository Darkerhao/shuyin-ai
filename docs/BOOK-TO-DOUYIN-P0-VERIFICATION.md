# 书籍到抖音全自动流程 P0 核验

后续范围修订：用户允许最后由本人点击抖音发布，因此本报告保留为官方API无人值守路线的历史研究，不再阻塞当前网页发布准备流程。已实施范围及验证见 [自动流程验收](AUTOMATION-VERIFICATION.md)；本报告不推翻已核实的接口事实，也不宣称官方全自动准入通过。

核验日期：2026-11-07（北京时间）。依据：[全自动创作与发布计划](BOOK-TO-DOUYIN-AUTOMATION-PLAN.md)第 4、8、9 节。范围：公开官方接口文档及其随页面提供的完整请求/响应结构；未进入控制台、读取凭据或调用平台业务接口。

**结论：P0 尚未通过，不能将全自动发布标为可用。** 新版创建接口及禁止下载枚举已有明确公开依据，但本次可读的官方契约没有证实原生定时、AI 声明、设置回读和实际公开结果的完整闭环；本应用获批能力、免逐作品确认规则和授权测试作品回执也尚未验证。这是现有证据不足，不是认定平台绝对不支持这些能力。

本次没有实施发布服务、数据库迁移或自动投稿代码。后续代码不能根据字段名称补造 `scheduled_at`、`aigc` 等平台参数，也不能把本机到点调用立即发布或自动点击网页最终按钮作为计划要求的原生定时能力。

## 1. 能力矩阵

| 必需能力 | 本次核验事实 | 当前判定及缺口 |
| --- | --- | --- |
| 官方创建 | `POST /api/douyin/v1/video/create_video/`，Scope 为 `video.create.bind`；能力实验室申请「代替用户发布内容到抖音」，需要用户授权。[S1] | 公开契约可读；本应用准入及目标账号授权未验证 |
| 视频上传 | `POST /api/douyin/v1/video/upload_video/`，同一 Scope；返回加密的视频文件 ID。[S2] | 公开契约可读；未上传试验素材 |
| 分片上传 | 文档写超过 50 MB 建议分片、超过 300 MB 必须分片、总大小 4 GB 内；建议每片 20 MB、最小 5 MB。[S2] | 新版初始化、分片、合并、恢复契约未取得可核验正文，不直接套用旧端点 |
| 禁止保存 | 创建请求 `download_type`：`0` 可以下载，`1` 作者不允许下载。[S1] | **枚举已核实，应映射 `allowDownload=false` → `download_type=1`**；尚未核实受理后回读方式和实际效果 |
| 公开可见设置 | `private_status`：`0` 全部人可见，`1` 自见，`2` 好友可见。[S1] | 请求枚举已核实；请求 `0` 不代表审核完成或最终公开 |
| 原生定时 | 递归读取创建请求全部子字段，未发现预约时间、预约窗口或改期字段。[S1] | 北京时间 01:11 原生预约及回读未证实；A15–A17 无法据现有契约通过 |
| AI 内容声明 | 同一完整创建请求结构未发现 AI/AIGC 声明字段。[S1] | 需获准路线的正式表达方式；不能用正文文字冒充平台声明 |
| 官方封面 | `custom_cover_image_url` 传图片上传得到的 `image_id`，不是任意 URL；图片上传同需 `video.create.bind`。[S1][S3] | 单封面参数可证实；独立横竖双封面映射、3:4 采用效果尚未实测 |
| 无人值守交互 | SDK/H5 规范明确用户手动操作发布，但该能力分类表未列明本次 `video.create.bind` 实验室能力的具体交互约束。[S4] | 不能用 SDK/H5 通过零点击验收；也不能把 SDK/H5 规则直接认定为该实验室能力的全部规则 |
| 创建回执 | 正常回包含明文 `video_id`、密文 `item_id` 及错误信息；说明明确创建后会审核，审核期间只有自己可见。[S1] | 创建受理不等于公开；该回包不提供预约时间或下载设置回读 |
| 查询作品 | `POST /api/douyin/v1/video/video_basic_info/` 的 Scope 是 `posting.behavior`，申请的是「用户投稿任务」；只查询对应授权用户上传的作品。[S5] | 不能认为获得 `video.create.bind` 就能调用；实际适用性、所需权限和完整状态语义未核实 |
| 状态枚举 | 基础信息示例包含 `video_status: 5`；本次随页正式 `RespSchema` 没有该字段，其值也没有枚举说明。[S5] | 不能把示例 `5` 硬编码为公开成功，更不能用 `create_time` 冒充实际公开时间 |
| 发布结果/回调 | 「查询视频发布结果」是 `/share-id/`、Scope `aweme.forward`；事件表 `create_video` 同属 `aweme.forward` 分享事件，消息包含 `share_id`。[S6][S7] | 不是已经证实适用于本路线的审核完成查询/回调 |
| OAuth 与凭据 | Token 交换为表单 POST，需 `client_key`、`client_secret`、`code`、`grant_type=authorization_code`；官方建议服务端保存 Token。[S8] | 需要本应用的获准配置及服务部署；安装包不能携带共享 Secret |
| 续期 | 授权说明写 access token 15 天、refresh token 30 天、最长续期 195 天；并发重复消费刷新凭据可令其失效，用户撤销立即失效。[S9] | 应按实际响应有效期并串行刷新；本次未验证真实授权及续期 |
| 内容与媒体限制 | 创建文字含话题总长不超过 1000；创建/上传错误码列 15 分钟限制；创建错误码列单用户单应用每日 75 个作品。[S1][S2] | 这些是文档值，不是目标账号额度保证；真实音频、成片和上传均未实测 |
| 生产链前置条件 | 本次研究只覆盖官方发布公开文档。 | NotebookLM 支持格式/实际时长、图片 Provider 两比例产物及视频环境没有在本轮重新验收 |

## 2. 已核实的请求与回执边界

创建视频使用查询参数 `open_id` 和请求头 `access-token`，正文必填 `video_id`。上传得到的是加密视频文件 ID，创建正常返回的 `data.item_id` 是密文作品 ID，`data.video_id` 是明文作品 ID；不能把上传文件 ID 与创建后作品 ID 混作同一状态。[S1][S2]

创建主请求字段共 21 项：

```text
agent_client_key, at_users, cover_tsp, custom_cover_image_url,
download_type, micro_app_id, micro_app_title, micro_app_url,
music_begin_time, music_end_time, music_id, music_volume,
poi_commerce, poi_id, private_status, publish_clip, publish_way,
task_id, text, video_id, vr_transcode_extra_params
```

以上列表从随页 `ReqSchema.BodyParam` 键集合提取。`publish_clip` 子字段是幻灯片投稿资源与时长，`vr_transcode_extra_params` 是全景视频参数，均未提供原生预约语义。[S1]

仅传 `download_type=1` 可以按公开契约表达禁止保存，但计划还要求平台受理后的设置核对。本次创建 `RespSchema` 的 `data` 只有 `item_id`、`video_id`、`error_code` 和 `description`，不能从请求原值或 HTTP 200 自证平台采用了禁止保存和预约时间。[S1]

基础信息查询正文支持 `item_ids` 和 `video_ids`；文档写二者同时传时优先 `video_ids`。其正式响应结构包含封面、创建时间、媒体类型、作品 ID、标题及错误信息，没有明确预约、禁止下载、审核完成时间或公开链接字段。它可以作为待验证候选，但目前不能完成本计划的结果归档契约。[S5]

### 2.1 当前代码核对

实施基线为 `dev` / `91461aef64b1233085d05be90d1b6f4ec06e5ded`，本地应用 0.51.1、schema v19。以下均核对当前源码，不把历史设计接口当成已经实现：

| 位置 | 当前行为与缺口 |
| --- | --- |
| [douyin.rs](../src-tauri/src/douyin.rs)、[douyin.ts](../src/services/douyin.ts) | 打开官方创作者网页并维护辅助填写会话；没有官方 OAuth、创建和审核查询适配器 |
| [audio.rs](../src-tauri/src/database/audio.rs) 的 `StartAudioJobInput` | 启动必须传入 `writing_version_id`，尚无从固定指导直接生成原书音频的自动入口 |
| [video_commands.rs](../src-tauri/src/video_commands.rs) 的 `execute` | 准备完成进入 `awaiting_confirmation`，渲染通过另一显式操作启动 |
| [publishing.rs](../src-tauri/src/database/publishing.rs) 的 `checked` / `record_publish_success` | 清单依赖人工确认标题/封面，记录成功要求 `input.confirmed`；没有官方回执事务 |
| [database.rs](../src-tauri/src/database.rs) 的 `MIGRATIONS`、[lib.rs](../src-tauri/src/lib.rs) 的命令注册 | 迁移到 `0019_editor_owners.sql`，没有自动流程表；只注册现有网页发布命令 |

对 `src`、`src-tauri`、`scripts`、`.github` 的相关符号搜索未发现官方接入服务或配置。这不能证明用户的外部应用未获批，只说明仓库和本次任务没有提供可核验的接入资料。未读取系统凭据或用户登录数据来推断授权。

## 3. 当前阻塞及所需接入材料

在写正式自动发布适配器前，需要补齐以下**非密钥资料与验证结果**：

1. 本项目开发者主体、应用类型和应用状态，以及已获批 `video.create.bind` 和配套查询能力的证明；公开资料不能代替控制台审批结果。
2. 平台针对该获准能力允许免逐作品确认的说明，以及 AI 内容声明的正式接入规范。
3. 原生定时的官方字段/端点、时区和预约窗口、改期规则，以及预约时间与禁止下载的可靠回读契约。
4. 与本路线匹配的分片协议、作品状态查询/回调权限、状态枚举、真实公开时间/链接取得方式，以及创建结果未知时的核对方法。
5. 已确认的发布服务域名、OAuth 回调完整 URL 与部署环境；Secret 和 Token 应在服务端或系统凭据设施配置，不在聊天、文档或 Git 中提供。
6. 授权测试账号与可发布试验内容的实际回执：一次完整预约、禁止保存回读、等待审核和公开核验，并补做计划所列生产链验收。

取得这些资料后继续原计划 P0 真实验证，再按 P1–P4 逐步实施。当前可以研究本地编排，但不得把仅有本地产物或假 API 回执的实现作为本次全自动目标完成，也不得由本报告推导出已授权执行任意真实账号投稿。

## 4. 实际执行与证据

- 已用 PowerShell 调用 Python 标准库 `urllib.request`，只读 GET 9 个主要官方文档；9 页均返回 HTTP 200，HTML 均保留在忽略目录 `output/douyin-automation-p0/`。
- 用 `html.parser.HTMLParser` 去除 script/style 提取可见正文，保存对应 `*.txt`；用 `json.JSONDecoder().raw_decode` 解析页面 `_ROUTER_DATA`，递归检查 `openApiMeta.ReqSchema` / `RespSchema`，覆盖网页折叠子属性。
- 读取官方 OpenAPI 列表及从基础信息页链接的 `account-video-list`。列表可读，后者本轮只有 318 字站点外壳；没有把 HTTP 200 当作获得接口正文。[S10][S11]
- 对旧站分片初始化、上传、合并文档入口进行只读探查，三页均 HTTP 200，但仅 54 字站点外壳，未取得协议。没有调用这些路径对应的业务接口，也没有把旧地址当成已经验证的新协议。
- 本轮证据文件包括 `fetch-results.json`、`followup-results.json`、`create-router.json`、`basic-info-router.json`、`create-all-fields.txt` 和 `basic-info-all-fields.txt`。它们是公开文档快照，供本机复核，不包含用户凭据。
- PowerShell here-string 执行 `python -` 检查本报告 UTF-8、冲突标记、行尾空白、11 组官方引用、本地计划链接，并比对原始结构中的 21 个正文参数和 4 个创建回执字段，全部通过；`git check-ignore output/douyin-automation-p0/create.html` 确认原始证据在忽略目录。
- 本报告无产品代码、依赖、版本或数据库变更；未运行 lint、TypeScript、单元测试、Rust 检查、构建、安装或真实平台发布。

验收对应：本轮完成公开契约核对及 P0 缺口识别；A1–A20 均不能据本报告标为完整通过。尤其 A13（免逐次确认）、A15–A17（原生预约及设置核对）、A19（整链零点击）缺少外部准入或真实回执证据。

## 5. 官方来源

以下链接均在 2026-11-07 本轮只读访问；引用只覆盖取得有效正文或随页结构的事实。

[S1]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/video-create
[S2]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/upload-video
[S3]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-image-text/image-upload/
[S4]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/operation-standard/platform-capabilities/useclue
[S5]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/posting-task/video-basic-info
[S6]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/search-video/video-share-result
[S7]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/webhooks/event-list
[S8]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/account-permission/get-access-token
[S9]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/sdk/mobile-app/permission/overall-permission
[S10]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/list
[S11]: https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/search-video/account-video-list
