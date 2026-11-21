# 认证模型

## 身份链

每个请求按以下链路解析：

1. **JWT access token**（Forge `aster_forge_auth`）：claims 携带 `sub`（字符串化的 `users.id`）、`sid`（会话 id）、`session_version`（代际围栏）、`token_type`（access/refresh）、`restriction`（密码修改门）和 `jti`（仅 refresh）。
2. **`AuthenticatedClaims`**（`src/services/auth_service/tokens.rs`）：Forge claims 的产品视图——数值 `user_id`、`session_version`、`jti`、`password_change`、`token_type`。**角色和 operator scope 不进 token**；每请求从数据库现查，因此 scope 和 role 变更立即生效。
3. **数据库查找**：按 `sub` 查 `users.id`；`session_version` 与 `users.session_version` 比对；`restriction` 必须与 `users.must_change_password` 完全一致。

Refresh 轮换用 refresh `jti` 对照 `auth_sessions.current_refresh_jti`，记录前一个 jti（重放检测）并 bump `last_seen`。`session_version` bump 会使所有未决 token 失效（密码修改、管理员禁用、管理员 scope 变更）。

## 角色

`UserRole`（SeaORM active enum，`users.role`）：`Admin`、`Operator`、`User`。

- `Admin` —— 全部后台访问；受保护，其他管理员不可修改。
- `Operator` —— 后台访问限于被授予区域。
- `User` —— 玩家；仅 `/api/v1/account/*` 和 Yggdrasil 协议面。

## Operator 区域授权

Operator scope 授权以命名空间标识符存于 `user_operator_scopes`（每授权一行）：

```
com.astercosm.yggdrasil.permission.admin.{overview|users|profiles|texture_library|audit|tasks|settings|external_auth}
```

强制点是每个 admin 路由组上的 `RequireAdminOrScope`（`src/api/middleware/admin.rs`）：

```rust
user.role.is_admin()
    || (user.role == UserRole::Operator && user.operator_scopes.contains(&scope))
```

计划中的迁移（issue #13）：授权从裸区域名迁移为上面的完整命名空间 id，由 `PermissionRegistry` 承载（descriptor 优先，参照 AsterACME 的 `ProviderRegistry`），构造期校验 id，管理端从 descriptor 渲染未知条目。role/scope 变更 bump `session_version`，未决 token 立即失效。

## 玩家能力封禁

`UserBanScope` 值是玩家能力标识符，存于带可选过期的 `user_ban` 行：

```
com.astercosm.yggdrasil.capability.yggdrasil.{access|join}
com.astercosm.yggdrasil.capability.profile.manage
com.astercosm.yggdrasil.capability.texture.upload
com.astercosm.yggdrasil.capability.texture_library.interact
```

`ban_service` 在能力使用点（Yggdrasil 端点、材质上传、档案管理、材质库）执行 deny 规则。封禁作用于玩家；后台访问由上面的授权管理——两个词汇表（`permission.admin.*` vs `capability.*`）从不相交。

## 密码处理

哈希与验证经 `aster_forge_auth::PasswordHashRuntime`（有上限的 argon2 worker pool），宿主在 `RuntimeConfig::AuthRuntimes`。登录遇到未知标识符时燃烧 runtime 的 dummy hash，使响应时序不暴露账号是否存在。`must_change_password` 映射为 token 上的 Forge `Restriction::MustChangePassword`，并在 `current_user` 中逐请求再校验。

## 协议凭据

Yggdrasil token 是独立凭据族（`yggdrasil_token` 表、sha256 哈希的 access token、`selected_profile` 绑定）。它与站点 JWT 会话相互独立，由档案删除、管理员删号、封禁（`yggdrasil_access`）以及 `invalidate`/`signout` 吊销。
