# 用户能力封禁实现说明

能力封禁是用户级、scope 级的业务限制。它限制某个用户能否使用 Yggdrasil、Minecraft profile、材质上传和公共材质库交互能力，不替代账号状态、operator scope 或 Minecraft profile 生命周期。

用户侧使用文档见 `docs/guide/user-bans.md`。本文只说明代码边界和扩展契约。

## 领域模型

核心类型在 `src/types/user.rs`：

- `UserBanScope`
- `UserBanStatus`
- `UserBanEventType`
- `UserBanScopes`

`UserBanScope` 当前包含：

| Scope | 代码语义 |
| --- | --- |
| `yggdrasil_access` | Yggdrasil authenticate 和 token 使用 |
| `yggdrasil_join` | join/hasJoined 和 Minecraft services multiplayer 状态 |
| `minecraft_profile_manage` | profile 创建、删除、改名，以及 profile 材质绑定改写 |
| `texture_upload` | wardrobe 上传、Yggdrasil 直接上传、材质元数据更新、profile 材质绑定/删除 |
| `texture_library_interact` | 用户侧公共材质库提交、撤回、举报等交互动作 |

不要把 scope 当裸 `String` 在 service/repository 之间传递。新增 scope 时先扩展 `UserBanScope`，再补服务检查点、前端文案、OpenAPI、测试和本文档。

## 存储

迁移在 `migration/src/m20260620_000002_user_bans.rs`，实体在：

- `src/entities/user_ban.rs`
- `src/entities/user_ban_event.rs`

表：

- `user_bans`
- `user_ban_events`

`user_bans.scopes` 和事件表的 `previous_scopes` / `next_scopes` 是 TEXT，内容是 JSON 数组，例如：

```json
["texture_upload","minecraft_profile_manage"]
```

代码层必须通过 `UserBanScopes` wrapper 读写。`UserBanScopes::new(Vec<UserBanScope>)` 会排序、去重并拒绝空数组。数据库允许 TEXT 存储 JSON 数组，但业务层不允许空 scope 集合，也不允许绕过 wrapper 写裸字符串。

## Service 边界

业务逻辑集中在 `src/services/ban_service.rs`。

主要输入和输出：

- `CreateUserBanInput.scopes: Vec<UserBanScope>`
- `UpdateUserBanInput.scopes: Option<Vec<UserBanScope>>`
- `ListUserBansInput.scope: Option<UserBanScope>`
- `UserBanInfo.scopes: Vec<UserBanScope>`
- `UserBanEventInfo.previous_scopes / next_scopes`

列表筛选仍使用单个 `scope` query 参数，语义是“返回 scopes 数组中包含这个 scope 的记录”。创建、更新和响应使用 `scopes` 数组。不要恢复旧的单数 `scope` 请求字段。

生效判断：

- `status == active`
- `revoked_at IS NULL`
- `starts_at <= now`
- `expires_at IS NULL OR expires_at > now`

同一用户不能同时存在覆盖同一 scope 的有效封禁。`reject_duplicate_effective_scopes` 会逐个 scope 检查当前有效记录，并在 update 时排除当前 ban id。

## API

管理员 API 在 `src/api/routes/admin/user_bans.rs`：

```text
GET    /api/v1/admin/user-bans
GET    /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/users/{user_id}/bans
PATCH  /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/user-bans/{ban_id}/revoke
GET    /api/v1/admin/user-bans/{ban_id}/events
```

当前用户自查 API 在 `src/api/routes/account.rs`：

```text
GET /api/v1/account/bans
```

DTO 在：

- `src/api/dto/admin.rs`
- `src/api/dto/account.rs`

创建和更新 DTO 使用 `#[serde(deny_unknown_fields)]`。旧的单数 `scope` 字段应该被拒绝。`scopes: []` 应该返回校验错误。

管理员响应包含内部字段，例如 `admin_note`、`created_by_user_id`、`revoked_by_user_id`、`revoke_note`。用户自查响应通过 `AccountUserBanInfo` 去掉这些内部字段。

## 接入点

新增受限能力时优先使用：

