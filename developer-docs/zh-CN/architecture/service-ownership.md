# 服务模块所有权

每个服务模块承载一个产品域。Handler 只做认证、参数提取、调用 service、返回响应；repository 在 `src/db/repository/`；DTO 在 `src/api/dto/`。服务文件 facade 优先：壳文件（`auth_service.rs` 或 `auth_service/mod.rs`）声明子模块并 re-export 公开面，内部重构时 `crate::services::auth_service::login` 这类路径保持稳定。

## 服务模块

| 模块 | 域 | 子模块 |
| --- | --- | --- |
| `services/auth_service/` | 站点账号：注册、初始化、登录/登出、令牌轮换与验证、会话、联系方式验证、密码修改、token scope 规则 | `shared` `session` `login` `registration` `tokens` `contact_verification` `password_change` `token_scope` |
| `services/passkey_service.rs` | Passkey/WebAuthn 注册与认证 | 单文件 |
| `services/user_invitation_service.rs` | 邀请创建、撤销、接受 | 单文件 |
| `services/profile_service/` | 站点用户资料与头像 | `avatar*` `info` `profile` `shared` |
| `services/yggdrasil_service/` | Yggdrasil 协议：authenticate/refresh/invalidate/signout、join/hasJoined、profile 查询、metadata、材质属性签名 | `auth` `session` `token` `login` `profile` `properties` `metadata` `cache` `error` `minecraft_services` |
| `services/texture_service/` | 衣柜材质生命周期：上传、处理、绑定、删除、预览、维护 | `shared` `library` `reports` `moderation` `wardrobe` `deletion` `processing` `query` `preview` `cache` `types` `error` |
| `services/mail/` | 邮件 outbox 派发、模板、发送、邮件审计 | `outbox` `outbox_runtime` `audit` `sender` `template` |
| `services/ban_service.rs` | 玩家能力封禁：列表/创建/更新/撤销、执行查询 | 单文件 |
| `services/config_service/` | 静态配置轨道之上的运行时配置服务层：读写删、config action、公开配置、schema | `actions` `public` `runtime` `schema` `system` |
| `services/admin_user_service.rs` | 管理员用户 CRUD、临时密码、scope 归一化 | 单文件 |
| `services/admin_overview_service.rs` | 管理员总览聚合与系统健康摘要 | 单文件 |
| `services/external_auth_service/` | 外部认证 provider、OAuth/OIDC 流程、身份绑定、密码链接 | `login` `resolution` `providers` `links` `verification` `password_link` `normalize` |
| `services/captcha_service.rs` | 图形验证码生成与校验 | 单文件 |
| `services/health_service.rs` | readiness 与 diagnostics 健康范围 | 单文件 |
| `services/yggdrasil_session_forward_service.rs` | session-forward 服务器注册与上游检查 | 单文件 |
| `services/yggdrasil_signature.rs` | Yggdrasil 签名密钥生成与轮换 | 单文件 |
| `services/texture_preview.rs` | 渲染 crate 之上的预览路由胶水 | 单文件 |

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
