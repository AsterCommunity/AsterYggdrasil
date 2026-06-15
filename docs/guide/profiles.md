# 玩家档案

Minecraft profile 是玩家在 Yggdrasil 协议里的角色身份。站点账号负责登录，profile 负责进服时的玩家名、UUID 和材质属性。

AsterYggdrasil 把 profile 单独建模，不把它混在用户表里。这样一个站点账号可以有多个 Minecraft profile，管理员也能清楚地看到“账号”和“角色身份”之间的关系。

## 创建和列表

当前用户 API：

```text
GET    /api/v1/profiles/minecraft
POST   /api/v1/profiles/minecraft
GET    /api/v1/profiles/minecraft/{uuid}/textures
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}
```

管理员 API：

```text
GET    /api/v1/admin/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}
GET    /api/v1/admin/users/{user_id}/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}/textures
DELETE /api/v1/admin/minecraft-profiles/{uuid}/textures/{skin|cape}
DELETE /api/v1/admin/minecraft-textures/{hash}
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

管理员列表支持按 name、uuid、user 相关条件筛选。

## 和 wardrobe 的关系

用户可以先把 skin/cape 上传到 wardrobe：

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
```

然后再把 wardrobe 里的某个 texture 绑定到 profile 的 skin 或 cape 槽位。

这和 Yggdrasil 直接上传不冲突。直接上传会把处理后的材质写到指定 profile；wardrobe 则更像个人材质库，适合复用和管理。

## 名称规则

profile name 创建后不可改名。不要通过数据库直接改名，因为启动器缓存、token、审计、服务端白名单和材质属性都会受到影响。

需要换名时：

1. 删除旧 profile。
2. 创建新 profile。
3. 重新登录启动器。

当前没有名称历史表。

## 删除语义

删除 profile 会：

- 删除 profile 记录。
- 删除 profile 关联的 texture 记录。
- 按引用计数清理不再被引用的 texture 对象。
- 吊销 selectedProfile 指向该 profile 的 Yggdrasil token。
- 写入 audit log。

如果多个 profile 引用同一材质 hash，删除其中一个 profile 不会误删仍被其他 profile 使用的对象。

## 禁用和封禁

当前版本不提供简单的 `profile.disabled` 字段。profile 禁用会影响登录、join、hasJoined、材质读取、管理端展示和审计策略，应该由后续统一封禁系统一起定义。

在封禁系统落地前，不要添加临时禁用字段或绕过删除流程。
