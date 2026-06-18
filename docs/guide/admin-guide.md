---
description: AsterYggdrasil 管理员指南，说明管理员需要维护的用户、profile、材质、配置、审计和后台任务。
---

# 管理员指南

::: tip 这一篇覆盖什么
这页按管理员日常工作组织，不替代具体配置页。配置细节看 [配置和密钥](/guide/configuration)。
:::

## 管理员要管什么

管理员主要维护六类对象：

| 对象 | 你要关心什么 |
| --- | --- |
| 用户 | 谁能登录站点，谁是管理员，会话是否需要吊销 |
| Minecraft profile | 玩家名、UUID、所属用户、改名和删除 |
| 材质 | skin/cape 上传、绑定、公开读取和孤儿清理 |
| Yggdrasil 配置 | 公开 URL、profile name 登录、上传开关、token 策略 |
| 签名密钥 | metadata 公钥、textures property 签名、key rotate |
| 审计和任务 | 管理操作、登录协议行为、清理任务和失败重试 |

## 第一次管理员账号

第一次运行时，通过 setup 流程创建首个账号：

```text
POST /api/v1/auth/setup
```

第一个账号会成为管理员。之后管理员可以管理用户、运行时配置、Minecraft profiles、审计日志和后台任务。

## 管用户和 profile

用户是站点登录身份，Minecraft profile 是进服身份。一个用户可以拥有多个 profile。

常用管理员 API：

```text
GET    /api/v1/admin/users
GET    /api/v1/admin/users/{id}
PATCH  /api/v1/admin/users/{id}
POST   /api/v1/admin/users/{id}/sessions/revoke
GET    /api/v1/admin/users/{user_id}/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}
PUT    /api/v1/admin/minecraft-profiles/{uuid}/name
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

改 profile name 必须走 API。直接改数据库会让 token、启动器缓存、白名单、材质属性和审计互相不一致。

## 管材质

材质分两层：

- wardrobe：用户自己的材质库。
- profile texture：绑定到某个 Minecraft profile 的 skin/cape 槽位。

管理员可以查看 profile 绑定的材质，也可以删除绑定或按 hash 删除材质引用：

```text
GET    /api/v1/admin/minecraft-profiles/{uuid}/textures
DELETE /api/v1/admin/minecraft-profiles/{uuid}/textures/{skin|cape}
DELETE /api/v1/admin/minecraft-textures/{hash}
```

删除会走服务层引用计数。不要直接删 storage 文件，否则一致性检查会报告 missing object。

## 管配置

运行时配置通过 Admin Config API 修改：

```text
GET    /api/v1/admin/config
GET    /api/v1/admin/config/schema
PUT    /api/v1/admin/config/{key}
DELETE /api/v1/admin/config/{key}
POST   /api/v1/admin/config/{key}/action
```

上线前优先确认：

- `public_site_url`
- `yggdrasil_public_base_url`
- `yggdrasil_skin_domains`
- `yggdrasil_allow_skin_upload`
- `yggdrasil_allow_cape_upload`
- `yggdrasil_token_ttl_days`
- `yggdrasil_max_active_tokens`

签名私钥不应手动修改。轮换应使用 action：

```text
rotate_yggdrasil_signature_key
```

## 看审计和后台任务

管理员操作、Yggdrasil 登录行为、材质上传/删除、profile 创建/删除/改名都会写入审计。

```text
GET /api/v1/admin/audit-logs
GET /api/v1/admin/tasks
POST /api/v1/admin/tasks/cleanup
POST /api/v1/admin/tasks/{id}/retry
```

重点关注：

- `yggdrasil-token-cleanup`
- `yggdrasil-texture-cleanup`
- `yggdrasil-storage-consistency-check`

如果一致性检查失败，先确认数据库和材质目录是否被人工改动或只恢复了一半备份。
