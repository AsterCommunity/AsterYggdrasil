# AsterYggdrasil

自托管 Minecraft 皮肤站与 Yggdrasil/authlib-injector 认证服务器。

> **快速开发版本提醒**
>
> 当前目标版本是 `0.1.0-beta.1`，仍处于快速开发阶段。项目已经具备账号、分权 operator、公开认证流程图形验证码、Minecraft profile、Yggdrasil 协议端点、wardrobe 材质、公共材质库、运行时配置、审计日志和后台维护任务等完整链路。请不要把当前 beta 当成长期稳定接口；生产部署前先阅读文档并做好备份。

- English README: [README.md](README.md)
- 在线文档: [yggdrasil.astercosm.com](https://yggdrasil.astercosm.com/)
- 文档源码: [docs/index.md](docs/index.md)
- 快速开始: [docs/guide/getting-started.md](docs/guide/getting-started.md)
- 用户手册: [docs/guide/user-guide.md](docs/guide/user-guide.md)
- Docker 部署: [docs/deployment/docker.md](docs/deployment/docker.md)
- 配置示例: [config.example.toml](config.example.toml)
- 开发者文档: [developer-docs/README.md](developer-docs/README.md)

## AsterYggdrasil 是什么？

AsterYggdrasil 把 Minecraft 私有部署里常见的身份和材质链路放进一个自托管服务里：

- 站点账号注册、登录、刷新、退出和首个管理员初始化。
- Minecraft profile 独立建模，一个站点账号可以拥有多个 profile。
- Yggdrasil/authlib-injector 协议根路径 `/api/yggdrasil`。
- 启动器登录、token refresh/validate/invalidate/signout。
- Minecraft join / hasJoined / profile 查询。
- skin/cape 上传、PNG 重编码、旧式 cape 兼容、hash 公开读取和 local/S3/MinIO 对象存储。
- wardrobe 材质管理，以及公共材质库提交、审核、发布、标签、复制、举报和下架。
- 管理员和分权 operator 工作流，覆盖用户、profile、公共材质库审核、配置、审计、任务和外部认证。
- 运行时配置、Yggdrasil 签名密钥轮换、审计日志和周期维护任务。

AsterYggdrasil 聚焦 Minecraft/Yggdrasil 业务域：账号、玩家档案、皮肤、披风、启动器登录、服务端进服验证、签名密钥、材质存储和管理员运维。

## 当前适合谁

AsterYggdrasil 适合这些场景：

- 你在运营 authlib-injector 或离线登录生态下的 Minecraft 服务器。
- 你希望自己掌握玩家账号、Minecraft profile、材质文件、数据库、签名密钥和备份。
- 你需要 Yggdrasil/authlib-injector 兼容协议端点。
- 你想从 SQLite + local object storage 起步，后续再按需要扩展数据库或存储后端。
- 你想要单一二进制直接部署，不想维护复杂的 PHP 运行环境、Web 服务器插件和扩展依赖。
- 你想基于 Rust、Actix Web、SeaORM、React 和 Vite 做二次开发。

当前版本不适合这些场景：

- 需要成熟商业级运营后台，且不打算自己做上线前验证。
- 需要客户端直接向 S3/MinIO presigned 上传。当前上传统一走服务端 streaming。
- 需要多主高可用、自动故障切换、复杂封禁系统或企业合规承诺。
- 需要完整 Minecraft 游戏服务器生命周期管理，例如世界托管、插件管理、控制台访问或定时开关服。
- 需要替代 Mojang 官方 online-mode 面向任意公网客户端提供通用账号服务。

## 当前真实能力

### 账号与站点 API

- `POST /api/v1/auth/setup`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/refresh`
- `POST /api/v1/auth/logout`
- `GET /api/v1/auth/me`
- 会话管理、Passkey、头像、外部认证 provider、分权 operator 和图形验证码策略。
- 项目 API 使用统一 envelope 和稳定 `AsterErrorCode`。

### Yggdrasil 协议 API

协议根路径：

```text
/api/yggdrasil
```

常用端点：

```text
GET  /api/yggdrasil
POST /api/yggdrasil/authserver/authenticate
POST /api/yggdrasil/authserver/refresh
POST /api/yggdrasil/authserver/validate
POST /api/yggdrasil/authserver/invalidate
POST /api/yggdrasil/authserver/signout

POST /api/yggdrasil/sessionserver/session/minecraft/join
GET  /api/yggdrasil/sessionserver/session/minecraft/hasJoined
GET  /api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}

POST /api/yggdrasil/api/profiles/minecraft
GET  /api/yggdrasil/textures/{hash}
```

协议端点返回 Yggdrasil/authlib-injector 兼容响应，不套 `/api/v1` 的项目 envelope。

站点首页 `/` 会返回：

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

支持 API Location Indication 的启动器可以通过站点根地址发现协议根路径。

### Minecraft profile

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
PUT    /api/v1/admin/minecraft-profiles/{uuid}/name
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

profile name 支持通过用户或管理员 API 受控改名。改名会保留 UUID、材质绑定和审计链路，并临时失效绑定该 profile 的 Yggdrasil token，让启动器通过 refresh 获取新名称。

### 材质系统

站点用户可以先上传 wardrobe 材质，再绑定到 profile：

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

启动器或兼容工具可以走 Yggdrasil 上传接口：

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
GET    /api/yggdrasil/textures/{hash}
```

上传文件必须是 PNG。服务端会校验 MIME、尺寸、上传开关和 profile 所属关系，把图片重编码为安全 PNG，再按处理后的内容计算 hash。

公共材质库 API 支持用户发布和复用 wardrobe 材质：

