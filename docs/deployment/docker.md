# Docker 部署

这一页描述 AsterYggdrasil 作为皮肤站和 Yggdrasil 认证服务器的生产部署关注点。

如果尚未梳理上线检查顺序，请先看 [部署总览](/deployment/)。这页聚焦 Docker 场景下需要挂载和配置的内容。

## 持久化目录

容器内运行状态应挂载到 `/data`。至少需要持久化：

- `config.toml`
- SQLite 数据库或外部数据库连接配置。
- local texture storage 目录。
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

[texture_storage]
backend = "local"
local_root = "textures"

[cache]
enabled = true
backend = "memory"
```

如果 `config.toml` 位于 `/data/config.toml`，`local_root = "textures"` 会解析为 `/data/textures`。

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
- `/data/textures`。
- `data/config.toml` 或对应 secret/config 管理记录。

数据库和 texture storage 必须作为一组备份。只恢复其中一个会导致 storage consistency check 报 missing object 或 orphan object。
