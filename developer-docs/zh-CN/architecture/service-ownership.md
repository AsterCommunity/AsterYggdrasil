# 服务模块所有权

服务层按**领域平面**组织（对齐 AsterDrive 的 `auth/` / `user/` / `ops/` 结构）：模块名不带 `_service` 后缀，`services/mod.rs` 只声明顶层域。每个域的 `mod.rs` 声明子模块并 re-export 公开面；Handler 只做认证、参数提取、调用 service、返回响应；repository 在 `src/db/repository/`；DTO 在 `src/api/dto/`。

## 领域平面

```text
services/
├── auth/       认证面：local（站点账号）、external（外部登录）、passkey、captcha
├── user/       用户域：profile（资料/头像）、admin（管理员用户管理）、ban、invitation
├── ops/        运营面：admin（总览）、audit、config、health、task
├── texture/    材质域
├── yggdrasil/  协议域：protocol（协议流程）、session_forward、signature
└── mail/       邮件域
```

## 服务模块

| 模块 | 域 | 子模块 |
| --- | --- | --- |
| `services/auth/local/` | 站点账号：注册、初始化、登录/登出、令牌轮换与验证、会话、联系方式验证、密码修改、token scope 规则 | `shared` `session` `login` `registration` `tokens` `contact_verification` `password_change` `token_scope` |
| `services/auth/external/` | 外部认证 provider、OAuth/OIDC 流程、身份绑定、密码链接 | `login` `resolution` `providers` `links` `verification` `password_link` `normalize` |
| `services/auth/passkey.rs` | Passkey/WebAuthn 注册与认证 | 单文件 |
| `services/auth/captcha.rs` | 图形验证码生成与校验 | 单文件 |
| `services/user/profile/` | 站点用户资料与头像 | `avatar*` `info` `details` `shared` |
| `services/user/admin.rs` | 管理员用户 CRUD、临时密码、scope 归一化 | 单文件 |
| `services/user/ban.rs` | 玩家能力封禁：列表/创建/更新/撤销、执行查询 | 单文件 |
| `services/user/invitation.rs` | 邀请创建、撤销、接受 | 单文件 |
| `services/yggdrasil/protocol/` | Yggdrasil 协议：authenticate/refresh/invalidate/signout、join/hasJoined、profile 查询、metadata、材质属性签名 | `auth` `session` `token` `login` `profile` `properties` `metadata` `cache` `error` `minecraft_services` |
| `services/yggdrasil/session_forward.rs` | session-forward 服务器注册与上游检查 | 单文件 |
| `services/yggdrasil/signature.rs` | Yggdrasil 签名密钥生成与轮换 | 单文件 |
| `services/texture/` | 衣柜材质生命周期：上传、处理、绑定、删除、预览、维护 | `shared` `library` `reports` `moderation` `wardrobe` `deletion` `processing` `query` `preview` `cache` `types` `error` |
| `services/mail/` | 邮件 outbox 派发、模板、发送、邮件审计 | `outbox` `outbox_runtime` `audit` `sender` `template` |
| `services/ops/admin/overview.rs` | 管理员总览聚合与系统健康摘要 | 单文件 |
| `services/ops/audit/` | 审计输入、上下文、查询与展示 | `context` `details` `filters` `manager` `models` `presentation` `query` `runtime` |
| `services/ops/config/` | 静态配置轨道之上的运行时配置服务层：读写删、config action、公开配置、schema | `actions` `public` `runtime` `schema` `system` |
| `services/ops/health/` | readiness 与 diagnostics 健康范围 | 单文件 |
| `services/ops/task/` | 后台任务创建、派发、lease、重试、展示 | `dispatch` `spec` 等 |

## State Traits

服务函数接收单个能力 trait，而不是平铺的 bound 列表：

```rust
pub async fn login<S>(state: &impl SharedRuntimeState, ...) -> Result<AuthTokenBundle>
```

能力 trait 通过 `src/runtime/state_traits.rs` 的 supertrait 组合：

- 基础：`DatabaseRuntimeState`、`AppConfigRuntimeState`、`RuntimeConfigRuntimeState`、`CacheRuntimeState`、`TextureStorageRuntimeState`（texture store）、`MetricsRuntimeState`、`AuditRuntimeState`
- `SharedRuntimeState` = 以上全部
- `MailRuntimeState: SharedRuntimeState` 增加 `mail_sender()`
- `TaskRuntimeState: SharedRuntimeState` 增加 `wake_background_task_dispatcher()`
- `YggdrasilSessionForwardRuntimeState: SharedRuntimeState` 增加 forward HTTP client

只有确实只需两个能力的纯查询 helper 保留窄 bound。具体 `AppState` 实现全部能力；测试用 `runtime::test_support::setup_state`。

## Forge 委托

服务**不拥有**的部分（委托给 Forge crate）：

- JWT 签发/验证和密码哈希 —— `aster_forge_auth`（`RuntimeConfig` 里的 `AuthRuntimes`）
- Audit 缓冲与排空 —— `aster_forge_audit::AuditWriter`（`AuditRuntimeState`）
- Token/session claims 模型 —— Forge `TokenClaims`，由产品 `AuthenticatedClaims` 包装
- 邮件 outbox 状态机 —— `aster_forge_db::MailOutboxDbStore` + `aster_forge_mail`
- 限流与可信代理 IP 解析 —— `aster_forge_middleware`
- metrics provider 与基础设施记录器 —— `aster_forge_metrics`
- 后台任务 claim/lease/retry 规则 —— `aster_forge_tasks`
- 出站 HTTP —— `aster_yggdrasil_outbound`（`Profile` + `ClientConfig` 唯一构建点）
- 出站 HTTP —— `aster_yggdrasil_outbound`（`Profile` + `ClientConfig` 唯一构建点）
