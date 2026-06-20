# Docker 部署

这一页描述 AsterYggdrasil 作为皮肤站和 Yggdrasil 认证服务器的生产部署关注点。

如果尚未梳理上线检查顺序，请先看 [部署总览](/deployment/)。这页聚焦 Docker 场景下需要挂载和配置的内容。

## 持久化目录

容器内运行状态应挂载到 `/data`。至少需要持久化：

- `config.toml`
- SQLite 数据库或外部数据库连接配置。
- local object storage 目录。
- 运行时临时目录和日志目录，如果配置启用。

示例静态配置：

```toml
[server]
host = "0.0.0.0"
port = 3000
start_mode = "primary"
temp_dir = ".tmp"

[database]
url = "sqlite://asteryggdrasil.db?mode=rwc"

[object_storage]
backend = "local"
local_root = "storage"

[cache]
backend = "memory"
```

如果 `config.toml` 位于 `/data/config.toml`，`local_root = "storage"` 会解析为 `/data/storage`。材质和用户上传头像都会写入这个对象存储目录。

如果使用 S3 或 MinIO，对象不会写入 `/data/storage`，但数据库和 `config.toml` 仍然必须持久化。示例：

```toml
[object_storage]
backend = "s3"

[object_storage.s3]
endpoint = "https://s3.example.com"
region = "auto"
bucket = "asteryggdrasil"
base_path = "production"
access_key_id = "..."
secret_access_key = "..."
force_path_style = false
```

材质和上传头像都会走同一个 object storage backend。S3/MinIO 上传由服务端 streaming 完成，不需要给浏览器开放 presigned 上传。

## 反向代理

生产环境通常通过 Nginx、Caddy 或 Traefik 暴露 HTTPS。必须保证外部访问路径和运行时配置一致：

```text
https://skin.example.com/api/yggdrasil
```

对应运行时配置：

```json
yggdrasil_public_base_url = ["https://skin.example.com/api/yggdrasil"]
yggdrasil_skin_domains = ["skin.example.com"]
```

authlib-injector 会检查材质 URL host 是否在 `skinDomains` 中。public base URL 和 skinDomains 不一致时，启动器或服务端可能拒绝材质。

如果 S3 bucket 或前置 CDN 允许公开读，可以额外配置运行时 `yggdrasil_texture_public_base_url`，让已上传材质的 URL 直接指向对象存储/CDN。此时 bucket/CDN 需要允许站点来源匿名 `GET`/`HEAD` 读取；默认皮肤仍走 Yggdrasil API。

## ALI

站点首页会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

反向代理不要删除这个响应头。这样用户可以在启动器里填站点根地址，由启动器自动发现 Yggdrasil API。

## trusted proxies

如果服务在反向代理后面运行，需要配置可信代理，避免信任客户端伪造的 forwarded headers。

```toml
[network_trust]
trusted_proxies = ["127.0.0.1"]
```

实际值应填写代理到应用之间的来源地址或网段。

## 多实例

周期维护任务应该只在一个 primary 节点运行：

```toml
[server]
start_mode = "primary"
```

其他实例使用 follower 模式，避免重复执行清理、邮件 outbox、后台任务 dispatch 等全局任务。

## 签名 key

首次启动会确保 Yggdrasil 签名私钥和公钥存在。生产环境应通过管理端 config action 轮换 key，而不是直接编辑私钥：

```text
POST /api/v1/admin/config/yggdrasil/action
```

轮换后，客户端和服务端可能需要重新获取 metadata。

## 备份

至少备份：

- 数据库。
- `/data/storage`。
- `data/config.toml` 或对应 secret/config 管理记录。

数据库和 object storage 必须作为一组备份。只恢复其中一个会导致 storage consistency check 报 missing object 或 orphan object。
