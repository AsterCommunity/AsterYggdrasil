# Yggdrasil API 实现说明

这份文档记录仓库当前的 Yggdrasil/authlib-injector 协议实现，面向继续开发和排查兼容问题的人。用户接入说明放在 `docs/guide/yggdrasil-api.md` 和 `docs/guide/yggdrasil-textures.md`，这里重点写代码边界、端点行为、鉴权和测试约定。

## 代码边界

| 层 | 位置 | 约定 |
| --- | --- | --- |
| 路由 | `src/api/routes/yggdrasil.rs`、`src/api/routes/yggdrasil/texture.rs`、`src/api/routes/yggdrasil/minecraft_services.rs` | handler 只做参数提取、协议鉴权、调用 service 和组装协议响应。 |
| DTO | `src/api/dto/yggdrasil.rs` | 所有协议字段都放这里，保持 wire name 和 authlib-injector/Mojang 兼容。不要在 handler 里临时 `json!` 拼协议响应。 |
| Service | `src/services/yggdrasil_service.rs`、`src/services/yggdrasil_service/*` | token、profile、session、metadata、minecraftservices 行为在这里实现。 |
| 材质处理 | `src/services/texture_service/`、`src/texture_storage/` | PNG 校验、重编码、hash、存储、公开读取都走 texture service。 |
| 配置 | `src/config/definitions.rs`、`src/config/yggdrasil.rs` | Yggdrasil 运行时配置必须在这里定义和规范化。 |
| OpenAPI | `src/api/openapi.rs` | 新增或改动协议 DTO/端点后要注册 path 和 schema。 |
| 测试 | `tests/test_yggdrasil.rs` | 协议兼容、错误体、token 生命周期、材质和 minecraftservices 端点都在这里覆盖。 |

Yggdrasil 协议端点不使用项目 API envelope。`/api/v1` 仍然返回：

```json
{ "code": "success", "msg": "", "data": {} }
```

`/api/yggdrasil` 下必须返回协议原生字段和错误体。

## API Root

默认协议根路径：

```text
/api/yggdrasil
```

首页 `/` 会通过 `X-Authlib-Injector-API-Location: /api/yggdrasil/` 暴露 API Location Indication。authlib-injector 请求会把 Mojang 域名映射到这个根路径，例如：

| 原始服务 | AsterYggdrasil 路径 |
| --- | --- |
| `authserver.mojang.com` / `authserver.ely.by` 等认证服务 | `/api/yggdrasil/authserver/*` |
| `sessionserver.mojang.com` | `/api/yggdrasil/sessionserver/*` |
| `api.minecraftservices.com` | `/api/yggdrasil/minecraftservices/*` |

客户端转发来的 `Authorization` header 会被保留，所以 `minecraftservices` 端点可以按 Mojang 风格验证 Bearer token。

## Metadata

| Method | Path | Auth | 主要实现 |
| --- | --- | --- | --- |
| `GET` | `/api/yggdrasil` | 无 | `metadata` |
| `GET` | `/api/yggdrasil/` | 无 | `metadata` |

响应 DTO：`YggdrasilMetaResp`。

当前返回：

- `meta.serverName`: 来自 `yggdrasil_server_name`。
- `meta.implementationName`: 固定为 `AsterYggdrasil`。
- `meta.implementationVersion`: 当前 crate 版本。
- `meta.links.homepage`: 有 `public_site_url` 时返回站点首页。
- `meta.links.register`: 允许用户注册时返回 `/register`。
- `meta.feature.non_email_login`: 来自 `yggdrasil_allow_profile_name_login`。
- `meta.feature.enable_profile_key`: 来自 `yggdrasil_enable_profile_key`。
- `meta.feature.enable_mojang_anti_features`: 来自 `yggdrasil_enable_mojang_anti_features`。
- `meta.feature.username_check`: 当前固定为 `true`，让 authlib-injector 保持用户名字符检查；Aster 当前 profile name 规则也是 3-16 位 ASCII 字母、数字、下划线。
- `skinDomains`: 默认 Mojang 域名、配置域名和当前公开材质 URL host 的合集。
- `signaturePublickey`: 用于验证 `textures` property 签名的 RSA 公钥。

