---
description: AsterYggdrasil 能力封禁指南，说明管理员如何按 scope 限制用户使用 Yggdrasil、profile、材质上传和公共材质库交互能力。
---

# 能力封禁

::: tip 这一篇覆盖什么
这页写给站点管理员。能力封禁用于限制某个用户能不能使用特定功能，而不是封掉整个账号或某个 Minecraft profile。
:::

能力封禁是按 **用户 + scope** 生效的限制。一条封禁记录可以包含多个 scope，例如同时限制材质上传和 profile 管理。公共材质库浏览不属于封禁范围，用户即使被限制交互，也仍可浏览公开材质。

## 什么时候用

常见场景：

- 用户反复上传违规或损坏材质，只限制材质上传。
- 用户滥用 profile 名称或频繁创建/删除 profile，只限制 profile 管理。
- 用户暂时不应进服，但账号仍可登录站点，只限制 Yggdrasil 进服。
- 用户不应提交、撤回或举报公共材质，只限制公共材质库交互。

如果要阻止用户登录整个站点，应使用用户状态或会话吊销；能力封禁只处理具体业务能力。

## Scope 说明

| Scope | 影响范围 | 不影响 |
| --- | --- | --- |
| `yggdrasil_access` | Yggdrasil/authlib-injector 登录、refresh/validate 等 token 使用 | 站点网页登录 |
| `yggdrasil_join` | `join`、`hasJoined` 命中、Minecraft services multiplayer 状态 | 站点网页登录、材质浏览 |
| `minecraft_profile_manage` | 创建、删除、改名 Minecraft profile，以及需要改写 profile 材质绑定的操作 | 查看自己已有 profile |
| `texture_upload` | wardrobe 上传、Yggdrasil 直接上传、更新 wardrobe 材质元数据、绑定/删除 profile 材质 | 公共材质库浏览 |
| `texture_library_interact` | 提交/撤回公共材质库审核、举报、以及用户侧公共库交互动作 | 公共材质库浏览 |

`texture_upload` 会在接收上传文件前检查。被限制的用户不能先把图片写到磁盘或创建材质记录后再失败。

## 管理入口

在管理后台打开用户详情页，进入“能力封禁”区域：

1. 新建封禁。
2. 勾选一个或多个 scope。
3. 填写管理员内部原因。
4. 可选填写用户可见原因、开始时间、过期时间和内部备注。
5. 保存。

开始时间为空时立即生效。过期时间为空时表示长期有效。过期时间必须晚于开始时间。

同一个用户不能同时存在覆盖同一 scope 的有效封禁。例如已有一条有效记录包含 `texture_upload`，就不能再创建另一条包含 `texture_upload` 的有效记录；需要先更新或撤销原记录。

## 更新和撤销

只能更新或撤销当前有效的封禁。已过期或已撤销的记录保留为历史，不应继续编辑。

更新封禁时可以调整 scope 列表、原因、可见原因、备注和时间范围。撤销封禁时可以填写撤销说明。创建、更新和撤销都会写入审计，并且封禁事件页会保留变更前后的 scope、状态和过期时间。

## 用户能看到什么

用户账户首页会显示自己当前有效的能力限制。默认不显示已撤销或已过期的记录，避免把历史处罚堆到日常页面里。

用户可见信息包括：

- 被限制的 scope。
- 当前状态。
- 开始时间和过期时间。
- 用户可见原因；如果没有填写可见原因，会显示封禁原因。

用户看不到管理员备注、操作员 ID、撤销说明等内部字段。

## API 速查

管理员 API 使用项目统一响应 envelope。

```text
GET    /api/v1/admin/user-bans
GET    /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/users/{user_id}/bans
PATCH  /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/user-bans/{ban_id}/revoke
GET    /api/v1/admin/user-bans/{ban_id}/events
```

创建请求示例：

```json
{
  "scopes": ["texture_upload", "minecraft_profile_manage"],
  "reason": "repeated invalid uploads",
  "public_reason": "Texture upload is temporarily restricted.",
  "admin_note": "review again after appeal",
  "starts_at": null,
  "expires_at": "2026-07-01T00:00:00Z"
}
```

`scopes` 必须是非空数组。接口不接受旧的单个 `scope` 字段。

列表接口可以用单个 `scope` 查询包含该 scope 的记录：

```text
GET /api/v1/admin/user-bans?user_id=123&scope=texture_upload&effective_only=true
```

用户自查 API：

```text
GET /api/v1/account/bans?effective_only=true
```

## 错误码

项目 API 被能力封禁拦截时返回 `403`，错误码为：

```json
{
  "code": "user_ban.forbidden",
  "error": {
    "retryable": false
  }
}
```

管理操作常见错误码：

| 错误码 | 含义 |
| --- | --- |
| `user_ban.already_active` | 目标用户已有覆盖相同 scope 的有效封禁 |
| `user_ban.not_active` | 试图更新或撤销已过期/已撤销封禁 |
| `user_ban.duration_invalid` | 过期时间不晚于开始时间 |
| `user_ban.reason_invalid` | 原因、备注或 scope 列表不合法 |
| `user_ban.not_found` | 封禁记录不存在 |

Yggdrasil 协议端点仍返回协议错误体，不使用项目 envelope。启动器通常只能看到 Yggdrasil 的 `ForbiddenOperationException` 或 invalid token/credentials 类错误，这是协议兼容要求。

## 上线前检查

- operator 需要 `users` scope 才能管理能力封禁。
- 给封禁填写用户可见原因，避免用户只看到笼统的失败。
- 不要用能力封禁替代账号禁用；两者语义不同。
- 测试 `texture_upload` 时确认失败发生在上传文件落盘之前。
- 确认用户账户页不会默认展示已撤销记录。
