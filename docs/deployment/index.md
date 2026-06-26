---
description: AsterYggdrasil 部署总览，按上线前准备、公开 URL、反向代理、持久化、备份和验收组织。
---

# 部署总览

::: tip 这一篇覆盖什么
这页用于梳理上线前检查项。具体 Docker 配置看 [Docker 部署](/deployment/docker)。
:::

## 入口速查

| 你想做什么 | 去哪里 |
| --- | --- |
| 先在本机跑通 | [快速开始](/guide/getting-started) |
| 用 Docker 上线 | [Docker 部署](/deployment/docker) |
| 配公开访问地址 | [配置和密钥](/guide/configuration) |
| 排查启动器、皮肤、进服问题 | [故障排查](/guide/troubleshooting) |
| 理解材质和上传头像保存在哪里 | [对象存储](/guide/storage) |

## 上线前先确认

生产部署不能只验证容器启动状态。AsterYggdrasil 还要被启动器、Minecraft 客户端和服务端访问，所以公开地址必须真实可达。

上线前至少确认：

- 站点可以通过 HTTPS 域名访问。
- `/api/yggdrasil` 返回 metadata。
- 首页 `/` 保留 `X-Authlib-Injector-API-Location` 响应头。
- `public_site_url` 或 `yggdrasil_public_base_url` 能生成客户端可访问的材质 URL。
- `skinDomains` 覆盖材质 URL 的 host。
- 数据库、`config.toml` 和 object storage backend 已经持久化或可恢复。

## 推荐部署路径

1. 先按 [快速开始](/guide/getting-started) 本地跑通账号、profile 和材质。
2. 准备域名和 HTTPS。
3. 按 [Docker 部署](/deployment/docker) 挂载 `/data`。
4. 配置 `public_site_url`，需要高级路径时再配 `yggdrasil_public_base_url`。
5. 用真实启动器登录一次，并进一次测试服务器。
6. 备份数据库、`config.toml` 和 object storage backend。

## 公开 URL 是核心

材质 URL 会写进 Yggdrasil `textures` property。客户端拿到的必须是绝对 URL，例如：

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

如果这个地址只能在服务器本机访问，启动器和 Minecraft 客户端就无法加载皮肤。此时应优先检查公开 URL 和反向代理配置。

## 多实例边界

周期维护任务、邮件 outbox、审计清理和材质一致性检查都带有全局副作用。当前文档只覆盖单实例部署。

如果生产环境需要多实例，请先在外层部署保证只有一个实例启动完整后台任务。后续多实例高可用应由 Forge runtime 的租约/锁机制承载，不应在 Yggdrasil 里维护独立主从分支。

## 备份对象

至少备份三类数据：

- 数据库。
- `config.toml` 或等价的 secret/config 管理记录。
- object storage backend。local backend 默认类似 `data/storage`；S3/MinIO backend 需要备份桶内对象和对应配置。

数据库和 object storage 要作为一组恢复。只恢复数据库会出现 missing object；只恢复对象存储会出现 orphan object。