当前不声明这些高级 flag：

- `feature.legacy_skin_api`
- `feature.no_mojang_namespace`

按 authlib-injector 语义，不返回通常等价于 false，也就是不声明服务端支持。

`feature.legacy_skin_api` 继续不声明。旧官方域名 `skins.minecraft.net` 已经不是可靠入口，现代官方链路是 username -> UUID -> sessionserver profile -> `textures.minecraft.net/texture/{hash}`，没必要在服务端实现旧式 `GET /skins/MinecraftSkins/{username}.png`。

metadata 使用 `Cache-Control: no-cache, no-store, must-revalidate`。签名密钥或公开 URL 变化后，客户端应重新拉 metadata。

## Authserver

| Method | Path | Auth | 成功 | 失败 |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/authserver/authenticate` | 账号密码 | `200` + `YggdrasilAuthenticateResp` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/refresh` | request body token | `200` + `YggdrasilRefreshResp` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/validate` | request body token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/invalidate` | request body token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/signout` | 账号密码 | `204` | `400`/`403` + `YggdrasilErrorBody` |

实现入口：

- route: `src/api/routes/yggdrasil.rs`
- service: `src/services/yggdrasil_service/auth.rs`、`login.rs`、`token.rs`

关键约定：

- `authenticate` 支持邮箱/账号标识登录；`yggdrasil_allow_profile_name_login = true` 时也接受 profile name。
- `signout` 复用登录解析，所以在允许 profile name login 时也会接受 profile name。这是宽松扩展，和部分文档写“username 是邮箱”的字面定义不完全一致。
- access token 明文只返回给客户端；数据库保存 token hash。
- `clientToken` 由客户端提供；缺省时服务端生成。
- `refresh` 必须在同一事务内吊销旧 token 并签发新 token。失败时旧 token 仍应可用。
- profile rename 会临时失效对应 selected profile 的 token，让启动器通过 refresh 拿到新名称。
- 高频认证端点有 debug 日志，但日志只能记录长度、hash、布尔状态、token id、user id、profile id 等非明文字段。

## Sessionserver

| Method | Path | Auth | 成功 | 失败/未命中 |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/sessionserver/session/minecraft/join` | request body access token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `GET` | `/api/yggdrasil/sessionserver/session/minecraft/hasJoined` | 无 | `200` + `YggdrasilProfile` | 未命中 `204`，非法请求 `400` |
| `GET` | `/api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}` | 无 | `200` + `YggdrasilProfile` | 未命中 `204`，非法请求 `400` |
| `GET` | `/api/yggdrasil/sessionserver/blockedservers` | 无 | 当前无 `200` | `404` + `MinecraftServicesPathError` |

实现入口：

- route: `src/api/routes/yggdrasil.rs`
- blocked servers route: `src/api/routes/yggdrasil/minecraft_services.rs`
- service: `src/services/yggdrasil_service/session.rs`、`properties.rs`

关键约定：

- `join` 验证 access token、selected profile 和 serverId，并记录 profile、serverId、客户端 IP。当前不保存明文 access token，足够支持 `hasJoined`，也减少敏感数据留存。
- `hasJoined` 按 username、serverId 和可选 IP 匹配 join 记录。
- `profile/{uuid}` 的 `unsigned` 默认是 `true`。传 `unsigned=false` 时 `textures` property 会带签名。
- `blockedservers` 目前返回 404 空路径响应，等价于不提供 Mojang 阻止服务器列表。以后如果做封禁/风控系统，不要只依赖这个端点；真正的封禁还要在 `authenticate`、`refresh`、`validate`、`join`、`hasJoined` 等链路强制执行。

