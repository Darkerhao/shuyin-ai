# 工作台视觉升级与验收

日期：2026-11-10

## 实现范围

| 区域 | 实现 | 业务约束 |
| --- | --- | --- |
| 书架 | 书封边缘、纸页层次、指针跟随高光和小幅倾斜，筛选/排序过渡，续作图标 | 原搜索、菜单、路由及继续任务保持有效 |
| 自动制作 | 六组阶段、真实当前步骤、暂停/待确认样式、任务和成果卡 | 不新增百分比、不将任务ID视为成品、不将准备发布视为已发布 |
| 普通后台任务 | 紧凑图标卡、新任务短入场、完成/失败标识 | 沿用generationJobs和workflow快照 |
| 封面工坊 | 画布展示台、等高陈列的三比例缩略图、选中抬升与勾选 | 不改变Canvas像素、原尺寸、裁切与导出逻辑 |
| 封面历史 | 展开后读取最近六张已导出封面，点击/键盘放大、Esc关闭及焦点返回 | 查看与恢复配置分开，不写库或覆盖未保存编辑 |
| 共享交互 | 按钮按压与导航颜色/边框短过渡 | 深浅主题、键盘焦点和prefers-reduced-motion继续有效 |

修改文件：`BookCard.vue`、`ProjectsPage.vue`、`AutomationTasks.vue`、`RunningTasks.vue`、`CoversPage.vue`、`CoverThumbnail.vue`、`components/ui/button/index.ts`、`style.css`；补充阶段测试和浏览器验收脚本，并同步PRD、技术设计与开发计划。

数据库仍为v23，无迁移、业务接口、权限、依赖或版本号变更；未提交、推送或打包发布。React Bits Pro仅作为公开设计参考，代码在现有Vue体系实现。使用的Vue TransitionGroup与Reka Dialog核对了本仓库安装源码及官方文档。

## 验收标准

- [x] 书架跟随与归位、搜索与网格/列表切换正常。
- [x] 真实阶段决定完成标记，暂停/等待人工发布有明确区分；旧流程不套用新流程顺序。
- [x] 三比例封面展示完整，切换尺寸保留各自未保存编辑。
- [x] 历史放大不采用旧配置、不触发生成/正式保存，关闭后返回原按钮。
- [x] 深浅主题下书架、任务卡和封面工坊共22组布局无页面横向溢出。
- [x] 减少动态效果时书封无变换；沿用全局CSS关闭其他非必要动画。
- [ ] Windows WebView2安装版、系统缩放及真实生成/发布任务人工验收。

## 实际命令与结果

- `npx eslint src/components/AutomationTasks.vue src/components/RunningTasks.vue src/components/CoverThumbnail.vue src/components/BookCard.vue src/modules/projects/ProjectsPage.vue src/modules/covers/CoversPage.vue src/components/ui/button/index.ts tests/automation-tasks.test.ts --fix`：格式修正成功。
- `npm run build`：TypeScript与生产构建通过。保留现有单分包超过500kB提示。
- `npx vitest run tests/automation-tasks.test.ts tests/automation.test.ts tests/ui-refinement.test.ts tests/covers.test.ts tests/coverRenderer.test.ts tests/generation.test.ts tests/theme.test.ts tests/voice-reminders.test.ts`：首次111通过/1失败，新测试的CSS属性选择器未匹配含&的href；改为核对实际链接属性后，`npx vitest run tests/automation-tasks.test.ts` 4项通过。
- `npm run lint`：通过。
- `npx vitest run`：首次470通过，updates与voice-reminders各1项触发既有5000ms超时；未调整超时或断言，改用`npx vitest run --maxWorkers=2`，34个文件472项全部通过。
- 最后将预览统一为现有Reka弹窗体系、历史图片改为展开才加载后，重新执行`npm run lint`、`npm run build`均通过；`npx vitest run tests/covers.test.ts tests/automation-tasks.test.ts tests/ui-refinement.test.ts --maxWorkers=2` 68项通过。
- `npm run dev -- --port 1421`：启动本地Vite验收服务。
- `npx --yes --package @playwright/cli playwright-cli -s=shuyin-visual open http://127.0.0.1:1421 --browser chrome`：启动独立浏览器会话。
- `npx --yes --package @playwright/cli playwright-cli -s=shuyin-visual run-code --filename output/playwright/visual-upgrade-run.js`：22组布局与书架、任务、草稿、尺寸和预览交互通过，pageerror为0，未知IPC为0，未触发生成或封面正式保存。
- 最终同脚本加`--raw`重跑通过，补验放大时Ctrl+1不跳走、键盘Enter可打开预览、390×800下弹窗完整可见。重复执行时命中既有未保存内容离页保护，仅接受隔离夹具的beforeunload提示，不操作正式数据。
- `git diff --check`：通过。

