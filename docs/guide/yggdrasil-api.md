# Yggdrasil API

AsterYggdrasil 的协议 API 根路径是：

```text
/api/yggdrasil
```

这个根路径专门服务 Minecraft 启动器、authlib-injector 和 Minecraft 服务端，不返回项目管理 API 的统一 envelope。协议错误也按 Yggdrasil/authlib-injector 兼容格式返回。

## API Location Indication

站点首页 `/` 会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

这就是 authlib-injector 的 API Location Indication。用户在支持 ALI 的启动器里填写站点地址时，启动器可以从响应头发现真正的 Yggdrasil API 根路径。

如果你直接配置 authlib-injector，也可以使用完整 API 根路径：

```text
-javaagent:authlib-injector.jar=https://example.com/api/yggdrasil
```

## Metadata

```text
GET /api/yggdrasil
GET /api/yggdrasil/
```

metadata 包含：

- `meta.serverName`: 服务显示名称。
- `meta.implementationName`: 实现名称，当前为 `AsterYggdrasil`。
- `meta.implementationVersion`: 当前服务版本。
- `meta.feature.non_email_login`: 是否允许非邮箱标识登录，当前由 `yggdrasil_allow_profile_name_login` 控制。
- `skinDomains`: 材质 URL 域名白名单。
- `signaturePublickey`: 用于验证 profile properties 签名的 RSA 公钥。

metadata 响应使用防缓存头。签名密钥轮换后，客户端应重新获取 metadata，避免继续用旧公钥验签。

## Authserver

```text
POST /api/yggdrasil/authserver/authenticate
POST /api/yggdrasil/authserver/refresh
POST /api/yggdrasil/authserver/validate
POST /api/yggdrasil/authserver/invalidate
POST /api/yggdrasil/authserver/signout
```

这些端点处理启动器登录、token 刷新、token 验证、吊销和账号登出。

`authenticate` 支持：

- 邮箱/账号标识登录。
- 在 `yggdrasil_allow_profile_name_login = true` 时用 profile name 登录。
- `clientToken` 由客户端提供；没有提供时服务端生成。
- `selectedProfile` 随登录结果返回。

## Sessionserver

```text
POST /api/yggdrasil/sessionserver/session/minecraft/join
GET  /api/yggdrasil/sessionserver/session/minecraft/hasJoined
GET  /api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}
```

`join` 由 Minecraft 客户端调用，用 access token、selected profile 和 serverId 记录加入会话。

`hasJoined` 由 Minecraft 服务端调用，用 username、serverId 和可选 ip 验证客户端是否完成 join。成功响应里的 textures property 会带数字签名，供服务端验证。

`profile/{uuid}` 查询 profile 属性。`unsigned=false` 时 textures property 会签名；默认 unsigned 行为保持协议兼容。

## Profile Lookup

```text
POST /api/yggdrasil/api/profiles/minecraft
```

按 profile name 批量查询 profile。请求体是名称数组，响应按协议返回匹配项。

## Texture API

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
GET    /api/yggdrasil/textures/{hash}
```

上传和删除需要有效 access token，公开读取按 hash 直接返回处理后的 PNG。

## 协议错误

Yggdrasil 协议端点不会返回：

```json
{ "code": "success", "msg": "", "data": {} }
```

它们会返回协议兼容错误体，例如：

```json
{
  "error": "ForbiddenOperationException",
  "errorMessage": "Invalid token."
}
```

管理端 `/api/v1` 才使用项目统一 envelope。
