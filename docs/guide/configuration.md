# 配置和密钥

AsterYggdrasil 的配置分成静态配置和运行时配置。

- 静态配置在 `data/config.toml`，用于数据库、监听地址、cache、object storage 等启动期配置。
- 运行时配置在 `system_config` 表，适合 Yggdrasil 策略、公开 URL、上传开关、token 策略和签名 key。

## 静态配置

静态配置示例：

```toml
[server]
host = "127.0.0.1"
port = 3000
temp_dir = ".tmp"
start_mode = "primary"

[database]
url = "sqlite://asteryggdrasil.db?mode=rwc"

[cache]
backend = "memory"

[object_storage]
backend = "local"
local_root = "storage"
```

相对路径按 `data/config.toml` 所在目录解析。默认 `local_root = "storage"` 会落到 `data/storage`。材质和用户上传头像都走这个对象存储配置。

## Yggdrasil 运行时配置

常用 key：

```text
yggdrasil_server_name
yggdrasil_allow_profile_name_login
yggdrasil_allow_skin_upload
yggdrasil_allow_cape_upload
yggdrasil_token_ttl_days
yggdrasil_max_active_tokens
yggdrasil_max_texture_upload_bytes
yggdrasil_max_texture_pixels
yggdrasil_skin_domains
yggdrasil_public_base_url
yggdrasil_texture_public_base_url
yggdrasil_signature_public_key
yggdrasil_signature_private_key
```

公共材质库也使用运行时配置：

```text
texture_library_enabled
texture_library_review_required
```

公开认证流程的图形验证码也使用运行时配置：

```text
auth_captcha_enabled
auth_captcha_login_required
auth_captcha_register_required
auth_captcha_invitation_accept_required
auth_captcha_register_activation_resend_required
auth_captcha_preset
auth_captcha_ttl_secs
auth_captcha_length
auth_captcha_max_attempts
```

头像相关设置也走运行时配置：

```text
gravatar_base_url
```

`gravatar_base_url` 控制 Gravatar 头像 URL 的基础地址，默认是 `https://www.gravatar.com/avatar`。
可以改成镜像地址，例如 `https://cravatar.cn/avatar`。保存时会去掉首尾空白和尾部斜杠；
非空值必须是带 host 的 `http`/`https` base URL，不能包含 query 或 fragment。空值会回退默认地址。

管理端通过 Admin Config API 修改：

```text
GET    /api/v1/admin/config
GET    /api/v1/admin/config/schema
PUT    /api/v1/admin/config/{key}
DELETE /api/v1/admin/config/{key}
POST   /api/v1/admin/config/yggdrasil/action
```

配置写入会经过类型化 normalizer/validator。不要绕过服务层直接写 `system_config`。

## 公共材质库配置

`texture_library_enabled` 控制公共材质库总开关。关闭后，公共材质库入口、列表、详情、复制和举报流程都应被前端隐藏或禁用；后端公共材质库接口也会拒绝相关业务请求。

`texture_library_review_required` 控制发布是否需要审核：

- `true`: 用户提交公开材质后进入 `pending_review`，需要管理员审核通过才会出现在公共库。
- `false`: 用户提交公开材质后直接进入 `published`。

这两个配置会通过 public frontend config 返回给前端，用于控制公共入口、Footer 链接和用户侧材质库操作。不要只在前端写死开关，后端配置仍然是最终判断依据。

## 图形验证码配置

`auth_captcha_enabled` 是总开关。启用后，可以分别要求以下公开认证流程完成验证码：

- 本地密码登录。
- 自助注册。
- 接受邀请创建账号。
- 重发注册激活邮件。

验证码挑战由服务端生成，前端通过公开 captcha policy 判断当前流程是否需要显示验证码。验证码渲染强度由 `auth_captcha_preset` 控制，可选值是 `readable`、`balanced`、`hardened`。

管理端可以通过 config action 预览当前验证码渲染效果：

```text
POST /api/v1/admin/config/auth_captcha/action
```

## public base URL

普通部署优先配置 `public_site_url`：

```json
["https://skin.example.com"]
```

未单独配置 `yggdrasil_public_base_url` 时，服务端会从第一个有效 `public_site_url` 派生 Yggdrasil API 和 texture URL：

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

`yggdrasil_public_base_url` 是高级覆盖项，也是 JSON 字符串数组，例如：

```json
["https://skin.example.com/api/yggdrasil"]
```

配置后服务端优先使用第一个有效 http/https URL 生成 texture URL。base URL 可以带 path，最终会拼成：

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

如果 `yggdrasil_public_base_url` 和 `public_site_url` 都没有可用值，Yggdrasil profile textures 响应会返回配置错误；协议响应不会输出相对 texture URL。

`yggdrasil_texture_public_base_url` 是对象存储/CDN 直链覆盖项，适用于 S3 bucket 或 CDN 公开读、服务端私有写的部署。它是普通字符串，不是数组：

```text
https://cdn.example.com/env/production/textures
```

配置后，已上传材质会用 `{yggdrasil_texture_public_base_url}/{storage_key}`，例如 `https://cdn.example.com/env/production/textures/ab/abcdef...png`。默认皮肤不在对象存储里，仍通过 Yggdrasil API URL 返回。

如果这个 URL 指向 S3 bucket 或 CDN，前端材质预览会从该域名直接加载图片。bucket/CDN 必须允许 `public_site_url` 中对外服务的站点来源执行匿名 `GET`/`HEAD` CORS 读取。这里不需要开放浏览器上传 CORS，因为上传始终由服务端 streaming 到对象存储。

## skinDomains

`yggdrasil_skin_domains` 也是 JSON 字符串数组，表示额外材质域名白名单。authlib-injector 会校验材质 URL 的 host 是否在 metadata `skinDomains` 中。

规则可以是：

- 精确域名，例如 `skin.example.com`。
- 点前缀域名，例如 `.example.com`。

metadata 响应会自动包含 Mojang 官方域名 `.minecraft.net`、`.mojang.com`，以及当前有效 texture URL 的 host。用户只需要在允许额外 CDN 或外部材质域名时配置 `yggdrasil_skin_domains`。

## 签名密钥

authlib-injector 要求服务端对部分 profile properties 签名：

- `hasJoined` 响应。
- `profile/{uuid}?unsigned=false` 响应。

AsterYggdrasil 使用 RSA 私钥生成签名，metadata 中公开公钥。

私钥不能通过普通 config set API 直接修改。轮换 key 使用 config action：

```text
POST /api/v1/admin/config/yggdrasil/action
```

action type：

```text
rotate_yggdrasil_signature_key
```

轮换后：

- 新生成的 textures property 会使用新私钥签名。
- metadata 会派生并返回新公钥。
- 旧 token 不需要重发；签名是每次生成 profile property 时计算的，不是持久存储在 token 里。
- 如果启动器或服务端缓存了旧 metadata，可能短时间验签失败，应重新获取 metadata。

## 敏感配置

`yggdrasil_signature_private_key` 是敏感配置：

- 不允许前端直接改。
- 不应出现在普通 API 响应、audit details 或错误信息中。
- 需要变更时只能走 rotate action。

`yggdrasil_signature_public_key` 主要作为没有私钥时的 fallback 语义。正常情况下应以 private key 派生出的 public key 为准。
