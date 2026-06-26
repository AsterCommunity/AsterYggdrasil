---
description: AsterYggdrasil 使用指南总览，按第一次运行、玩家日常、启动器接入、管理员维护和项目参考组织。
---

# 使用指南

这一组文档按“你现在要做什么”组织，不要求你先理解所有协议细节。

如果你是第一次来，先走 [快速开始](./getting-started)。如果服务已经跑起来，按自己的角色直接跳到对应入口。

## 第一次运行

你只想先把服务跑起来，看这几篇：

- [快速开始](./getting-started)：启动后端、创建第一个管理员、验证 metadata、profile 和材质路径
- [部署总览](/deployment/)：上线前检查公开 URL、反向代理、持久化、备份和验收
- [Docker 部署](/deployment/docker)：准备正式上线、挂 HTTPS、配置持久化和反向代理时看
- [配置和密钥](./configuration)：确认公开 URL、skinDomains、上传策略和签名密钥

## 玩家日常使用

服务已经能打开后，普通用户优先看这里：

- [用户手册](./user-guide)：账号、Minecraft profile、皮肤/披风、启动器登录和常见问题
- [玩家档案](./profiles)：profile 名称、UUID、改名、删除和管理员查看
- [材质处理](./yggdrasil-textures)：wardrobe、profile 绑定、直接上传、PNG 校验和公开读取
- [常见问题速查](./faq)：短问题直接找答案

## 启动器和服务端接入

如果你在接 authlib-injector、启动器或 Minecraft 服务端，看这几篇：

- [启动器填写](./launcher-setup)：玩家和服主应该填哪个地址、账号和 javaagent 参数
- [启动器登录](./launcher-login)：authenticate、refresh、selectedProfile、token 生命周期和 join/hasJoined
- [Yggdrasil 转发](./yggdrasil-forwarding)：让服务端统一接入 AY，并按优先级转发 Yggdrasil 兼容站点、Mojang 等上游 session 验证
- [Yggdrasil API](./yggdrasil-api)：ALI、metadata、authserver、sessionserver、profile lookup、texture API 和协议错误
- [材质处理](./yggdrasil-textures)：textures property URL、skinDomains、签名和缓存相关问题
- [故障排查](./troubleshooting)：启动器登录、进服、皮肤显示、验签和材质 404

## 管理员维护

管理员要先分清三类东西：启动配置、运行时配置、协议对外表现。

- [管理员指南](./admin-guide)：用户、profile、材质、配置、审计和任务分别负责什么
- [能力封禁](./user-bans)：按 scope 限制用户使用 Yggdrasil、profile、材质上传和公共材质库交互能力
- [配置和密钥](./configuration)：`config.toml`、`system_config`、公开 URL、签名 key rotate
- [对象存储](./storage)：local/S3/MinIO backend、保存路径、公开 URL 和一致性检查
- [审计与后台任务](./audit-tasks)：审计范围、runtime task、多实例边界和维护建议
- [部署总览](/deployment/) 和 [Docker 部署](/deployment/docker)：持久化目录、反向代理、trusted proxies、多实例和备份

## 项目本身

想知道 AsterYggdrasil 为什么这样设计、适合谁、不适合谁，看 [关于 AsterYggdrasil](./about)。

准备修改文档前，看 [文档贡献说明](./docs-contributing)。这里的文档是给真实部署者、玩家和服主看的，不是把源码目录改写成文章。
