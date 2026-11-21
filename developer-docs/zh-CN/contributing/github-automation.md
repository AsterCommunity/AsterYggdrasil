# GitHub 自动化

AsterYggdrasil 使用固定 commit SHA 的 [`AsterCommunity/aster-automation`](https://github.com/AsterCommunity/aster-automation) Action 和组织 GitHub App `AsterCommunity Automation` 维护 PR 元数据、关联 Issue 生命周期、聚合 CI 诊断和默认分支故障 Issue；产品路径、标签、workflow 与提示规则保留在默认分支的 `.github/aster-automation.json`。自动化只消费 GitHub API 数据，不 checkout 或执行外部 PR 分支代码。

前置条件：`AsterCommunity Automation` App 需要安装到 AsterYggdrasil 仓库，且组织级 secrets `ASTER_AUTOMATION_APP_ID` / `ASTER_AUTOMATION_PRIVATE_KEY` 对本仓可见（与 AsterDrive 共用同一 App 与 secrets）。

## PR 元数据

`.github/workflows/pr-automation.yml` 监听 `pull_request_target`，根据 changed files 确定性维护以下信息：

- 语言与文档标签：`Rust`、`TypeScript`、`Documentation`、`Dependencies`
- 产品范围标签：现有 `Scope:*`（Auth、Yggdrasil Protocol、Profiles、Textures、Texture Library、External Auth、Admin UI、Runtime、Config、Mail、Docs）
- 高风险边界：migration、认证、CSRF、passkey、封禁、Yggdrasil 协议与签名、texture storage 和 workflow 变更使用 `Risk: High`

自动化只维护可由文件路径证明的标签。`Priority:*` 仅在所有 closing issue 给出同一个优先级时继承；冲突或缺失时不猜测。脚本保留人工添加的非托管标签。

PR 使用 GitHub 原生 closing keyword（例如 `Fixes #123`）关联 Issue 后，自动化会为 Issue 增加 `Wait For PR`，并在没有其他状态时增加 `Status: In Progress`。合并或关闭后，在不存在另一个 open closing PR 时移除这两个临时状态；成功合并还会给 PR 增加 `Merged`。Issue 是否关闭仍由 GitHub 原生 closing 语义决定。

仅用于验收跟踪、但合并 PR 不应关闭 Issue 时，在 PR 正文使用严格字段 `Tracking-Issue: #123`。scheduled reconciliation 会同步该 Issue 的 `Wait For PR` 和进行中状态，但不会关闭它；正文中的普通 `Related to #123` 不参与自动状态迁移。

## CI 聚合与 PR Gate

`.github/workflows/ci-diagnostics.yml` 在各 CI workflow 完成后重新读取 PR 最新 HEAD 的检查状态：

1. 根据与现有 workflow 相同的 path filter 计算本次需要运行的检查。
2. 将结果汇总为稳定的 `PR Gate` Check Run。
3. 相关 CI 仍在运行时添加 `CI: Running`；至少一个相关 workflow 全部成功时切换为 `CI: Passed`。失败、取消、新 commit 或 PR 关闭/合并会清理这两个 CI 生命周期标签；没有相关 CI 的元数据变更不添加 `CI: Passed`。
4. 失败时创建一条包含隐藏 marker 的 PR 评论；后续运行只更新该评论。
5. 新 commit 到达后只处理当前 HEAD，旧 SHA 结果不会覆盖新诊断。
6. 所有必要检查恢复后，评论更新为 resolved。

`PR Gate` 只汇总已有 CI，不重新执行测试。添加或修改 path-filtered workflow 时，必须同步更新 `.github/aster-automation.json` 的 `workflows` 路径规则；共享状态机和路径匹配测试由 `AsterCommunity/aster-automation` 持有，AsterYggdrasil 的 `Repository Automation` 验证本仓配置和 workflow 静态契约。

## 默认分支与定时任务故障

默认分支或 scheduled workflow 失败时，自动化使用 `workflow + branch + failed job set` 生成稳定 fingerprint：

- 首次失败创建带 `CI: Failure` 的 Issue。
- 同一 fingerprint 再次失败更新原 Issue 和出现次数。
- runner、Docker、网络、磁盘或超时信号同时增加 `CI: Infrastructure`。
- 同一 fingerprint 达到三次后增加 `CI: Flaky`。
- Issue 明确记录 `awaiting_verification`、`recovering` 或 `recovered`，以及该分支当前最新 commit。

## 权限和安全边界

- `pull_request_target` 和 `workflow_run` 都只 checkout 默认分支。
- checkout action 固定 commit SHA，且 `persist-credentials: false`。
- 共享 Action 固定完整 commit SHA，目标仓配置限制在 `GITHUB_WORKSPACE` 内并在任何 API 写入前校验。
- App token action 固定完整 commit SHA；短期 token 只授予当前仓库，job 结束时自动吊销，App 私钥只存在于组织 secret。
- App installation token 只请求 Actions 读、Checks 写、Contents 读、Issues 写和 Pull requests 写，不授予 administration、workflow、secret、member、deployment 或 contents 写权限。
- 标题、正文、文件路径、日志、job 名称及外部链接均按不可信输入处理，不拼接为 shell 命令。
- 模型分析不参与合并门禁、产品优先级、Issue 自动关闭或发布决策。

## master 分支保护（建议）

AsterDrive 的 `master governance` ruleset 对 `refs/heads/master` 启用：PR 必须、至少一个 approval、所有 review conversation 已解决、`PR Readiness` 由 Automation App 成功发布、线性历史、admin 保留 always bypass。AsterYggdrasil 尚未启用同等 ruleset；启用时建议照搬 Drive 的配置（见 `AsterDrive` 仓库 ruleset 设置）。Ruleset 是强制执行层；automation 不自行合并、批准、dismiss review 或修改 ruleset。