```text
GET    /api/v1/texture-library/tags
GET    /api/v1/texture-library/textures
GET    /api/v1/texture-library/textures/{texture_id}
POST   /api/v1/texture-library/textures/{texture_id}/copy
POST   /api/v1/texture-library/textures/{texture_id}/reports
POST   /api/v1/wardrobe/textures/{texture_id}/library-submission
DELETE /api/v1/wardrobe/textures/{texture_id}/library-submission
```

管理员和拥有 `texture_library` scope 的 operator 可以审核提交、管理标签、处理举报和下架公共材质：

```text
GET  /api/v1/admin/texture-library/textures
POST /api/v1/admin/texture-library/textures/{texture_id}/approve
POST /api/v1/admin/texture-library/textures/{texture_id}/reject
POST /api/v1/admin/texture-library/textures/{texture_id}/unpublish

GET  /api/v1/admin/texture-library/reports
POST /api/v1/admin/texture-library/reports/{report_id}/accept
POST /api/v1/admin/texture-library/reports/{report_id}/reject
```

### 配置、审计和任务

- `system_config` 管理运行时配置。
- `texture_library_enabled` 和 `texture_library_review_required` 控制公共材质库。
- `auth_captcha_*` 控制图形验证码策略和预览。
- `POST /api/v1/admin/config/yggdrasil/action` 用于 Yggdrasil 签名密钥轮换。
- `POST /api/v1/admin/config/auth_captcha/action` 用于预览验证码渲染效果。
- `GET /api/v1/admin/audit-logs` 查询审计日志。
- `GET /api/v1/admin/tasks`、`POST /api/v1/admin/tasks/{id}/retry`、`POST /api/v1/admin/tasks/cleanup` 管理后台任务。
- runtime task 覆盖 token 清理、材质对象清理、存储一致性检查、审计清理和 task artifact 清理。
- 数据库搜索 helper 和事务机械行为接入 AsterForge，Yggdrasil 保留产品侧错误映射和 repository 业务语义。

## 快速开始

### 从源码运行

```bash
git clone https://github.com/AsterCommunity/AsterYggdrasil.git
cd AsterYggdrasil

cd frontend-panel
bun install
bun run build
cd ..

cargo run
```

默认监听：

```text
http://127.0.0.1:3000
```

首次启动会生成 `data/config.toml`，创建默认 SQLite 数据库，执行迁移，并初始化运行时配置。

健康检查：

```text
GET /health
GET /health/ready
```

### Docker 试跑

本地 HTTP 试跑可以用：

```bash
mkdir -p ./data

docker run -d \
  --name asteryggdrasil \
  -p 3000:3000 \
  -e ASTER__SERVER__HOST=0.0.0.0 \
  -e ASTER__AUTH__BOOTSTRAP_INSECURE_COOKIES=true \
  -e 'ASTER__DATABASE__URL=sqlite:///data/asteryggdrasil.db?mode=rwc' \
  -v "$(pwd)/data:/data" \
  ghcr.io/astercommunity/asteryggdrasil:latest
```

`ASTER__AUTH__BOOTSTRAP_INSECURE_COOKIES=true` 只适合本机或内网 HTTP 测试。正式部署请使用 HTTPS，并保持安全 Cookie 开启。

完整部署说明见 [docs/deployment/docker.md](docs/deployment/docker.md)。

## 生产部署提醒

- 不要直接把 `:3000` 暴露到公网。请放在反向代理后面，由代理处理 HTTPS、上传限制和真实客户端 IP。
- 正式使用前配置 `public_site_url` 或 `yggdrasil_public_base_url`，否则 textures property 无法生成客户端可访问的绝对 URL。
- 备份数据库、`data/config.toml` 和 object storage backend 或 local object storage 目录。
- Yggdrasil 签名私钥属于敏感配置。使用 config action 轮换，不要直接手写数据库。
- 当前部署文档只覆盖单个任务 owner。多实例部署时，请先在外层保证只有一个实例启动完整后台任务。
- 当前生产可用的 object storage backend 是 local、S3 或 MinIO。材质和上传头像都会走同一个 backend。
- 对公开读的 S3/MinIO bucket 或 CDN，可以配置 `yggdrasil_texture_public_base_url` 让已上传材质 URL 直接指向对象存储；默认皮肤仍走 Yggdrasil API。

## 常用开发命令

```bash
# 后端
cargo fmt
cargo check
cargo test
cargo test --features openapi --test generate_openapi
cargo test --features metrics
cargo run

# 前端
cd frontend-panel
bun install
bun run dev
bun run build
bun run check
bun run test
bun run test:e2e

# 文档
cd docs
bun install
bun run docs:dev
bun run docs:build
```

## 项目结构

```text
src/                         Rust 后端
src/api/                     路由、DTO、OpenAPI 注册、中间件、响应封装
src/cache/                   cache trait 以及 memory/noop/Redis 实现
src/config/                  静态配置、运行时配置定义、配置规范化
src/db/                      数据库连接适配、事务边界、repository
src/entities/                SeaORM entity
src/runtime/                 AppState、启动、关闭、日志、后台任务循环
src/services/                auth、external auth、config、mail、audit、task、health、Yggdrasil、texture
src/object_storage/          材质和上传头像共用的对象存储抽象
src/types/                   共享枚举和 DB wrapper 类型
src/utils/                   crypto、ID、path、number、email、RAII 等工具
migration/                   SeaORM migration crate
api-docs-macros/             OpenAPI 辅助宏
frontend-panel/              React + Vite 产品前端和管理后台
developer-docs/              开发说明
docs/                        用户/部署文档站
tests/                       集成测试和 OpenAPI 导出测试
tmp/authlib-injector/wiki/   authlib-injector/Yggdrasil 参考资料入口
```

## License

MIT. See [LICENSE](LICENSE).
