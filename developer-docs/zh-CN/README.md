# AsterYggdrasil 开发者指南

AsterYggdrasil 是自建 Minecraft 皮肤站和 Yggdrasil 认证服务器，不再是通用 Rust + React 模板。当前代码已经包含账号系统、外部认证、passkey、用户档案、Minecraft profile、衣柜材质、authlib-injector 兼容接口、运行时配置、后台任务、审计和管理后台。

这些文档描述当前实现和扩展约定。新增内容要围绕 Minecraft/Yggdrasil 业务建模，不要把旧模板里的云盘、文件分享、团队协作或通用 starter 叙事带回来。

## 核心原则

- 先读现有 service、repository、DTO、OpenAPI、前端 service/page pattern，再动手。
- 产品域命名要直接表达 `yggdrasil`、`minecraft_profile`、`texture`、`skin`、`cape`、`wardrobe`、`authlib_injector` 等语义。
- 项目后台和站点自身 API 使用统一 response envelope；Yggdrasil 协议端点必须保持协议原生响应格式。
- 新公开错误原因时扩展 `AsterErrorCode`，不要新增第二套客户端可见 subcode。
- 管理员操作、安全敏感变更、Minecraft profile/texture 变更、Yggdrasil token/session 行为和运行时生命周期事件要接 audit。
- API contract 改动必须同步 OpenAPI schema 和前端生成类型。
- 已发布后的 migration 只追加，不改已经应用过的迁移。
- `frontend-panel/` 已经是产品前端，不要再按模板/demo 的信息架构扩展。

## 当前产品域

后端已经落地这些主要域：

- 本地账号、注册、登录、refresh/logout、会话管理和用户资料。
- Passkey/WebAuthn 登录与凭据管理。
- 外部认证 provider、OAuth/OIDC 登录、账号绑定和邮箱验证流程。
- Minecraft profile 创建、查询、删除和材质绑定。
- Wardrobe 材质库，支持皮肤/披风上传、校验、存储、绑定和删除。
- 公共材质库，支持 wardrobe 材质提交、审核、发布、下架、标签和登录用户举报。
- Yggdrasil/authlib-injector 协议接口，包括 metadata、authenticate、refresh、validate、invalidate、signout、join、hasJoined、profile、textures。
- 管理后台 API，包括配置、用户、Minecraft profile、审计、外部认证 provider、后台任务和系统信息。
- 运行时配置、邮件 outbox、审计、后台任务、metrics、CORS/CSRF、安全头和限流。

前端已经落地这些主要页面：

- `/` 公共接入页和 authlib-injector 连接信息。
- `/init` 初始化首个管理员。
- `/login`、`/register` 账号入口，包含外部认证和 passkey 登录。
- `/reset-password`、`/invite/:token` 密码重置和邀请注册入口。
- `/force-password-change` 强制改密入口。
- `/account` 账号工作台。
- `/account/profiles` Minecraft profile 与 launcher/材质操作。
- `/account/wardrobe` 当前用户材质库、公共库提交状态和审核/下架意见。
- `/account/audit` 当前用户审计记录。
- `/account/settings` 个人设置、会话和 passkey；`/settings/security` 兼容旧安全设置入口。
- `/admin/*` 管理配置、用户、用户邀请、Minecraft profile、公共材质库、外部认证、审计、任务和关于页。
- `/tos`、`/privacy` 法务页面。

## 后端扩展路径

新增后端能力时按这个形状走：

```text
src/entities/                  SeaORM model
migration/                     schema migration
src/db/repository/             数据库访问
src/services/                  业务行为
src/api/dto/                   request/response DTO
src/api/routes/                HTTP handler 和路由注册
src/api/openapi.rs             OpenAPI path 和 schema
tests/                         集成测试
frontend-panel/src/services/   前端 service wrapper
frontend-panel/src/pages/      需要时增加页面
```

对象文件存储走 `src/object_storage/`，不要复活旧云盘文件模型。Yggdrasil 业务逻辑集中在 `src/services/yggdrasil_service/`，材质业务逻辑集中在 `src/services/texture_service/`，handler 只做认证、参数提取、调用 service、返回响应。

## Runtime Startup

启动拆在 `src/runtime/startup/`：

- `common.rs` 准备运行时目录、metrics、数据库句柄、migration、runtime config、cache、audit manager。
- `state.rs` 构建 `AppState` 需要的产品运行时资源。
- `mod.rs` 串行执行 startup phase，并记录 `server_start`。

进程入口在 `src/runtime/entrypoint.rs`，实际装配在 `src/runtime/assembly.rs`：

