---
description: 给 AsterYggdrasil 普通用户和服主看的使用说明：账号、玩家档案、材质、启动器登录和常见边界。
---

# 用户手册

这一页按真实使用流程组织。阅读完整个 Yggdrasil 协议不是前置条件；先理解账号、角色名、皮肤和启动器登录分别在哪里发挥作用即可。

如果你是部署者或服主，建议也先看一遍普通用户流程。许多接入问题来自 profile 未创建、公开 URL 未正确配置，或启动器仍在使用旧 token。

## 登录站点

AsterYggdrasil 使用站点账号作为登录身份。第一次运行时，系统还没有用户，需要先创建第一个管理员账号。

常见情况：

- 系统还没有任何用户：第一次创建的账号会成为管理员。
- 已经有账号：用用户名或邮箱登录。
- 管理员允许公开注册：新用户可以自己注册。
- 管理员启用了注册激活或密码重置：邮件配置必须能正常投递。

站点账号不是 Minecraft profile。一个站点账号下面可以有一个或多个 Minecraft profile，启动器登录后会看到这些 profile。

## 创建玩家档案

Minecraft profile 是进服时使用的玩家身份。它包含协议里的 `id` 和 `name`，也会被用于 skin/cape 的 textures property。

当前用户可用的 profile 操作包括：

```text
GET    /api/v1/profiles/minecraft
POST   /api/v1/profiles/minecraft
PUT    /api/v1/profiles/minecraft/{uuid}/name
GET    /api/v1/profiles/minecraft/{uuid}/textures
DELETE /api/v1/profiles/minecraft/{uuid}
```

创建 profile 时只需要提供名称。名称支持受控改名；改名后 UUID、材质绑定和审计链路保持不变，已绑定旧名称的启动器 token 会暂时失效，重新 refresh 或重新登录即可拿到新名称。

::: warning 不要直接改数据库里的 profile name
直接改名会让启动器缓存、Yggdrasil token、服务端白名单、材质属性和审计记录互相不一致，后续排查成本很高。
:::

## 管理皮肤和披风

材质有两条使用路线。

第一条是“衣柜”路线：先把材质上传到自己的 wardrobe，再把其中一张绑定到某个 profile。这适合网页端管理体验，也适合以后给同一账号下多个 profile 复用材质。

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

第二条是 Yggdrasil/authlib-injector 的直接上传路线。它把材质直接写到目标 profile 上，适合启动器或兼容工具调用。

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
```

上传要求：

- 文件必须是 `image/png`。
- skin 支持 `64x32` 或 `64x64` 的整数倍。
- cape 支持 `64x32` 或 `22x17` 的整数倍。
- 旧式 `22x17` cape 会在存储前补透明到标准画布。
- 服务端只保存重编码后的 PNG，不长期保存原始上传文件。

公开材质读取地址是：

```text
GET /api/yggdrasil/textures/{hash}
```

这个 hash 来自处理后的 PNG 内容。你重新上传同一张处理后完全一致的图片，URL 会稳定命中同一个 hash。

## 用启动器登录

启动器登录走 Yggdrasil authserver：

```text
POST /api/yggdrasil/authserver/authenticate
```

用户填写站点账号的用户名或邮箱，以及站点账号密码。管理员开启 `yggdrasil_allow_profile_name_login` 时，也可以允许用 Minecraft profile name 登录。

登录成功后，启动器会拿到：

- `accessToken`
- `clientToken`
- `availableProfiles`
- `selectedProfile`

如果账号下没有 profile，登录可以成功，但没有能进服的 `selectedProfile`。这时先回站点创建 profile，再重新登录启动器。

## 给 authlib-injector 配地址

协议根路径是：

```text
https://你的域名/api/yggdrasil
```

如果启动器支持 API Location Indication，也可以只填站点根地址。AsterYggdrasil 首页会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

直接使用 `javaagent` 时，写完整协议根路径：

```text
-javaagent:authlib-injector.jar=https://你的域名/api/yggdrasil
```

## 常见问题

### 启动器登录成功但不能进服

先确认账号下是否已经创建 Minecraft profile。没有 profile 就没有可用的 `selectedProfile`。

再确认旧 token 是否还有效。删除 profile 会吊销指向该 profile 的 Yggdrasil token，换名后需要重新登录。

### 皮肤没有显示

先看 profile textures 响应里有没有 textures property，再看 textures property 里的 URL 是否是公网可访问的绝对 URL。

生产环境通常要配置：

```text
public_site_url
```

或高级覆盖项：

```text
yggdrasil_public_base_url
```

如果用了 CDN 或额外域名，还要确认 metadata 的 `skinDomains` 覆盖了材质 URL 的 host。

### 服务端验签失败

让启动器或服务端重新获取 `/api/yggdrasil` metadata。签名 key 轮换后，旧 metadata 里的公钥不能验证新生成的 textures property。

### 能不能改名

可以，但必须走站点 API 或管理员 API。受控改名会保留 profile UUID 和材质绑定，并临时失效已绑定该 profile 的 Yggdrasil token；启动器 refresh 或重新登录后会拿到新名称。不要直接改数据库。
