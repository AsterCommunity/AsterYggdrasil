# 启动器登录

启动器登录走 Yggdrasil authserver 协议。AsterYggdrasil 复用站点账号作为登录身份，再把账号下的 Minecraft profiles 暴露给启动器选择。

## 登录流程

启动器调用：

```text
POST /api/yggdrasil/authserver/authenticate
```

请求通常包含：

- `username`: 账号邮箱、账号名，或在允许时使用 Minecraft profile name。
- `password`: 站点账号密码。
- `clientToken`: 启动器生成的客户端标识。
- `agent`: Minecraft agent。
- `requestUser`: 是否返回用户属性。

成功响应包含：

- `accessToken`
- `clientToken`
- `availableProfiles`
- `selectedProfile`
- 可选 `user`

## clientToken

`clientToken` 是启动器侧标识，不是认证密钥。客户端提供时，refresh 和 validate 会校验它是否和 token 匹配；invalidate 按 Yggdrasil 规范只检查 `accessToken`，`clientToken` 不影响吊销结果。

没有提供 `clientToken` 时，服务端会生成一个并在响应中返回。

## selectedProfile

一个站点账号可以有多个 Minecraft profiles。启动器登录后会收到 `availableProfiles` 和 `selectedProfile`。

如果账号没有 profile，登录仍可能成功，但没有可用于进服的 selected profile。用户需要先在站点创建 profile。

refresh 会保留/更新 selected profile 信息，并返回新的 access token。旧 access token 会失效。

## Token 生命周期

Yggdrasil token 有有效期和最大活跃数量限制：

- `yggdrasil_token_ttl_days`
- `yggdrasil_max_active_tokens`

服务端会在颁发 token 时清理旧 token，也有后台任务 `yggdrasil-token-cleanup` 删除过期或已吊销 token。

`invalidate` 吊销单个 token。`signout` 使用账号密码吊销该用户的所有 Yggdrasil token。

## 进服验证

Minecraft 客户端进服时调用：

```text
POST /api/yggdrasil/sessionserver/session/minecraft/join
```

Minecraft 服务端随后调用：

```text
GET /api/yggdrasil/sessionserver/session/minecraft/hasJoined
```

`hasJoined` 成功响应包含 profile id、name 和 properties。返回的 profile properties 会带数字签名，服务端可用 metadata 里的公钥验证。

## Profile Name 登录

`yggdrasil_allow_profile_name_login` 控制是否允许用户用 Minecraft profile name 登录。

关闭时，`feature.non_email_login` 会按策略反映能力，启动器不应该假设 profile name 一定可登录。

## 常见失败

- access token 过期或被吊销：重新登录。
- clientToken 不匹配：启动器应使用同一客户端配置刷新或重新登录。
- selected profile 不存在：用户需要创建 profile，或选择仍存在的 profile。
- profile 被删除：关联 token 会被吊销，需要重新创建 profile 并重新登录。
