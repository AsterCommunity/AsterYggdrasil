---
description: AsterYggdrasil 故障排查，按启动、登录、进服、材质、签名、公开 URL 和存储一致性问题分流。
---

# 故障排查

::: tip 先按症状找
这页按用户看到的现象排查，不按源码模块排查。出现错误时，先确认公开 URL、profile、token 和材质路径；不要直接修改数据库。
:::

## 速查表

| 现象 | 优先检查 |
| --- | --- |
| 启动器找不到认证服务器 | 地址是否填 `/api/yggdrasil`，或 ALI 响应头是否被代理移除 |
| 登录成功但不能进服 | 账号是否有 profile，token 是否指向已删除或改名的 profile |
| 皮肤或披风不显示 | `textures` property 里的 URL 是否公网可访问，`skinDomains` 是否覆盖 host |
| 服务端验签失败 | 服务端是否缓存旧 metadata，签名 key 是否刚轮换 |
| 材质 404 | hash 是否存在，storage 文件是否被删，公开 URL path 是否正确 |
| 一致性检查失败 | 数据库和材质目录是否只恢复了一半，或有人直接删了 storage 文件 |

## 启动器找不到认证服务器

先确认启动器填的是公开地址，不是内网地址：

```text
https://skin.example.com/api/yggdrasil
```

如果启动器支持 API Location Indication，也可以填：

```text
https://skin.example.com
```

然后访问站点首页，确认响应头存在：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

如果没有这个头，检查反向代理是否删除了自定义响应头。

## 启动器登录失败

按顺序检查：

1. 用站点账号用户名或邮箱登录，不要把 Minecraft profile name 当成账号，除非管理员启用了 `yggdrasil_allow_profile_name_login`。
2. 确认账号密码能在站点登录。
3. 如果注册激活或密码重置依赖邮件，确认邮件配置可用。
4. 如果短时间多次失败，等限流窗口过去再试。

协议登录走：

```text
POST /api/yggdrasil/authserver/authenticate
```

这个端点返回 Yggdrasil 协议错误，不返回项目统一 envelope。

## 启动器登录成功但不能进服

最常见原因是账号下没有 Minecraft profile。登录账号和进服身份不是一回事。

先让用户在站点创建 profile，再重新登录启动器。

如果 profile 刚改名：

- 旧 token 会被临时失效。
- 启动器需要 refresh 或重新登录。
- profile UUID 不变，材质绑定不变。

如果 profile 被删除，关联 token 会吊销。需要重新创建 profile 并重新登录。

## 皮肤或披风不显示

先查 `profile/{uuid}` 或 `hasJoined` 响应里的 `textures` property。解码后应该能看到类似：

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

继续检查：

- URL 是否是绝对 URL。
- 浏览器或客户端机器是否能访问这个 URL。
- metadata `skinDomains` 是否包含 `skin.example.com`。
- CDN 或反向代理是否把 `/api/yggdrasil/textures/{hash}` 转发到应用。

生产环境通常要配置：

```text
public_site_url
yggdrasil_public_base_url
yggdrasil_skin_domains
```

普通部署优先配 `public_site_url`。只有 API 暴露在特殊路径或单独域名时，再配 `yggdrasil_public_base_url`。

## 服务端验签失败

先让启动器或服务端重新获取：

```text
GET /api/yggdrasil
```

metadata 里有 `signaturePublickey`。签名 key 轮换后，旧 metadata 里的公钥不能验证新生成的 textures property。

如果只在某些客户端失败，通常是缓存问题。如果所有客户端都失败，再检查管理端是否刚执行过签名 key rotate，以及反向代理是否缓存了 metadata。

## 材质上传失败

上传只接受 PNG。常见失败原因：

- MIME 不是 `image/png`。
- skin 尺寸不是 `64x32` 或 `64x64` 的整数倍。
- cape 尺寸不是 `64x32` 或 `22x17` 的整数倍。
- 文件超过 `yggdrasil_max_texture_upload_bytes`。
- 像素数超过 `yggdrasil_max_texture_pixels`。
- 管理员关闭了 skin 或 cape 上传。

旧式 `22x17` cape 会被补透明到标准画布，不需要用户手动改图。

## 材质 404 或一致性检查失败

公开读取路径是：

```text
GET /api/yggdrasil/textures/{hash}
```

如果返回 404：

- 确认 hash 来自当前 profile textures property。
- 确认 local texture storage 目录已经挂载。
- 确认没有直接删除 storage 文件。
- 确认数据库和材质目录是同一时间点的备份。

如果 `yggdrasil-storage-consistency-check` 报错，先不要跑清理。先确认是不是恢复备份漏了材质目录，或者有人绕过服务层手工删文件。
