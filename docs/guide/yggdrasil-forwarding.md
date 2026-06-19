---
description: 面向站点主和服主的 Yggdrasil session 转发指南，说明 AY、Yggdrasil 兼容站点、Mojang 上游、优先级权重、材质转发和 CustomSkinLoader 兜底方案。
---

# Yggdrasil 转发

::: tip 这一篇覆盖什么
这页写给站点主和 Minecraft 服务器管理员。它说明如何让服务端统一接入 AsterYggdrasil，同时允许来自 AY、本地外部皮肤站或 Mojang 的玩家通过 session 验证。
:::

Yggdrasil 转发只处理 **sessionserver 的进服验证**。它不会把外部皮肤站账号导入 AY，也不会代理外部 authserver 登录。

典型用途：

- Minecraft 服务端只配置 AY 的 Yggdrasil API。
- AY 本地玩家可以直接进服。
- Yggdrasil 兼容站点（如 LittleSkin）和 Mojang 等上游玩家也可以通过 AY 的 `hasJoined` 转发进服。
- 站点主可以按优先级和权重调整上游查询顺序。
- 如果客户端信任 AY metadata，可以让 AY 代理外部材质 URL 并重新签名。

## 入口速查

| 你想做什么 | 去哪里 |
| --- | --- |
| 让服务端统一接入 AY | 管理后台 -> Yggdrasil 转发 |
| 调整 AY、Yggdrasil 兼容站点、Mojang 查询顺序 | 修改转发服务器的优先级和权重 |
| 让 AY 客户端显示兼容站点材质 | 对对应兼容站点上游开启材质转发 |
| 让兼容站点客户端保持原材质显示 | 对对应兼容站点上游关闭材质转发 |
| 多皮肤站客户端侧兜底 | 使用 CustomSkinLoader |

## 服务端应该填什么

Minecraft 服务端仍然只配置 AY：

```text
https://skin.example.com/api/yggdrasil
```

如果是本机测试，可能是：

```text
http://localhost:3300/api/yggdrasil
```

authlib-injector javaagent 示例：

```text
-javaagent:authlib-injector.jar=https://skin.example.com/api/yggdrasil
```

服务端保持在线模式：

```properties
online-mode=true
enforce-secure-profile=true
```

这样 Minecraft 服务端进服时只会问 AY：

```text
GET /api/yggdrasil/sessionserver/session/minecraft/hasJoined
```

AY 再按你配置的转发服务器顺序查询本地和上游。

## 转发服务器

转发服务器是一条 `hasJoined` 查询来源。当前常见类型：

| 来源 | 作用 | 建议 |
| --- | --- | --- |
| AsterYggdrasil | AY 本地 join 记录 | 保留启用，通常优先级较高 |
| Yggdrasil 兼容站点（如 LittleSkin） | 查询外部皮肤站 sessionserver | 按社区需要启用 |
| Mojang | 查询正版 sessionserver | 当前建议视为测试中 |

AY 本地也作为一条转发服务器参与排序。这样你可以把 AY 本地和外部上游放在同一张表里调整顺序。

## 优先级和权重

查询顺序先看优先级，再看权重。

- 优先级数字越小，越先查询。
- 同一优先级下，权重越高，被排在前面的概率越高。
- 某个上游返回匹配 profile 后，AY 立即返回该 profile，不再继续查询后面的上游。
- 上游失败不会直接让进服失败，AY 会记录失败并继续尝试后面的上游。

推荐从简单配置开始：

| 上游 | enabled | priority | weight |
| --- | ---: | ---: | ---: |
| AsterYggdrasil | 是 | 100 | 1 |
| Yggdrasil 兼容站点 | 是 | 150 | 1 |
| Mojang | 否或测试中 | 200 | 1 |

如果你希望外部上游优先于 AY 本地，可以把外部上游 priority 调到更小。但要先确认重名 profile 的处理符合你的服务器预期。

## Session 转发行为

玩家进服时有两段动作：

1. 客户端向自己登录的 Yggdrasil 服务发送 `join`。
2. Minecraft 服务端向 AY 发送 `hasJoined`。

例如 Yggdrasil 兼容站点玩家进 AY 服务端：

```text
兼容站点客户端 -> 兼容站点 join
Minecraft 服务端 -> AY hasJoined
AY -> 兼容站点 hasJoined
```

只要兼容站点上游能匹配这次 join，AY 就会向 Minecraft 服务端返回该玩家 profile，玩家就能进服。

## 材质转发是什么

材质转发只影响上游返回的 `textures` property。

关闭材质转发时，AY 原样返回上游的 `textures.value` 和 `signature`。客户端会继续使用上游原始材质 URL。

开启材质转发时，AY 会：

1. 解码上游 `textures.value`。
2. 把其中的材质 URL 改写为 AY 的代理 URL。
3. 用 AY 的 Yggdrasil 私钥重新签名 `textures` property。
4. 客户端请求 AY 的代理 URL。
5. AY 后端请求原始上游材质 URL，并把 PNG 返回给客户端。

代理 URL 形如：

```text
https://skin.example.com/api/yggdrasil/sessionserver/session/minecraft/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}
```

`ticket` 是 AY 签出的转发凭据，里面包含原始材质 URL。它不依赖短期缓存，所以玩家在线很久、后来者重新拉皮肤，也不会因为 30 秒或几分钟 TTL 过期而 404。

