# 配置和密钥

AsterYggdrasil 的配置分成静态配置和运行时配置。

- 静态配置在 `data/config.toml`，用于数据库、监听地址、cache、texture storage 等启动期配置。
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
enabled = true
backend = "memory"

[texture_storage]
backend = "local"
local_root = "textures"
```

相对路径按 `data/config.toml` 所在目录解析。默认 `local_root = "textures"` 会落到 `data/textures`。

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
yggdrasil_signature_public_key
yggdrasil_signature_private_key
```

管理端通过 Admin Config API 修改：

```text
GET    /api/v1/admin/config
GET    /api/v1/admin/config/schema
PUT    /api/v1/admin/config/{key}
DELETE /api/v1/admin/config/{key}
POST   /api/v1/admin/config/yggdrasil/action
```

配置写入会经过类型化 normalizer/validator。不要绕过服务层直接写 `system_config`。

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
