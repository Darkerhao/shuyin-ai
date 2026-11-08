# 自动制作 P4 官方接入复核（2026-11-08）

**结论：公开核验已完成，最终自动提交尚未交付。** 本轮重新读取9份官方文档及随页完整请求/响应结构，创建、上传、查询的契约与[11月7日P0证据](BOOK-TO-DOUYIN-P0-VERIFICATION.md)一致。无法据公开文档确认本应用的获批资格，也尚未取得原生定时、AI声明、设置回读和实际公开结果的完整契约，不能实现后宣称通过A17。

制作P2/P3继续完成，不受本项阻塞；当前产品准确停在等待用户最终发布。没有新增猜测参数、OAuth服务、假回执或网页自动点击最终发布。没有调用投稿、授权或平台业务接口，没有读取用户登录数据/凭据，也没有真实投稿。

## 当前证据与缺口

| 能力 | 本次实时证据 | 尚缺条件 |
| --- | --- | --- |
| 官方创建/上传 | `POST /api/douyin/v1/video/create_video/`、`upload_video/`，需要`video.create.bind` | 本应用能力实验室获批准入及目标账号实际授权 |
| 保存权限 | 创建参数`download_type`仍明确0允许下载、1禁止下载 | 实际受理后的可靠回读与平台效果 |
| 原生定时 | 创建请求仍为21个主字段，递归子字段未提供可证实的预约时间/预约窗口语义 | 北京时间01:11原生预约字段、窗口、改期及回读契约 |
| AI声明 | 同一正式创建请求没有已核实的AI/AIGC字段 | 获准路线正式声明方式 |
| 封面 | `custom_cover_image_url`接收上传图片ID | 双封面映射与实际效果；16:9仍只在本地归档 |
| 创建结果 | data返回item_id、video_id、error_code、description；创建后仍需审核 | 不得将受理或HTTP 200标为公开成功 |
| 作品查询 | `video_basic_info/`要求`posting.behavior`；正式RespSchema未给出示例video_status的枚举含义 | 与本路线匹配的权限、状态枚举、公开时间/链接、未知提交核对 |
| 无人值守 | SDK/H5规范要求手动发布，不能直接替代实验室能力的交互规则 | 该获准能力免逐作品确认的官方许可 |
| 应用服务 | 仓库相关源码/脚本/CI未检出官方创建、`video.create.bind`、`posting.behavior`或OAuth回调实现 | 应用类型/状态、服务域名和回调地址、服务端凭据配置方案 |

没有公开字段证据不等于平台永久不支持。用户提供当前控制台能力证明及官方补充契约后，可以基于同一发布任务实施最小正式适配器，不需要重建制作流程。Secret/Token应配置在系统凭据或服务端，不能写进聊天、普通配置或仓库。

## 实际命令与结果

- `python output/douyin-automation-p4/refresh.py`：9页HTTP 200，提取可见正文与`_ROUTER_DATA.openApiMeta`，保存摘要及原文。只读GET，未访问业务接口。
- Python递归比较当前create/upload/basic-info的ReqSchema/RespSchema与P0快照：6组完全相同；创建21个主参数，回执data仍为4个字段。结果在`output/douyin-automation-p4/contract-comparison.json`。
- `rg -l -i 'video\.create\.bind|create_video/|posting\.behavior|client_secret|oauth.*callback' src src-tauri/src scripts .github -g '!*.lock'`：未命中。这只证明本次检索范围没有接入实现，不代表用户外部应用未获批。

全部快照位于忽略目录`output/douyin-automation-p4/`，无凭据。P4没有新增数据库、依赖、权限、安装或发布服务。

## 下一步所需资料

当前应用获批能力和授权状态、免逐作品最终点击规则；原生预约/AI声明/下载设置回读及查询状态的官方规范；确认的服务域名和OAuth回调。取得后按[主计划](BOOK-TO-DOUYIN-AUTOMATION-PLAN.md)A17验证真实受理、未知回执核对、账号变化、重复触发及公开结果归档。真实投稿需在明确的账号和内容验收范围内执行。

## 本轮访问的官方来源

- [创建视频](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/video-create)
- [上传视频](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-video/upload-video)
- [图片上传](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/create-image-text/image-upload/)
- [能力使用规范](https://developer.open-douyin.com/docs/resource/zh-CN/dop/operation-standard/platform-capabilities/useclue)
- [作品基础信息](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/posting-task/video-basic-info)
- [分享发布结果](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/video-management/douyin/search-video/video-share-result)
- [事件列表](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/webhooks/event-list)
- [获取授权Token](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/openapi/account-permission/get-access-token)
- [授权说明](https://developer.open-douyin.com/docs/resource/zh-CN/dop/develop/sdk/mobile-app/permission/overall-permission)
