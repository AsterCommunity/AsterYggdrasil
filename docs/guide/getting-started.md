# 快速开始

这一页只做一件事：把 AsterYggdrasil 在本机跑起来，并确认 Yggdrasil metadata、账号、profile 和材质路径都是真能工作的。

如果你已经准备好正式上线，可以先跑完这一页，再去看 [Docker 部署](/deployment/docker)。先把本地链路跑通，能减少后续排查公开 URL、反向代理和启动器接入问题的成本。

## 前置条件

- Rust stable toolchain。
- SQLite。默认配置使用本地 SQLite，不需要额外数据库服务。
- Bun。只有运行文档站或前端管理面板时需要。

## 1. 启动后端

```bash
cargo run
```

首次启动会创建运行时目录、SQLite 数据库和默认静态配置。默认配置文件位置是：

```text
data/config.toml
```

默认监听地址：

```text
http://127.0.0.1:3000
```

健康检查地址：

```text
GET /health
GET /health/ready
```

## 2. 创建第一个管理员

站点内置本地认证和管理员能力。第一次运行时，先通过 setup 流程创建管理员账号：

```text
POST /api/v1/auth/setup
```

后续普通登录、注册和刷新使用：

```text
POST /api/v1/auth/login
POST /api/v1/auth/register
POST /api/v1/auth/refresh
```

第一个创建成功的账号会成为管理员。管理员用来配置公开 URL、Yggdrasil 策略、签名密钥、审计和后台任务。

管理员能力包括：

- 查看和更新运行时配置。
- 执行 Yggdrasil 签名密钥轮换 action。
- 查看 audit logs。
- 查看和重试后台任务。
- 管理用户、Minecraft profiles 和 textures。

## 3. 验证 Yggdrasil metadata

启动后访问：

```text
GET /api/yggdrasil
GET /api/yggdrasil/
```

响应是 authlib-injector metadata，不使用项目 API envelope。它应该包含：

- `meta.serverName`
- `meta.implementationName`
- `meta.implementationVersion`
- `meta.feature.non_email_login`
- `skinDomains`
- `signaturePublickey`

站点首页 `/` 会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

支持 ALI 的启动器可以只填写站点地址，再自动发现真正的 Yggdrasil API 根路径。反向代理上线时别把这个响应头删掉。

## 4. 创建 Minecraft profile

登录站点账号后，用户可以创建自己的 Minecraft profile。profile 才是启动器和服务端看到的玩家身份：

```text
POST /api/v1/profiles/minecraft
GET  /api/v1/profiles/minecraft
```

profile name 创建后不可改名。需要换名时删除旧 profile 并重新创建。

## 5. 上传和绑定材质

当前用户可以把材质先放进 wardrobe，再绑定到某个 profile：

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/skin
POST   /api/v1/wardrobe/textures/cape
PUT    /api/v1/profiles/minecraft/{uuid}/textures/skin
PUT    /api/v1/profiles/minecraft/{uuid}/textures/cape
```

启动器或兼容工具也可以走 Yggdrasil 材质上传端点，直接把材质写到 profile 上：


```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/skin
PUT    /api/yggdrasil/api/user/profile/{uuid}/cape
DELETE /api/yggdrasil/api/user/profile/{uuid}/skin
DELETE /api/yggdrasil/api/user/profile/{uuid}/cape
```

上传请求需要 Yggdrasil access token。服务端会校验 token、profile 所属关系、上传开关、MIME、PNG 尺寸，并把图片重编码为安全 PNG。

公开读取：

```text
GET /api/yggdrasil/textures/{hash}
```

## 6. 配公开 URL

本地试跑可以先跳过这一步。只要要给真实启动器、真实服务端或外网用户使用，就必须配置公开 URL。

普通部署优先配置：

```text
public_site_url
```

如果 Yggdrasil API 暴露在单独路径或域名下，再配置高级覆盖项：

```text
yggdrasil_public_base_url
```

否则 textures property 里无法生成客户端可访问的绝对 URL，皮肤显示会失败。

## 7. 本地文档站

```bash
cd docs
bun install
bun run docs:dev
```

构建文档：

```bash
cd docs
bun run docs:build
```

## 下一步

- [Yggdrasil API](./yggdrasil-api.md)
- [启动器登录](./launcher-login.md)
- [玩家档案](./profiles.md)
- [材质处理](./yggdrasil-textures.md)
- [配置和密钥](./configuration.md)
