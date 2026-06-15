---
description: AsterYggdrasil 为什么存在、现在能做什么、适合谁，以及当前版本不应该承诺什么。
---

# 关于 AsterYggdrasil

::: tip 这一页不是接入手册
如果你只是想先把服务跑起来，去 [快速开始](./getting-started)。

如果你想知道这个项目现在到底是什么、能不能拿来给自己的 Minecraft 服务器用，继续往下看。
:::

## 它解决什么问题

AsterYggdrasil 是一个自托管的 Minecraft 皮肤站和 Yggdrasil/authlib-injector 认证服务器。

简单说，它让你可以把“账号登录、玩家档案、皮肤/披风材质、启动器认证、服务端进服验证”放在自己的服务里，而不是依赖第三方皮肤站或临时拼出来的脚本。

它现在已经不是空壳。当前代码里已经有：

- 站点账号的注册、登录、刷新、退出和管理员初始化。
- `/api/yggdrasil` 协议根路径，包含 metadata、authserver、sessionserver、profile lookup 和公开材质读取。
- Minecraft profile 独立建模，一个站点账号可以拥有多个 profile。
- skin/cape 上传、PNG 重编码、旧式 cape 兼容、hash 公开读取和本地材质存储。
- 运行时配置、签名密钥轮换、审计日志和周期维护任务。

这些能力是后端真实存在的，不是路线图上的愿望。文档会尽量按这个边界写，避免把尚未落地的能力写成已经可用的功能。

## 适合谁

如果你在运营一个离线登录或 authlib-injector 生态下的 Minecraft 服务器，希望玩家能有自己的账号、角色名、皮肤和披风，AsterYggdrasil 适合你试。

如果你不想把玩家身份和材质托管在不受控的平台上，希望自己掌握数据库、材质文件、签名密钥和备份，AsterYggdrasil 也适合你试。

如果你在做自己的皮肤站、启动器或服务器面板，想要一个 Rust 后端作为 Yggdrasil 协议基础，也可以把它当成二次开发起点。

如果你想要单一二进制直接部署，不想维护复杂的 PHP 运行环境、Web 服务器插件和一堆扩展依赖，AsterYggdrasil 也符合这个方向。

## 不适合谁

如果你需要一个已经打磨完整的商业级皮肤站前端，当前版本还不适合直接拿来当最终产品。后端能力更扎实，前端管理面板仍在演进。

如果你需要多节点对象存储、S3/MinIO 材质后端、多主高可用或复杂封禁系统，当前版本还不适合直接承担这些场景。当前生产可用的材质存储后端是 local；S3 配置形状已经预留，但 backend 还没有实现。

如果你只是想要一个 Mojang 官方在线模式服务器，这个项目也不是那个方向。AsterYggdrasil 面向的是自建 Yggdrasil/authlib-injector 接入。

## 现在的边界

玩家档案名创建后不可改名。要换名，就删除旧 profile，再创建新的 profile。这样做不是偷懒，是为了避免启动器缓存、token、服务端白名单和材质签名互相打架。

材质上传只接受 PNG。服务端会重编码为安全 PNG，并以处理后的内容计算 hash。你上传的原始文件不会长期保存。

公开材质 URL 必须是客户端能访问的绝对 URL。生产部署时要认真配置 `public_site_url` 或 `yggdrasil_public_base_url`，不然 profile textures 响应会因为无法生成公网 URL 而失败。

签名私钥不要手写塞进数据库。生产环境通过管理端 config action 轮换，让服务端生成并维护成对的 RSA 私钥和公钥。

## 从哪里开始

- 想先跑起来：看 [快速开始](./getting-started)。
- 想知道普通用户怎么用：看 [用户手册](./user-guide)。
- 想接启动器或服务端：看 [Yggdrasil API](./yggdrasil-api) 和 [启动器登录](./launcher-login)。
- 想上线部署：看 [Docker 部署](/deployment/docker)。