1. bootstrap 加载静态配置、日志、panic hook 和 startup state。
2. assembly 创建 shutdown token、Actix HTTP service component。
3. `src/runtime/components.rs` 构建产品 runtime bundle，并注册后台任务、mail outbox、audit 和 database component。
4. `AsterRuntime::builder().component(http).component(product_components).run().await` 运行服务，等待退出信号并按 component dependency graph 关闭。

当前使用 `[deployment].profile = "single" | "cluster"` 选择部署策略，不维护 primary/follower 启动分支。`cluster` 要求 PostgreSQL/MySQL、Redis cache、Redis config-sync 和 S3/MinIO；Forge runtime 的 lease/claim 负责协调多实例后台任务，配置同步断线重连后会从 writer DB 全量 reconcile。

## 优雅退出

关闭流程由 `aster_forge_runtime::AsterRuntime` 和 Yggdrasil product component graph 协调：

1. Forge 等待 SIGINT/SIGTERM/Ctrl+C，并调用 HTTP service stop hook。
2. Actix 停止后，Forge 按 component dependency graph 运行 shutdown phase。
3. `background_tasks` 先取消并等待后台任务。
4. `mail_outbox` drain 可发送的邮件。
5. `audit_logs` 记录 `server_shutdown`。
6. `audit_manager` flush 全局 audit buffer。
7. `database` 关闭 writer/reader 句柄。

新增长跑 worker 时，必须监听 shutdown token，并保证持久化状态可恢复。新增有生命周期副作用的子系统时，优先在自己的模块暴露 component 构造函数，再由 `src/runtime/components.rs` 统一组合，不要把 shutdown 顺序散落到入口。

audit runtime component、`audit_logs` 共享 schema、基础写入、批量写入、通用 cursor 查询、统计和删除 helper 来自 AsterForge。Yggdrasil 只保留 typed `AuditAction`、detail schema、权限、presentation 和业务统计口径。

## 数据库 helper 与事务

数据库代码仍然放在 `src/db/`，但共享机械逻辑应该接 AsterForge，不要在产品仓库继续维护本地重复实现。

- Repository 搜索 helper 使用 `aster_forge_db::search_query`。
- 事务 begin、commit、rollback、tracing 和 rollback guard 行为由 `aster_forge_db::transaction` 提供。
- `src/db/transaction.rs` 只保留产品本地路径，默认把事务边界错误映射成 `AsterError`。
- 传给 `with_transaction` 的 service 回调应该返回 `AsterError` 或子系统错误类型，不要返回 `DbError`。
- `From<aster_forge_db::DbError> for AsterError` 是 Forge 数据库基础设施失败进入产品错误系统的边界。
- 只有协议或子系统需要跨回调保留自己的错误类型时，才使用显式的子系统错误事务入口。

不要因为校验、权限、协议或业务状态失败发生在事务里，就把它们包装成数据库错误。`with_transaction` 会保留回调错误，只把 begin/commit/rollback 失败映射进产品错误类型。

## 后台任务

任务系统在 `src/services/task_service/` 和 `src/tasks/runtime.rs`。

持久化的 `BackgroundTaskKind` 当前仍只有 `system_runtime`。具体系统任务通过 `SystemRuntimeTaskKind` 区分，当前包括：

- `background-task-dispatch`
- `system-health-check`
- `auth-session-cleanup`
- `external-auth-flow-cleanup`
- `mail-outbox-dispatch`
- `audit-cleanup`
- `task-cleanup`
- `yggdrasil-token-cleanup`
- `yggdrasil-storage-consistency-check`
- `yggdrasil-texture-cleanup`

Admin API 可以 list、retry、cleanup；普通用户任务 API 当前没有。新增业务 task kind 时，payload/result 类型、registry、retry 分类、初始 steps、presentation、可见性规则和测试要一起设计。

关键契约：

- Claim 带 `processing_token` fence。
- Worker 通过 heartbeat 续租。
- 过期 worker 不能覆盖新 lease。
- 优雅退出会把 processing task 放回 `retry`，不消耗 retry budget，也不写业务失败细节。
- Task presentation 使用稳定 message code，前端不解析 task payload 或 result blob。
- 修改 dispatch 语义时，要补 claim、retry、cleanup、shutdown 行为测试。

邮件 outbox 投递也是系统运行时任务，具体扩展规则见 [邮件运行时扩展](./mail-runtime.md)。

Yggdrasil/authlib-injector 协议端点、鉴权、错误形状、Minecraft services 兼容层和测试要求见 [Yggdrasil API 实现说明](./yggdrasil-api.md)。

用户能力封禁的 scope 模型、存储 wrapper、API、错误映射和测试要求见 [用户能力封禁实现说明](./user-bans.md)。

## Audit Service

Audit 代码在 `src/services/audit_service/`，稳定枚举在 `src/types/audit.rs`。

