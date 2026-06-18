---
description: 面向玩家和服主的启动器填写指南，说明 API 地址、ALI、javaagent、登录信息和常见验证点。
---

# 启动器填写

::: tip 这一篇覆盖什么
这页只讲启动器和服务端该填什么地址。协议字段细节看 [启动器登录](/guide/launcher-login) 和 [Yggdrasil API](/guide/yggdrasil-api)。
:::

## 先拿到正确地址

管理员应给玩家一个公开站点地址，例如：

```text
https://skin.example.com
```

Yggdrasil API 根路径通常是：

```text
https://skin.example.com/api/yggdrasil
```

如果启动器支持 API Location Indication，可以填写站点根地址。AsterYggdrasil 首页会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

不支持 ALI 的启动器就填写完整 `/api/yggdrasil` 地址。

## 玩家登录前要做什么

玩家至少需要：

- 一个站点账号。
- 站点账号密码。
- 账号下已经创建好的 Minecraft profile。

如果账号下没有 Minecraft profile，启动器可能能登录账号，但没有能进服的 `selectedProfile`。先回站点创建 profile。

## 启动器填写速查

| 启动器能力 | 填写方式 |
| --- | --- |
| 支持 ALI | 填 `https://skin.example.com` |
| 不支持 ALI | 填 `https://skin.example.com/api/yggdrasil` |
| 需要账号 | 填站点账号用户名或邮箱 |
| 允许 profile name 登录 | 管理员开启 `yggdrasil_allow_profile_name_login` 后才可用 |

不同启动器的界面名字可能不同，常见叫法包括“认证服务器”、“Yggdrasil API”、“第三方登录”、“authlib-injector 服务器”。本质上都是填同一个协议根路径。

## javaagent 写法

如果你直接使用 authlib-injector `javaagent`：

```text
-javaagent:authlib-injector.jar=https://skin.example.com/api/yggdrasil
```

服务端也应保持在线模式：

```properties
online-mode=true
enforce-secure-profile=true
```

如果临时关闭了安全档案验证，请先确认影响范围。排障用的临时配置不应长期保留在生产环境中。

## 登录后检查

登录成功后，启动器应拿到：

- `accessToken`
- `clientToken`
- `availableProfiles`
- `selectedProfile`

如果能登录但不能进服，看 [故障排查](/guide/troubleshooting#启动器登录成功但不能进服)。

如果能进服但皮肤不显示，看 [故障排查](/guide/troubleshooting#皮肤或披风不显示)。
