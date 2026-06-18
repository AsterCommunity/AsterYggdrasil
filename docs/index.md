---
layout: home
description: AsterYggdrasil 文档首页，按快速开始、玩家使用、启动器接入、管理员配置和部署维护组织。

hero:
  name: AsterYggdrasil
  text: 官方文档中心
  tagline: 自建 Minecraft 皮肤站与 Yggdrasil/authlib-injector 认证服务器，从本地跑通到真实启动器接入都按一条路径讲清楚。
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: 使用指南
      link: /guide/
    - theme: alt
      text: Docker 部署
      link: /deployment/

features:
  - title: 第一次运行
    details: 先启动服务、创建第一个管理员，再验证 metadata、公开 URL 和材质读取路径。
    link: /guide/getting-started
  - title: 玩家日常使用
    details: 按账号、Minecraft profile、wardrobe、皮肤/披风和启动器登录的真实顺序组织。
    link: /guide/user-guide
  - title: 启动器与服务端接入
    details: 先讲启动器该填什么，再说明 API Location Indication、authserver、sessionserver、材质 URL 和签名验证。
    link: /guide/launcher-setup
  - title: 管理员配置
    details: 分清用户、profile、材质、静态配置、运行时配置、审计和后台任务各自负责什么。
    link: /guide/admin-guide
  - title: 材质系统
    details: 覆盖 wardrobe、profile 绑定、Yggdrasil 直接上传、PNG 重编码、hash URL 和本地存储。
    link: /guide/yggdrasil-textures
---

## 先认识它

AsterYggdrasil 是自托管的 Minecraft 皮肤站和 Yggdrasil/authlib-injector 认证服务器。它让站点账号、Minecraft profile、skin/cape 材质、启动器登录和服务端进服验证由部署者自己的服务承载。

当前代码已经包含账号认证、外部认证、Minecraft profile、wardrobe 材质库、Yggdrasil 协议端点、材质处理、运行时配置、审计日志和后台维护任务。文档会按这些已经存在的能力写，不把还没落地的路线图写成用户能直接使用的功能。

## 按目的走

### 我只是想先跑起来

从 [快速开始](/guide/getting-started) 走一遍。它会带你启动后端、创建第一个管理员、确认 `/api/yggdrasil` metadata、创建 profile，并验证材质上传和公开读取路径。

准备正式上线时，再看 [部署总览](/deployment/) 和 [Docker 部署](/deployment/docker)。生产环境常见问题集中在公开 URL、反向代理响应头、签名公钥缓存和材质目录备份，建议逐项确认。

### 我是玩家，想知道怎么用

从 [使用指南](/guide/) 进入。普通用户优先看 [用户手册](/guide/user-guide)，里面按账号登录、创建 Minecraft profile、管理皮肤/披风、启动器登录和常见问题组织。

如果只关心角色名、UUID、改名或删除，直接看 [玩家档案](/guide/profiles)。如果只关心皮肤和披风，看 [材质处理](/guide/yggdrasil-textures)。

### 我要接启动器或服务器

先看 [启动器填写](/guide/launcher-setup)，再看 [启动器登录](/guide/launcher-login) 和 [Yggdrasil API](/guide/yggdrasil-api)。启动器通常需要协议根路径：

```text
https://你的域名/api/yggdrasil
```

支持 API Location Indication 的启动器也可以填写站点根地址。站点首页会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

### 我要管理一个实例

先看 [管理员指南](/guide/admin-guide) 和 [配置和密钥](/guide/configuration)，确认 `public_site_url`、`yggdrasil_public_base_url`、`yggdrasil_skin_domains` 和签名密钥轮换。再看 [审计与后台任务](/guide/audit-tasks)，了解 token 清理、材质一致性检查和管理员审计。

遇到启动器登录、进服、皮肤显示或验签问题，直接看 [故障排查](/guide/troubleshooting)。短问题看 [常见问题速查](/guide/faq)。

材质保存位置看 [材质存储](/guide/storage)。当前生产可用 storage backend 是 local；S3/MinIO 配置形状已预留，但后端实现还没接入。

### 我准备改文档

先看 [文档贡献说明](/guide/docs-contributing)。这份文档给真实用户看，不是给源码模块做目录索引。新增页面前先问一句：读者打开这页是为了完成什么任务？

## 当前边界

- Minecraft profile name 支持受控改名，必须走用户或管理员 API；不要直接改数据库。
- 删除 profile 会处理材质绑定、引用计数、相关 Yggdrasil token 和审计记录。
- Yggdrasil 协议端点返回协议格式；站点和管理 API 才返回 `{ "code": "success", "msg": "", "data": ... }`。
- 当前生产可用材质存储后端是 local。S3/MinIO 只是预留配置形状。
- 前端管理面板仍在演进；文档优先描述稳定后端能力和真实可部署语义。
