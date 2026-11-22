# 架构总览

AsterYggdrasil 构建在 [AsterForge](https://github.com/AsterCommunity/AsterForge) 运行时之上，采用 AsterDrive 的产品布局：薄应用壳承载 API 和业务服务，产品 crate 承载模型、迁移和渲染。

## Workspace Crates

| Crate | 职责 | 依赖 |
| --- | --- | --- |
| `aster_yggdrasil` | 应用壳：API 路由、服务、运行时装配、静态配置加载、管理中间件 | 全部产品 crate 和全部 Forge crate |
| `aster_yggdrasil_protocol` | 与项目无关的 Yggdrasil/authlib-injector 协议 wire 模型和错误状态机（档案、属性、请求/响应体、metadata、Minecraft services 兼容载荷、base64 `textures` 值编码） | `serde`、`serde_json`、`base64`、`http`、`validator`；可选 `utoipa` |
| `aster_yggdrasil_outbound` | 出站 HTTP 策略：用途 profile、超时、遥测 observer——出站 client 的唯一构建点 | `aster_forge_http`、`reqwest`、`tracing` |
| `aster_yggdrasil_model` | SeaORM 实体与领域类型（`UserRole`、`OperatorScope`、`UserBanScope`、任务 payload）。实体所属的 Forge trait 实现（`TaskRecord`、`ClaimableTaskRecord`）也在这里 | Forge crate、SeaORM。绝不依赖应用 crate |
| `aster_yggdrasil_migration` | 只追加的数据库迁移和独立迁移二进制 | Forge DB migration helper |
| `aster_yggdrasil_render` | Minecraft 皮肤/披风预览渲染（2D 和 3D 引擎） | 仅 `image` |

依赖方向单向：产品 crate 不依赖应用壳。服务代码通过 `aster_yggdrasil_model::` 全限定路径引用实体——没有 re-export 薄层。

## 运行时模型

进程运行在 `aster_forge_runtime::AsterRuntime::builder()` 上。组件显式声明依赖和关停顺序（`src/runtime/assembly.rs`）：

```
background_tasks -> mail_outbox -> audit_manager -> audit_logs -> database
```

- `src/runtime/assembly.rs` 装配组件图并运行。
- `src/runtime/shutdown.rs` 是关停收尾：`run()` 返回后 metrics provider 排空；Forge#74 的 audit-recovery 钩子上游落地后挂在这里。
- `src/runtime/bootstrap.rs` 安装 panic hook、初始化 JWT crypto provider、加载静态配置、准备运行时状态。
- `src/runtime/tasks.rs` 注册后台任务消费者和定时运行时任务。
- `src/runtime/test_support.rs` 提供共享测试 fixture（测试里不要手拼 `AppStateParts`）。

## 配置：三轨模型

AsterYggdrasil 与 AsterDrive 使用同一套三轨配置模型，Forge 拥有共享部分：

| 轨道 | 归属 | 位置 |
| --- | --- | --- |
| 静态 TOML 快照 | 产品（`src/config/`，OnceLock + loader） | `config.toml`，schema 在 `src/config/schema.rs` |
| 可热更运行时配置 | `aster_forge_config::SyncRuntimeConfig`，由 `RuntimeConfig` 包装 | `system_config` DB 表 |
| system-config 定义注册表 | 产品定义（`src/config/definitions.rs`），由 Forge 校验归一化 | `ensure_defaults` 播种 |

Forge 直接拥有共享段的 schema 类型（`aster_forge_cache::CacheConfig`、`aster_forge_config::ConfigSyncConfig`、`aster_forge_logging::LoggingConfig`）；产品段（`server`、`auth`、`texture_storage`、`rate_limit`、`network_trust`、`deployment`）是产品 schema——这是刻意边界。

`RuntimeConfig` 还承载进程运行时：`AuthRuntimes`（Forge `JwtSigner`/`JwtVerifier` 加有上限的 `PasswordHashRuntime` 和 dummy-hash 凭据）以及 config-sync 运行时。TTL key 热更；JWT secret 仅启动期。

## 响应信封

序列化通过 transparent adapter（`src/api/response.rs`）委托给 Forge 的校验信封：`AsterErrorCode` 实现 `aster_forge_api::response::ApiResponseCode`，包装器把产品 code 注入 `ForgeApiResponse<T, AsterErrorCode>`。构造器在编译期拒绝状态错配（成功码不能带 error 元数据、失败码不能带 data）。

wire 形状：

```json
{ "code": "success", "msg": "", "data": { ... } }
{ "code": "auth.credentials_failed", "msg": "...", "error": { "retryable": false } }
```

失败不携带 retryability 元数据时省略 `error`；信封迁到 Forge 时移除了 error 内层冗余的 `code` 字段。顶层 `code` 是前端 `errors.json` 映射消费的稳定公开契约。`src/api/response.rs` 的 wire-byte 契约测试锁定这些形状。

## 认证模型

身份链、角色、operator 区域授权和玩家能力封禁见 [auth-model.md](auth-model.md)。

## 相关文档

- [服务模块所有权](service-ownership.md)
- [配置模型](config-model.md)
- [认证模型](auth-model.md)