浏览器脚本为`tests/browser/visual-upgrade.mjs`。CLI不支持动态import回调且Windows命令行长度有限，将脚本的`export default async function verifyVisualUpgrade(page) {`替换为`async page => {`生成到忽略目录中的`output/playwright/visual-upgrade-run.js`，通过`--filename`执行。搜索验证等待Vue Router更新完成，不使用固定延时。

## 截图与验证边界

截图使用隔离的内存IPC数据，真实Vue组件、样式、Canvas和Reka交互；示例书籍与任务不代表用户正式数据库。

![深色书架](../output/playwright/visual-shelf-dark.png)
![深色制作流程](../output/playwright/visual-tasks-dark.png)
![浅色封面工坊](../output/playwright/visual-covers-light.png)
![历史封面放大](../output/playwright/visual-cover-preview.png)

截图保存在本地忽略的output目录，不随Git发布。未运行Rust、视频渲染、发布脚本测试或桌面打包；本轮没有修改这些执行层。后续在Windows桌面版用实际书籍确认指针动效、封面预览和制作阶段视觉反馈。

## 追加评审与修复验证（2026-11-10）

基线为`5a04cee7eaac01fbb45a047e928370fa928bef51`（当前HEAD），范围为本轮未提交工作区及新增测试/文档。使用code-review技能，由两个独立只读评审分别检查Standards与Spec，主代理复现、修复和执行验证。

### Standards

初评1项P2：`AutomationTasks.vue`以`output.to`作为TransitionGroup的key，内容校核没有修改时，后端会复用初稿版本，两张成果卡出现重复key。改为writing、checked-writing等用途key，既保留两个业务入口，又在目标版本更新后保持同一卡片节点。新增测试先复现3张卡仅2个唯一key，修复后通过；浏览器进一步验证真实TransitionGroup中目标URL复用、更新和节点身份。独立复审无剩余确定问题。

### Spec

初评1项P2：历史封面预览期间Ctrl/Cmd+S冒泡至页面保存监听，意外追加后台三尺寸配置。新增回归先确认saveCover被调用3次，再在保存入口加入对已打开Dialog/AlertDialog的检查；预览期间不保存，关闭后快捷键照常保存三尺寸。独立复审无剩余确定问题。

另补齐1处验收覆盖缺口：旧脚本在800/390宽度只检查默认编辑视图，展示台当时隐藏。现显式切换“查看实时预览”，检查画布可见、三比例正确及页面无横向溢出。此前22组结果仍为已执行事实，最终扩展为26组。

### 修复后实际命令

| 命令 | 结果 |
| --- | --- |
| `npx vitest run tests/covers.test.ts -t 'keeps save shortcuts inside' --maxWorkers=2` | 修复前按预期失败，确认预览期间发生3次保存 |
| `npx vitest run tests/automation-tasks.test.ts tests/covers.test.ts -t 'keeps separate stable\|keeps save shortcuts inside' --maxWorkers=2` | 保存隔离通过，重复key在修复前按预期失败 |
| `npx vitest run tests/automation-tasks.test.ts tests/covers.test.ts --maxWorkers=2` | 两处修复后61项通过 |
| `npm run lint` | 通过；后续最终浏览器脚本另用`npx eslint tests/browser/visual-upgrade.mjs`复核通过 |
| `npm run build` | TypeScript及生产构建通过，保留单分包超过500kB的现有提示 |
| `npx vitest run --maxWorkers=2` | 最终34个文件474项全部通过 |
| `npx --yes --package @playwright/cli playwright-cli -s=shuyin-review-final run-code --filename output/playwright/visual-review-run.js --raw` | 26组布局及10类交互通过，pageerror/重复key警告为0，未知IPC为0，生成/正式保存调用为0 |
| `git diff --check` | 通过 |

浏览器仍使用隔离IPC，并加载真实组件、CSS、Canvas和现有生成状态服务。最终覆盖书封跟随/归位、实际书名排序/搜索/视图切换/菜单焦点、减少动态效果、初稿与校核版本复用、暂停及最终人工发布边界、普通后台生成的运行/失败/成功/取消、窄屏预览及比例、历史按需读取、三尺寸独立草稿、预览键盘保存隔离和焦点返回。

本次评审修复仅修改两个业务UI文件、两个测试文件、浏览器验收脚本及相关文档；无数据库、依赖、权限或版本号变化。最终结论：Standards初评1项已关闭、剩余0项；Spec初评1项及1处覆盖缺口已关闭、剩余0项。下一步在Windows WebView2桌面版检查系统缩放、实际封面和真实任务；浏览器隔离验收不代表安装包或真实平台发布已验收。
