---
description: AsterYggdrasil 常见问题速查，覆盖账号、profile、启动器、材质、部署和当前功能边界。
---

# 常见问题速查

## AsterYggdrasil 是正版在线模式替代品吗？

不是。它面向自建 Yggdrasil/authlib-injector 接入，让你托管自己的站点账号、Minecraft profile 和材质。Mojang 官方在线模式服务器不是这个项目的目标。

## 站点账号和 Minecraft profile 是什么关系？

站点账号负责登录网站和启动器认证。Minecraft profile 是进服时看到的玩家身份，包含 profile name、UUID 和材质属性。

一个站点账号可以拥有多个 Minecraft profile。

## 能不能改 profile name？

可以。必须走用户或管理员 API。受控改名会保留 UUID、材质绑定和审计链路，并临时失效已绑定该 profile 的 Yggdrasil token。

不要直接改数据库。

## 删除 profile 会发生什么？

删除会处理：

- profile 记录。
- 绑定到 profile 的材质记录。
- 不再被引用的材质对象。
- 指向该 profile 的 Yggdrasil token。
- 审计日志。

## 为什么皮肤 URL 必须是公网绝对地址？

Minecraft 客户端和服务端运行在应用进程之外。它们只能访问 `textures` property 里的 URL，所以这个 URL 必须能从客户端机器访问。

本机测试可以先跳过，生产部署必须配置 `public_site_url` 或 `yggdrasil_public_base_url`。

## `skinDomains` 是干什么的？

authlib-injector 会检查材质 URL 的 host 是否在 metadata `skinDomains` 里。AsterYggdrasil 会自动包含有效材质 URL 的 host，额外 CDN 或外部材质域名才需要配置 `yggdrasil_skin_domains`。

## S3 或 MinIO 能用吗？

当前生产可用 texture storage backend 是 local。S3/MinIO 配置形状已经预留，但后端实现还没有接入。

## 能不能直接删材质文件？

不建议。材质对象和数据库记录通过 hash 与引用计数关联。直接删 storage 文件会导致公开读取 404 和一致性检查失败。

删除 profile、解绑材质或删除 hash 时都应该走 API。

## 签名 key 轮换后要重登吗？

通常不需要。签名是在生成 profile properties 时计算的，不存进 token。轮换后客户端或服务端可能需要重新获取 `/api/yggdrasil` metadata，拿到新的 `signaturePublickey`。

## 启动器能登录但没有角色怎么办？

账号下没有 Minecraft profile。先登录站点创建 profile，再让启动器重新登录。

## 我应该先看哪篇？

- 第一次运行：[快速开始](/guide/getting-started)
- 玩家操作：[用户手册](/guide/user-guide)
- 启动器填写：[启动器填写](/guide/launcher-setup)
- 生产部署：[部署总览](/deployment/)
- 出问题：[故障排查](/guide/troubleshooting)