## Profile Lookup

| Method | Path | Auth | 成功 | 失败 |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/api/profiles/minecraft` | 无 | `200` + `YggdrasilProfile[]` | `400` + `YggdrasilErrorBody` |

请求体是 profile name 数组。当前限制最多 100 个名称，名称必须符合 Minecraft profile name 规则：3-16 位 ASCII 字母、数字、下划线。

未命中的名称不会出现在响应数组里。

## Texture API

| Method | Path | Auth | 成功 | 失败 |
| --- | --- | --- | --- | --- |
| `PUT` | `/api/yggdrasil/api/user/profile/{uuid}/{skin\|cape}` | Bearer access token | `204` | `400`/`401`/`403` + `YggdrasilErrorBody` |
| `DELETE` | `/api/yggdrasil/api/user/profile/{uuid}/{skin\|cape}` | Bearer access token | `204` | `400`/`401`/`403` + `YggdrasilErrorBody` |
| `GET` | `/api/yggdrasil/textures/{hash}` | 无 | `200 image/png` | `404` |

实现入口：

- route: `src/api/routes/yggdrasil/texture.rs`
- service: `src/services/texture_service/`

上传约定：

- `PUT` 使用 `multipart/form-data`。
- 文件字段名是 `file`，必须是 `image/png`。
- skin 可选文本字段 `model`，支持 texture service 里的模型解析；cape 会强制使用默认模型。
- skin 尺寸允许 `64x32` 或 `64x64` 的整数倍。
- cape 尺寸允许 `64x32` 或 `22x17` 的整数倍；旧式 `22x17` cape 会补透明到标准 `64x32` 比例。
- 服务端会解码 PNG、校验像素数、重编码为干净 PNG，再按处理后内容计算 SHA-256 hash。
- 公开读取走 hash URL，带 texture service 给出的 `Cache-Control`、`ETag` 和 `Content-Length`；`If-None-Match` 命中时返回 `304`。
- `/textures/{hash}` 除了数据库里的材质，也能读取内置 Steve/Alex 默认皮肤 hash。未绑定 skin 的 profile 在 `textures` property 中会自动补一个基于 profile UUID 稳定选择的默认 skin。

authlib-injector 明确要求材质上传/删除缺失或错误 token 返回 `401`。这和普通 Yggdrasil token 错误多用 `403 ForbiddenOperationException` 不一样，别顺手统一掉。

## Minecraft Services

这些端点对应 `api.minecraftservices.com` 被 authlib-injector 重定向后的路径。

| Method | Path | Feature flag | Auth | 成功 | 失败 |
| --- | --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/minecraftservices/player/certificates` | `feature.enable_profile_key` | Bearer access token，且 token 必须绑定 selected profile | `200` + `MinecraftServicesCertificateResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/privileges` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPrivilegesResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/player/attributes` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPlayerAttributesResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/privacy/blocklist` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPrivacyBlocklistResp` | `401`/`404` |

实现入口：

- route: `src/api/routes/yggdrasil/minecraft_services.rs`
- service: `src/services/yggdrasil_service/minecraft_services.rs`

错误体匹配 Mojang 风格：

```json
{ "path": "/player/attributes" }
```

缺失、无效、过期、吊销、临时失效 token 都返回 `401`。对应 feature flag 关闭或未匹配路径返回 `404`，并设置 `Cache-Control: no-store`。

当前策略：

- `player/certificates` 生成临时 2048-bit RSA keypair，返回给已登录且选中 profile 的客户端。
- `publicKeySignature` 和 `publicKeySignatureV2` 是 authlib-injector 兼容用 dummy 值，不是 Mojang 官方签名。自建服务不能伪造 Mojang 官方签名。
- `expiresAt` 当前是签发后 48 小时。
- `refreshedAfter` 当前是签发后 36 小时。
- `privileges` 当前返回 chat、multiplayer、realms、telemetry、optionalTelemetry 全部 enabled。
- `player/attributes` 当前返回 permissive privileges、关闭脏话过滤、好友功能 disabled、文字聊天 enabled、空 ban scopes。
- `privacy/blocklist` 当前返回空 `blockedProfiles`。

未来封禁系统接入点：

- `minecraft_services_privileges`: 根据账号/profile 封禁关闭 `onlineChat`、`multiplayerServer`、`multiplayerRealms`。
- `minecraft_services_player_attributes`: 根据封禁和社交设置填充 `friendsPreferences`、`chatPreferences`、`banStatus.bannedScopes`。
- `minecraft_services_privacy_blocklist`: 接用户屏蔽列表。
- `blockedservers`: 如需提供服务端阻止列表，再改成协议需要的响应；否则保持 404 即可。
- 强制执行仍应放在认证和 join/session 链路，Minecraft services policy 只能作为客户端可见策略面。

`keyPair.privateKey` 是返回给客户端的敏感字段，不要持久化、不要日志输出、不要进 audit details。

## 字段参考

这里按 wire field 解释字段。Rust 字段名可能是 snake_case，但协议里必须保持这里的驼峰、点号或大写枚举值。

字段语义分三类：

- **协议已明确**：Yggdrasil/authlib-injector 文档或当前实现能直接确定用途。
- **从 Mojang 样例推断**：字段存在于 Mojang 官方响应或 authlib-injector 兼容层里，但没有在本仓库找到完整字段规范。这里会显式写“推测”。
- **Aster 当前约定**：字段协议存在，但当前值是我们为了兼容客户端返回的固定策略，未来接封禁、社交或偏好系统时要改。

### 通用错误字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `YggdrasilErrorBody` | `error` | string | 协议错误类型，例如 `ForbiddenOperationException`、`IllegalArgumentException`。客户端通常按这个粗分类处理。 |
| `YggdrasilErrorBody` | `errorMessage` | string | 可展示/可调试的错误说明。不能包含 access token、密码、私钥等敏感值。 |
| `YggdrasilErrorBody` | `cause` | string? | 可选错误原因。当前实现通常不返回，保留给协议兼容。 |
| `MinecraftServicesPathError` | `path` | string | Mojang 风格 minecraftservices 错误体，只返回被拒绝的相对路径，例如 `/privileges`。 |

### Profile 和 Property 字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `YggdrasilProfile` | `id` | string | 无横线 Minecraft UUID。当前 profile UUID 是创建 profile 时生成并持久化的值。 |
| `YggdrasilProfile` | `name` | string | Minecraft profile name，3-16 位 ASCII 字母、数字、下划线。 |
| `YggdrasilProfile` | `properties` | array? | profile 附加属性。authenticate/refresh 的 profile 摘要通常不带材质；session profile 会带材质相关 property。 |
| `YggdrasilProfileProperty` | `name` | string | 属性名。当前主要是 `textures` 和 `uploadableTextures`。 |
| `YggdrasilProfileProperty` | `value` | string | 属性值。`textures` 是 base64(JSON)，`uploadableTextures` 是逗号分隔能力值。 |
| `YggdrasilProfileProperty` | `signature` | string? | `unsigned=false` 或需要签名时返回。签名用 metadata 里的 `signaturePublickey` 验证。 |
| `YggdrasilUser` | `id` | string | Aster 用户 id 的协议表示。 |
| `YggdrasilUser` | `properties` | array | 用户级属性。当前固定空数组；`preferredLanguage` 等 authlib-injector 可选属性还没暴露。 |

`textures` property 的 `value` 解码后是 JSON：

| 字段 | 类型 | 语义 |
| --- | --- | --- |
| `timestamp` | number | 服务端生成 property 的毫秒时间戳。 |
| `profileId` | string | 无横线 Minecraft UUID。 |
| `profileName` | string | property 生成时的 profile name。 |
| `textures` | object | 材质映射，key 通常是 `SKIN`、`CAPE`。 |
| `textures.SKIN.url` | string | 公开 skin PNG URL。URL 的 host 必须被 metadata `skinDomains` 覆盖。 |
| `textures.SKIN.metadata.model` | string? | skin 模型。`slim` 表示细手臂；默认模型不需要 metadata。 |
| `textures.CAPE.url` | string | 公开 cape PNG URL。 |

如果 profile 没有绑定 skin，当前实现会补内置默认皮肤。默认模型由 profile UUID 最低位稳定决定：偶数使用 Steve/default，奇数使用 Alex/slim。默认皮肤也通过 `/api/yggdrasil/textures/{hash}` 公开读取。

`uploadableTextures` 的 `value` 当前来自 profile 记录里的可上传能力，例如 `skin,cape`、`skin`、`cape`。它告诉 authlib-injector 客户端这个 profile 可通过协议上传哪些材质。

### Authserver 请求/响应字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `YggdrasilAgentReq` | `name` | string | 客户端声明的 agent。当前只支持 Minecraft 语义。 |
| `YggdrasilAgentReq` | `version` | number | agent 版本。当前要求为 `1`。 |
| `YggdrasilAuthenticateReq` | `username` | string | 登录标识。默认是邮箱/账号标识；开启 profile-name login 后也可为 profile name。 |
| `YggdrasilAuthenticateReq` | `password` | string | 明文密码，只能用于本次验证，不得日志输出或持久化。 |
| `YggdrasilAuthenticateReq` | `clientToken` | string? | 启动器生成的客户端标识。缺省时服务端生成并返回。 |
| `YggdrasilAuthenticateReq` | `requestUser` | bool | 是否要求响应包含 `user` 字段。 |
| `YggdrasilAuthenticateReq` | `agent` | object? | agent 信息。存在时必须通过校验。 |
| `YggdrasilAuthenticateResp` | `accessToken` | string | 新签发的 Yggdrasil access token。只返回给客户端；数据库只保存 hash。 |
| `YggdrasilAuthenticateResp` | `clientToken` | string | 本次会话绑定的 client token，来自请求或服务端生成。 |
| `YggdrasilAuthenticateResp` | `availableProfiles` | array | 当前账号可用 profile 列表。 |
| `YggdrasilAuthenticateResp` | `selectedProfile` | object? | 已选择的 profile。只有账号存在可选 profile 且服务端按协议选择时返回。 |
| `YggdrasilAuthenticateResp` | `user` | object? | `requestUser=true` 时返回用户对象。 |
| `YggdrasilRefreshReq` | `accessToken` | string | 要刷新的旧 access token。刷新成功后旧 token 会被吊销。 |
| `YggdrasilRefreshReq` | `clientToken` | string? | 如果提供，必须匹配旧 token 绑定的 client token。 |
| `YggdrasilRefreshReq` | `requestUser` | bool | 是否要求响应包含 `user` 字段。 |
| `YggdrasilRefreshReq` | `selectedProfile` | object? | 请求把 token 绑定到某个 profile。旧 token 已绑定 profile 时不能再次改绑。 |
| `YggdrasilRefreshResp` | `accessToken` | string | 刷新后的新 access token。 |
| `YggdrasilRefreshResp` | `clientToken` | string | 绑定的 client token。 |
| `YggdrasilRefreshResp` | `selectedProfile` | object? | 新 token 绑定的 selected profile。 |
| `YggdrasilRefreshResp` | `user` | object? | `requestUser=true` 时返回用户对象。 |
| `YggdrasilTokenReq` | `accessToken` | string | validate/invalidate 使用的 access token。 |
| `YggdrasilTokenReq` | `clientToken` | string? | 如果提供，必须匹配 token 绑定值。 |
| `YggdrasilSignoutReq` | `username` | string | 账号登录标识。当前会复用登录解析，所以可能接受 profile name。 |
| `YggdrasilSignoutReq` | `password` | string | 明文密码，只用于确认登出请求。 |

### Sessionserver 字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `YggdrasilJoinReq` | `accessToken` | string | 客户端登录后拿到的 access token。 |
| `YggdrasilJoinReq` | `selectedProfile` | string | 无横线 UUID。必须属于 token 绑定用户。 |
| `YggdrasilJoinReq` | `serverId` | string | Minecraft 客户端和服务端握手得到的 serverId/hash。日志里只能记录 hash。 |
| `YggdrasilHasJoinedQuery` | `username` | string | 服务端要验证的 profile name。 |
| `YggdrasilHasJoinedQuery` | `serverId` | string | 和 join 请求一致的 serverId/hash。 |
| `YggdrasilHasJoinedQuery` | `ip` | string? | 可选客户端 IP。提供时必须和 join 记录匹配。 |
| `YggdrasilProfileQuery` | `unsigned` | bool? | 是否省略 property 签名。缺省按 `true` 处理；传 `false` 会返回签名。 |

### Metadata 字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `YggdrasilMetaResp` | `meta` | object | authlib-injector metadata 主体。 |
| `YggdrasilMetaResp` | `skinDomains` | string[] | 客户端允许加载材质的域名白名单。支持普通 host 和以点开头的域名后缀。 |
| `YggdrasilMetaResp` | `signaturePublickey` | string | PEM 格式 RSA 公钥，用于验 `textures`、`uploadableTextures` 等 property 签名。 |
| `YggdrasilMeta` | `serverName` | string | 服务显示名称。 |
| `YggdrasilMeta` | `implementationName` | string | 实现名称。当前固定 `AsterYggdrasil`。 |
| `YggdrasilMeta` | `implementationVersion` | string | 当前服务版本。 |
| `YggdrasilMeta` | `links` | object? | 站点链接。authlib-injector 可展示这些入口。 |
| `YggdrasilMeta` | `feature.non_email_login` | bool | 是否支持非邮箱登录标识。当前对应 profile name login。 |
| `YggdrasilMeta` | `feature.enable_profile_key` | bool | 是否由服务端处理 Minecraft profile key 证书端点。 |
| `YggdrasilMeta` | `feature.enable_mojang_anti_features` | bool | 是否由服务端处理 Mojang anti-features/policy 端点。 |
| `YggdrasilMeta` | `feature.username_check` | bool | 是否让 authlib-injector 启用用户名字符检查。当前固定 true，因为 Aster profile name 已限制为 Mojang 合法字符；未来如果支持自定义命名规则，需要重新评估这个 flag。 |
| `YggdrasilMetaLinks` | `homepage` | string | 站点首页 URL。 |
| `YggdrasilMetaLinks` | `register` | string? | 注册入口 URL。注册关闭时省略。 |

### Minecraft Services 字段

| DTO | 字段 | 类型 | 语义 |
| --- | --- | --- | --- |
| `MinecraftServicesCertificateResp` | `keyPair` | object | 客户端用于 profile key/chat signing 流程的临时 RSA keypair。 |
| `MinecraftServicesCertificateResp` | `publicKeySignature` | string | 对 public key 的签名字段。推测用于让客户端验证这个 profile key 由服务端签发；当前是 authlib-injector 兼容 dummy 值，不具备 Mojang 官方信任链语义。 |
| `MinecraftServicesCertificateResp` | `publicKeySignatureV2` | string | 新版客户端使用的第二签名字段。推测是新版签名载荷/算法兼容字段；当前同样是 dummy 值。 |
| `MinecraftServicesCertificateResp` | `expiresAt` | string | RFC3339 时间，超过后客户端不应继续使用该 keypair。 |
| `MinecraftServicesCertificateResp` | `refreshedAfter` | string | RFC3339 时间，超过后客户端应主动刷新证书。 |
| `MinecraftServicesKeyPair` | `privateKey` | string | PKCS#1 PEM 私钥。只返回给客户端，禁止保存或记录。 |
| `MinecraftServicesKeyPair` | `publicKey` | string | PKCS#1 PEM 公钥。 |
| `MinecraftServicesPrivilegesResp` | `privileges` | object | 当前账号/profile 的服务权限总览。 |
| `MinecraftServicesPlayerAttributesResp` | `privileges` | object | 同 `privileges` 端点，嵌入在 attributes 响应里。 |
| `MinecraftServicesPlayerAttributesResp` | `profanityFilterPreferences` | object | 脏话过滤偏好。推测影响客户端本地聊天过滤 UI/行为；当前总是关闭。 |
| `MinecraftServicesPlayerAttributesResp` | `friendsPreferences` | object | 好友系统偏好。推测影响社交/邀请相关客户端功能；当前 friends 和 acceptInvites 都是 `DISABLED`。 |
| `MinecraftServicesPlayerAttributesResp` | `chatPreferences` | object | 聊天偏好。推测影响客户端文本通信开关；当前 textCommunication 是 `ENABLED`。 |
| `MinecraftServicesPlayerAttributesResp` | `banStatus` | object | 封禁状态。推测供客户端展示或禁用部分在线能力；当前 bannedScopes 为空对象，真实封禁仍必须在服务端认证/join 链路执行。 |
| `MinecraftServicesPrivileges` | `onlineChat` | object | 是否允许在线聊天。未来聊天封禁应改这里。 |
| `MinecraftServicesPrivileges` | `multiplayerServer` | object | 是否允许加入多人服务器。未来多人游戏封禁应改这里。 |
| `MinecraftServicesPrivileges` | `multiplayerRealms` | object | 是否允许 Realms 多人游戏。 |
| `MinecraftServicesPrivileges` | `telemetry` | object | 推测表示必要 telemetry 能力/策略是否启用。Mojang 样例为 true；Aster 当前也返回 true，只做兼容声明。 |
| `MinecraftServicesPrivileges` | `optionalTelemetry` | object | 推测表示可选 telemetry 能力/策略是否启用。Mojang 样例为 true；Aster 当前也返回 true，只做兼容声明。 |
| `MinecraftServicesPrivilege` | `enabled` | bool | 单个权限是否开启。 |
| `MinecraftServicesProfanityFilterPreferences` | `profanityFilterOn` | bool | 推测表示客户端是否应启用脏话过滤。 |
| `MinecraftServicesFriendsPreferences` | `friends` | `ENABLED`/`DISABLED` | 推测表示好友列表功能状态。 |
| `MinecraftServicesFriendsPreferences` | `acceptInvites` | `ENABLED`/`DISABLED` | 推测表示是否接受好友邀请。 |
| `MinecraftServicesChatPreferences` | `textCommunication` | `ENABLED`/`DISABLED` | 推测表示文本聊天功能状态。 |
| `MinecraftServicesBanStatus` | `bannedScopes` | object | 推测是封禁 scope 映射。空对象表示没有活跃封禁；未来接封禁系统时需要用实际客户端识别的 scope 名称填充，不能随便自造。 |
| `MinecraftServicesPrivacyBlocklistResp` | `blockedProfiles` | string[] | 推测是当前用户屏蔽的 profile UUID 列表，用于客户端社交/聊天屏蔽；当前为空数组。 |

## 错误形状

普通 Yggdrasil 错误体：

```json
{
  "error": "ForbiddenOperationException",
  "errorMessage": "Invalid token."
}
```

映射在 `src/services/yggdrasil_service/error.rs`：

| Kind | HTTP | `error` |
| --- | --- | --- |
| `InvalidToken` | `403` | `ForbiddenOperationException` |
| `InvalidCredentials` | `403` | `ForbiddenOperationException` |
| `ForbiddenProfile` | `403` | `ForbiddenOperationException` |
| `BadRequest` | `400` | `IllegalArgumentException` |
| `AccessTokenAlreadyHasProfile` | `400` | `IllegalArgumentException` |
| `UnsupportedAgent` | `400` | `IllegalArgumentException` |
| `TooManyProfilesRequested` | `400` | `IllegalArgumentException` |
| `ProfileNotFound` | `204` | 无 body |
| `Internal` | `500` | `InternalServerError` |

Minecraft services 端点是例外：非内部错误统一映射成 `401 { "path": "..." }`，feature 关闭和未知路径映射成 `404 { "path": "..." }`。

## 运行时配置

相关 key 都在 `CONFIG_CATEGORY_YGGDRASIL`：

| Key | 作用 |
| --- | --- |
| `yggdrasil_server_name` | metadata `meta.serverName`。 |
| `yggdrasil_allow_profile_name_login` | 允许 profile name 登录，同时驱动 `feature.non_email_login`。 |
| `yggdrasil_allow_skin_upload` | 控制 Yggdrasil skin 上传能力，并影响 `uploadableTextures` property。 |
| `yggdrasil_allow_cape_upload` | 控制 Yggdrasil cape 上传能力，并影响 `uploadableTextures` property。 |
| `yggdrasil_enable_profile_key` | 开启 `/minecraftservices/player/certificates`，并声明 `feature.enable_profile_key`。 |
| `yggdrasil_enable_mojang_anti_features` | 开启 `/minecraftservices/privileges`、`/player/attributes`、`/privacy/blocklist`，并声明 `feature.enable_mojang_anti_features`。 |
| `yggdrasil_token_ttl_days` | Yggdrasil access token 生命周期。 |
| `yggdrasil_max_active_tokens` | 每个用户保留的最大活跃 Yggdrasil token 数。 |
| `yggdrasil_max_texture_upload_bytes` | 单次材质上传字节上限。 |
| `yggdrasil_max_texture_pixels` | 解码后材质像素上限。 |
| `yggdrasil_skin_domains` | 额外材质域名白名单。 |
| `yggdrasil_public_base_url` | 高级覆盖项，用于生成公开 Yggdrasil/材质 URL。 |
| `yggdrasil_signature_public_key` | 可选公钥覆盖。 |
| `yggdrasil_signature_private_key` | textures property 签名私钥，敏感配置。 |

改配置项时要同时更新：

- `src/config/definitions.rs`
- `src/config/yggdrasil.rs`
- `frontend-panel/src/i18n/locales/*/settings.json`
- 相关 OpenAPI/生成类型，如 DTO 对外暴露发生变化
- `tests/test_yggdrasil.rs` 或配置 normalizer 测试

## OpenAPI 和生成类型

协议端点虽然不使用项目 envelope，但仍注册进 OpenAPI，便于前端和调试工具看到完整 contract。

改动流程：

```bash
cargo fmt
cargo test --features openapi --test generate_openapi
bun run --cwd frontend-panel generate-api
cargo test --test test_yggdrasil
cargo check
```

只改文档时不需要跑这些命令；只改 DTO/route/schema 时至少跑 OpenAPI 生成和 `test_yggdrasil`。

## 测试覆盖要求

新增或修改 Yggdrasil 行为时，至少覆盖：

- 成功响应字段和 HTTP 状态。
- 协议错误体，不要误返回项目 envelope。
- token 缺失、错误、过期、吊销、clientToken 不匹配。
- selected profile 归属和 rename 后临时失效行为。
- `unsigned=false` 的 textures property 签名。
- 材质上传的 content type、尺寸、上限、hash、公开读取和删除。
- Minecraft services 的 feature flag 开关、Bearer token 鉴权、401 path body、404 path body。
- profile key 响应字段存在，但不要求 Mojang 官方签名。

安全边界测试要特别关注 access token、client token、私钥、profile key privateKey 不出现在日志、错误、audit details 或持久化明文字段里。