::: warning 材质转发依赖客户端信任 AY metadata
AY 改写材质 URL 后必须用 AY 私钥重新签名。客户端只有在使用 AY metadata 时，才会信任 AY 的 `signaturePublickey` 和 `skinDomains`。
:::

## 客户端预期

下面假设：

- Minecraft 服务端配置 AY。
- 兼容站点上游已启用 session 转发。
- 兼容站点上游开启材质转发。
- AY 的 `public_site_url` 或 `yggdrasil_public_base_url` 已正确配置，metadata 的 `skinDomains` 包含 AY 公开域名。

| 客户端登录源 | 能进 AY 服务端 | 看 AY 本地皮肤 | 看兼容站点材质 | 说明 |
| --- | ---: | ---: | ---: | --- |
| AY 客户端 | 是 | 是 | 是 | 推荐链路。客户端信任 AY 公钥和 AY 材质域名。 |
| 兼容站点客户端 | 是 | 通常否 | 可能失败 | 客户端信任对应兼容站点 metadata，不一定信 AY 重签后的 textures。 |
| Mojang 客户端 | 取决于登录和上游命中 | 通常否 | 通常否 | 不建议把 Mojang 客户端当作混合皮肤站显示方案。 |

如果兼容站点客户端也要稳定显示该站点原材质，建议关闭对应上游的材质转发。这样 AY 会原样返回上游的 `textures`，兼容站点客户端仍用该站点公钥和材质域名校验。

| 兼容站点上游材质转发 | AY 客户端看兼容站点材质 | 兼容站点客户端看原站材质 |
| ---: | ---: | ---: |
| 开 | 通常正常 | 可能失败 |
| 关 | 取决于 AY metadata 是否允许原站材质域名 | 通常正常 |

所以如果服务器希望“统一体验”，推荐让玩家使用 AY 登录。如果服务器允许玩家继续使用各自皮肤站登录，就要接受不同客户端看到的材质可能不一致。

## CustomSkinLoader 兜底

[CustomSkinLoader](https://github.com/xfl03/MCCustomSkinLoader) 是客户端 mod，可以从 Mojang、LittleSkin、Ely.by、CustomSkinAPI、UniSkinAPI、Legacy 等来源加载皮肤。它适合解决“玩家来自不同皮肤站，客户端希望尽量显示多来源材质”的问题。

它和 AY 转发的分工不同：

| 能力 | AY Yggdrasil 转发 | CustomSkinLoader |
| --- | ---: | ---: |
| 决定玩家能不能进服 | 是 | 否 |
| 由服务端统一配置 | 是 | 否，需要客户端安装 mod |
| 显示多皮肤站材质 | 部分场景可用 | 更适合 |
| 依赖 authlib-injector metadata 签名 | 是 | 不完全相同 |

建议：

- 服务端进服验证交给 AY 转发。
- AY 客户端需要显示兼容站点材质时，开启 AY 材质转发。
- 混合客户端社区可以推荐安装 CustomSkinLoader，把 LittleSkin、Mojang 或其他皮肤站加入客户端加载列表。
- 不要把 CustomSkinLoader 当作权限系统。它只影响材质显示，不证明玩家身份。

::: details 为什么 CustomSkinLoader 有帮助？
authlib-injector 的 `textures` property 有签名和 `skinDomains` 信任模型。一个客户端通常只信任自己登录的 Yggdrasil metadata。AY 可以重签外部材质，但兼容站点客户端不一定信 AY 公钥；原站签名又不能覆盖 AY 改写后的 URL。

CustomSkinLoader 从客户端侧额外查询材质来源，可以绕开“单一 metadata 信任根只能完整信一个服务”的限制。但这只是显示层补充，不改变 sessionserver 的进服验证。
:::

## 排障速查

| 现象 | 重点检查 |
| --- | --- |
| 兼容站点玩家不能进服 | 兼容站点上游是否启用、base URL 是否正确、服务端是否配置 AY、兼容站点客户端是否完成 join |
| AY 客户端看不到兼容站点材质 | 兼容站点上游是否开启材质转发、AY metadata 是否包含 AY 公开域名、`hasJoined` 返回的 URL 是否带 `{ticket}` |
| 材质 URL 返回 404 | 客户端是否拿到旧的两段 URL；重启后端并重新进服，让客户端拿新的 `{upstream_id}/{texture_hash}/{ticket}` URL |
| 日志提示签名验证失败 | 客户端可能没有使用 AY metadata，或 metadata 缓存了旧公钥 |
| 日志提示 URL 不在白名单 | 检查 `public_site_url` / `yggdrasil_public_base_url` 的 host 是否出现在 metadata `skinDomains` |
| 不同客户端看到不同皮肤 | 这是多 Yggdrasil 信任根的正常限制；考虑统一使用 AY 登录或安装 CustomSkinLoader |

## 上线前检查

- `public_site_url` 或 `yggdrasil_public_base_url` 是客户端可访问的绝对 URL。
- metadata 的 `skinDomains` 包含 AY 公开域名。
- 签名私钥已生成，metadata 的 `signaturePublickey` 能被客户端获取。
- 兼容站点等上游 base URL 填到 Yggdrasil API 根路径。
- Mojang 上游仍按测试中处理，不要把它作为唯一可用链路。
- 开启材质转发后，用 AY 客户端实际进服，看客户端是否请求 `/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}`。
