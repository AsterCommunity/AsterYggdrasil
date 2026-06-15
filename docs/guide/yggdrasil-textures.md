# Yggdrasil 材质处理

AsterYggdrasil 现在有两套材质入口。

面向启动器和 authlib-injector 兼容工具的是 Yggdrasil 上传接口：

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
GET    /api/yggdrasil/textures/{hash}
```

面向站点用户的是 wardrobe 和 profile 绑定接口：

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

Yggdrasil 协议接口返回协议格式，不使用项目 API envelope。`/api/v1/...` 站点和管理接口继续返回统一 envelope。

## 上传校验

上传文件必须是 `image/png`。服务端会先读取 PNG header 获取尺寸，再解码图像，避免在尺寸校验前把潜在 PNG bomb 完整读入内存。

允许的尺寸：

- skin: `64x32` 的整数倍，或 `64x64` 的整数倍。
- cape: `64x32` 的整数倍，或 `22x17` 的整数倍。

不符合尺寸的文件会返回 Yggdrasil 协议错误，错误类型为 `IllegalArgumentException`。

## Cape 22x17 兼容

authlib-injector 文档允许旧式 cape 使用 `22x17` 的整数倍，但它不是标准 cape 画布。AsterYggdrasil 在上传处理阶段会把这种 cape 补透明到对应的标准 `64x32` 画布：

- `22x17` 会保存为 `64x32`。
- `44x34` 会保存为 `128x64`。
- 原图像素从左上角开始保留。
- 新增区域填充完全透明像素。

这个归一化发生在存储前，所以后续 metadata、公开读取、hash、引用计数和孤儿清理都基于处理后的 PNG。客户端拿到的 URL 永远指向已经重编码后的安全 PNG。

## 重编码和 Hash

服务端会把上传 PNG 解码为 RGBA，再重新编码为 PNG。这样可以去掉与位图无关的 PNG 元数据，避免客户端分发用户上传文件中的额外数据。

材质 hash 使用处理后 PNG 文件内容计算，并作为公开 URL 的最后一段：

```text
/api/yggdrasil/textures/{sha256}
```

Minecraft/authlib-injector 客户端会把 URL 文件名当作材质标识缓存，所以同一张处理后图片会稳定命中同一个 hash。

## Public URL 和 skinDomains

`textures` property 中的 URL 必须是客户端可访问的绝对 URL。普通部署只需要配置 `public_site_url`，服务端会派生出 `{public_site_url}/api/yggdrasil/textures/{hash}`。如果配置了高级覆盖项 `yggdrasil_public_base_url`，服务端会优先使用第一个可用的 http/https URL。

authlib-injector 会校验材质 URL 的域名是否在 metadata 的 `skinDomains` 中。metadata 会自动包含 Mojang 官方域名 `.minecraft.net`、`.mojang.com`，以及当前有效 texture URL 的 host。`yggdrasil_skin_domains` 只用于额外允许 CDN 或外部材质域名。

## 存储和维护

当前支持 local texture storage，并通过 `texture_storage.local_root` 显式配置本地根目录。上传成功后只保存处理后的 PNG，不保存原始上传文件。

运行时维护任务包括：

- `yggdrasil-texture-cleanup`: 删除没有数据库引用的材质对象。
- `yggdrasil-storage-consistency-check`: 检查数据库记录指向的对象是否缺失，以及对象内容 hash 是否和数据库记录一致。
- `yggdrasil-token-cleanup`: 清理过期或已吊销的 Yggdrasil token。

删除材质会先删除数据库引用，再按引用计数清理对象。多个 profile 引用同一 hash 时，不会误删仍被引用的对象。
