export default {
  // 相对路径以本文件所在目录为起点。也可填写其他项目的绝对路径。
  cwd: '.',
  outputDir: 'output/codex-queue',
  sandbox: 'workspace-write',
  timeoutMinutes: 120,
  // model: '填写模型 ID', // 不填则沿用本机 Codex 配置。
  instructions: `
使用中文工作和汇报。遵循项目 AGENTS.md，完成实现、适用的检查和文档更新。
最终由我验收；不要自行提交、推送、发布、安装软件或操作正式业务数据。
`,
  tasks: [
    {
      id: 'task-1',
      title: '第一个任务',
      enabled: false, // 写好 prompt 后改成 true；所有示例默认不执行。
      prompt: `
在这里写第一个任务，可以换行。
说明目标、影响范围和验收标准。
`,
    },
    {
      id: 'task-2',
      title: '第二个任务',
      enabled: false,
      prompt: `
在这里写第二个任务。
可以基于第一个任务留下的代码继续实现；这是一个新的 Codex 会话。
`,
    },
    {
      id: 'task-3',
      title: '第三个任务',
      enabled: false,
      prompt: `
在这里写第三个任务。
每项任务请给出明确的验收标准，最终统一由我验收。
`,
    },
  ],
}