```rust
ban_service::ensure_user_not_banned(state, user_id, UserBanScope::...)
```

需要协议兼容错误体时使用 `is_user_banned` 并映射到协议错误。

当前关键检查点：

- Yggdrasil authenticate 和 token validate/refresh：`yggdrasil_access`
- Yggdrasil join、hasJoined、Minecraft services privileges/player attributes：`yggdrasil_join`
- Minecraft profile create/delete/rename：`minecraft_profile_manage`
- Wardrobe upload 和 Yggdrasil texture upload：`texture_upload`
- Profile texture bind/delete：`texture_upload` + `minecraft_profile_manage`
- 公共材质库用户交互：`texture_library_interact`

上传入口必须在读取 multipart 文件前检查 `texture_upload`。服务层仍要保留第二道检查，防止其他调用路径绕过 HTTP handler。

## 错误映射

项目 API 被封禁拦截时返回：

```text
403 user_ban.forbidden
```

稳定错误码定义在 `src/api/error_code.rs`：

- `user_ban.not_found`
- `user_ban.already_active`
- `user_ban.not_active`
- `user_ban.duration_invalid`
- `user_ban.reason_invalid`
- `user_ban.forbidden`

Texture service 有自己的 `TextureErrorKind::UserBanForbidden`。项目 API route 需要把它映射回 `AsterErrorCode::UserBanForbidden`，不要退化成 `minecraft_texture.upload_disabled`。

Yggdrasil 协议端点必须保持协议错误形状，不包项目 envelope。封禁错误在协议面通常映射成 invalid token/credentials 或 `ForbiddenOperationException`，具体由 `src/services/yggdrasil_service/*` 和 `src/services/texture_service/error.rs` 负责。

## Audit 和事件

管理操作会写 audit：

- `AdminCreateUserBan`
- `AdminUpdateUserBan`
- `AdminRevokeUserBan`

Audit entity type 是 `UserBan`，details 包含 `scopes` 数组、目标用户、状态、原因、备注和时间范围。前端 audit presentation 应展示 `scopes`，不要只读旧的 `scope`。

`user_ban_events` 保存每次创建、更新和撤销的状态变化。scope 变化使用 `previous_scopes` 和 `next_scopes`。

## 前端契约

API 类型来自 OpenAPI 生成文件。后端 DTO 变化后需要运行：

```bash
cargo nextest run --profile ci --features openapi --test generate_openapi
cd frontend-panel && bun run generate-api
```

管理端用户详情的能力封禁区应使用 `scopes` 数组创建/更新记录。用户账户概览默认只展示当前有效封禁；已撤销和已过期记录不要进入默认用户面板。

公共材质库浏览不应因为 `texture_library_interact` 被隐藏或阻断。

## 测试要求

相关测试集中在：

- `tests/test_user_bans.rs`
- `tests/test_yggdrasil.rs`
- `frontend-panel/src/components/admin/admin-users-page/UserDetailBanSection.test.tsx`
- `frontend-panel/src/pages/account/AccountOverviewPage.test.tsx`
- `frontend-panel/src/services/apiServices.test.ts`

改动时至少覆盖：

- 创建、更新、撤销、事件和 audit。
- `scopes` 非空、排序去重、拒绝旧 `scope` 字段。
- 一条记录包含多个 scope 时，每个 scope 都能命中封禁。
- 覆盖任一有效 scope 的重复封禁会被拒绝。
- 用户自查不泄漏管理员内部字段。
- 材质上传封禁在读取文件和创建记录前失败。
- 项目 API 返回 `user_ban.forbidden`，Yggdrasil 协议 API 保持协议错误体。

推荐验证命令：

```bash
cargo nextest run --profile ci --test test_user_bans
cargo nextest run --profile ci --test test_yggdrasil user_ban_texture_upload_scope_blocks_wardrobe_upload_before_temp_or_record_write
cd frontend-panel && bun run test -- src/components/admin/admin-users-page/UserDetailBanSection.test.tsx src/pages/account/AccountOverviewPage.test.tsx src/services/apiServices.test.ts
cd frontend-panel && bun run check
```
