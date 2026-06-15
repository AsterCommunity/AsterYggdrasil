---
layout: home
description: AsterYggdrasil 是自建 Minecraft 皮肤站与 Yggdrasil/authlib-injector 认证服务器。

hero:
  name: AsterYggdrasil
  text: Minecraft 皮肤站 + 认证服务器
  tagline: 把玩家账号、Minecraft profile、皮肤/披风材质和 authlib-injector/Yggdrasil 接入放回你自己的服务里。
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: 用户手册
      link: /guide/user-guide
    - theme: alt
      text: 关于项目
      link: /guide/about

features:
  - title: Yggdrasil API
    details: "`/api/yggdrasil` 提供 metadata、authserver、sessionserver、profile lookup 和公开材质读取。"
    link: /guide/yggdrasil-api
  - title: 启动器登录
    details: 复用站点账号登录，返回 accessToken/clientToken、availableProfiles 和 selectedProfile。
    link: /guide/launcher-login
  - title: 玩家档案
    details: Minecraft profile 独立建模；名称创建后不可改，删除会清理材质绑定并吊销相关 token。
    link: /guide/profiles
  - title: 材质系统
    details: 支持 wardrobe 材质、profile 绑定、skin/cape 上传、PNG 重编码、旧式 cape 兼容和 hash URL。
    link: /guide/yggdrasil-textures
  - title: 配置和密钥
    details: "Yggdrasil 策略、公开 URL、skinDomains 和签名密钥走运行时配置，私钥通过 action 轮换。"
    link: /guide/configuration
  - title: 维护任务
    details: runtime task 负责 token 清理、材质对象清理、存储一致性检查、审计清理和 task artifact 清理。
    link: /guide/audit-tasks
---

AsterYggdrasil 是自建 Minecraft 皮肤站和 Yggdrasil/authlib-injector 认证服务器。它不是“给 README 凑个名词”的模板项目，当前后端已经有账号、profile、Yggdrasil 协议、材质处理、配置、审计和维护任务这些基础能力。

如果你是第一次部署，先看 [快速开始](./guide/getting-started.md)。如果你是普通玩家或服主，先看 [用户手册](./guide/user-guide.md)。如果你想知道项目为什么这么设计，看 [关于 AsterYggdrasil](./guide/about.md)。

## 核心入口

协议根路径是：

```text
/api/yggdrasil
```

站点首页会返回 `X-Authlib-Injector-API-Location: /api/yggdrasil/`，支持 authlib-injector 的启动器可以通过 ALI 从站点首页发现 API 地址。

常用公开端点：

```text
GET  /api/yggdrasil
POST /api/yggdrasil/authserver/authenticate
POST /api/yggdrasil/authserver/refresh
POST /api/yggdrasil/sessionserver/session/minecraft/join
GET  /api/yggdrasil/sessionserver/session/minecraft/hasJoined
GET  /api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}
GET  /api/yggdrasil/textures/{hash}
```

站点和管理 API 在 `/api/v1` 下，例如账号、profile、wardrobe 材质、配置、审计和后台任务。

## 推荐阅读顺序

1. [关于项目](./guide/about.md)：先搞清楚它适合谁、不适合谁。
2. [快速开始](./guide/getting-started.md)：本地启动、创建首个管理员、确认 metadata 和材质路径。
3. [用户手册](./guide/user-guide.md)：账号、profile、材质、启动器登录的实际流程。
4. [Yggdrasil API](./guide/yggdrasil-api.md)：ALI、metadata、authserver、sessionserver 和协议错误。
5. [玩家档案](./guide/profiles.md)：profile 创建、删除、不可改名和管理 API。
6. [材质处理](./guide/yggdrasil-textures.md)：wardrobe、绑定、skin/cape 上传、hash、公开读取和 skinDomains。
7. [配置和密钥](./guide/configuration.md)：public URL、运行时配置和签名 key rotate。
8. [审计与后台任务](./guide/audit-tasks.md)：管理员可见的审计、runtime task 和维护策略。
9. [Docker 部署](./deployment/docker.md)：反向代理、HTTPS、持久化和备份。

## 当前边界

- Minecraft profile name 创建后不可改名。需要换名时删除并重新创建 profile。
- profile 禁用不在当前版本里直接加字段，后续会通过统一封禁系统定义登录、join、hasJoined 和材质访问语义。
- 当前 texture storage 生产可用 backend 是 local。S3/minio 配置形状已预留，但 backend 实现还需要后续接入。
- 前端管理面板仍在演进，文档优先描述当前稳定后端能力和真实可部署语义。