这些场景要写 audit：

- server start 和 shutdown
- setup、register、login、logout、refresh token、session revoke
- passkey register、rename、delete、login
- admin config 变更和 config action
- admin user 变更和 session revoke
- admin external auth provider 创建、更新、删除、测试
- external auth 登录、绑定、解绑
- admin task retry 和 cleanup
- admin user capability ban 创建、更新和撤销
- mail send 和 mail delivery failure
- Minecraft profile 创建和删除
- Minecraft texture 上传、绑定和删除
- 公共材质库提交、撤回、审核通过、打回、下架和举报处理
- Yggdrasil authenticate、refresh、invalidate、signout、join server

Audit entry 应包含结构化 details 和 presentation metadata。前端优先展示 `presentation`，raw `details` 只作为 fallback/debug 信息。

邮件 audit 的 details、presentation 和测试要求见 [邮件运行时扩展](./mail-runtime.md)。

## API 与错误

项目 API 响应使用 `src/api/response.rs` 里的统一 envelope：

```json
{ "code": "success", "msg": "", "data": {} }
```

客户端可见失败暴露稳定的 `AsterErrorCode`。错误码已经覆盖 auth、external auth、mail、config、audit log、task、Minecraft profile、Minecraft texture、wardrobe、passkey、avatar 和 frontend config。

Yggdrasil/authlib-injector 协议端点是例外：它们按协议返回状态码、字段和错误体，不包项目 envelope。协议错误映射留在 `src/services/yggdrasil_service/error.rs` 和 `src/api/routes/yggdrasil.rs` 周边，不要污染全局错误系统。

API contract 改动流程：

1. 更新 DTO 和路由注解。
2. 在 `src/api/openapi.rs` 注册 path 和 schema。
3. 生成 OpenAPI。
4. 重新生成前端 API 类型。
5. 更新前端 service/page。

命令：

```bash
cargo test --features openapi --test generate_openapi
cd frontend-panel
bun run generate-api
```

## 前端扩展路径

前端在 `frontend-panel/`，当前不是营销站，也不是模板 demo。

约定：

- `src/services/` 放 API wrapper。
- `src/types/api.generated.ts` 是生成类型，`src/types/api.ts` 做统一 re-export 和别名。
- `src/lib/presentation.ts` 放稳定 audit/task 展示格式化。
- `src/pages/account/` 放登录后账号页面。
- `src/pages/admin/` 放管理员页面。
- `src/components/yggdrasil/` 放 launcher、Minecraft preview、复制字段等 Yggdrasil/Minecraft 组件。
- `src/components/account/` 放账号域页面组合件。
- `src/components/admin/`、`src/components/common/`、`src/components/layout/` 放后台和通用 UI 组合件。

Admin screen 应该保持信息密度适中、可扫描、可重复操作。Profile/wardrobe 页面要围绕真实 Minecraft 工作流，不要回到模板式 feature card 或云盘式文件管理。

## 测试

常用命令：

```bash
cargo fmt
cargo check
cargo test
cargo test --features openapi --test generate_openapi

cd frontend-panel
bun run check
bun run test
bun run build
```

常用目标测试：

```bash
cargo test --test test_yggdrasil
cargo test --test test_admin_tasks
cargo test --test test_audit
cargo test --test test_auth
cargo test --test test_external_auth
cargo test --test test_database_backends
cargo test mail_template
cargo test texture_service
cargo test task_service::presentation
cargo test shutdown_release_returns_processing_task_to_retry_without_failure_update
```

改 migration、repository 或 SQL 时至少跑 SQLite 相关测试；涉及跨库语义时再跑 `ASTER_TEST_DATABASE_BACKEND=postgres|mysql cargo test --test test_database_backends`。改前端 service 或关键页面流程时跑 `bun run test`，涉及页面交互再跑对应 Playwright。

## 产品边界检查清单

往 AsterYggdrasil 加模块前先问：

- 这个能力是不是 Minecraft 皮肤站、Yggdrasil 认证、账号安全、运行时运维或管理后台需要的？
- 命名是否直接表达产品域，而不是旧模板/云盘/团队协作概念？
- 协议端点是否保持 authlib-injector/Yggdrasil 兼容格式？
- 项目 API 是否继续使用 envelope 和稳定 `AsterErrorCode`？
- 安全敏感数据是否避免明文落库、日志、审计和错误消息？
- 测试是否覆盖对应 service/repository/API 风险？
- 前端拿到的是稳定 presentation 或 DTO，而不是被迫解析后端内部结构？

如果答案是否定的，先停下确认需求，别把无关能力塞进这个产品。代码不测就是不负责，文档不对齐也一样。
