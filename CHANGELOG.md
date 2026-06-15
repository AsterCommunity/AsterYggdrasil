# Changelog

All notable changes to AsterYggdrasil will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.1.0-alpha.1] - 2026-06-16

### Release Highlights

**AsterYggdrasil 第一个公开版本！** 自托管 Minecraft 皮肤站与 Yggdrasil 认证服务器，单二进制分发，Rust 编写，MIT 许可证，authlib-injector 全协议兼容。

- **完整 Yggdrasil 协议** — authserver（authenticate / refresh / validate / invalidate / signout）+ sessionserver（join / hasJoined / profile），RSA 4096 签名材质 properties
- **皮肤站与衣柜系统** — skin / cape 上传（1.8 + legacy 尺寸校验、PNG 净化）、SHA-256 去重、头像 WebP 生成、Profile 多材质槽位管理
- **多种登录方式** — 账号密码（argon2）+ Passkey / WebAuthn + 第三方（OIDC / OAuth2 / GitHub / Google / Microsoft / QQ）
- **多数据库支持** — SQLite / MySQL / PostgreSQL，SeaORM 迁移统一时间列
- **企业级基础设施** — 运行时热配置、Redis / 内存缓存、Prometheus 指标、审计日志、后台任务引擎、SMTP outbox 邮件
- **前端面板** — React 19 + Vite 8 + Tailwind 4，skinview3d 3D 预览，rust-embed 内嵌进单二进制

### Added

- **Yggdrasil 认证协议**
  - 完整实现 authlib-injector 协议：authserver（authenticate / refresh / validate / invalidate / signout）与 sessionserver（minecraft/join、minecraft/hasJoined、minecraft/profile/{uuid}、api/profiles/minecraft）
  - access token 以 SHA-256 哈希存储，支持 client token 比对、用户活跃 token 上限裁剪、过期 / 吊销清理
  - join 流程用 serverId SHA-256 做缓存键（共享缓存，TTL 30s），hasJoined 可选校验客户端 IP 防代理穿透
  - RSA 4096 + SHA-1 PKCS#1 v1.5 签名 textures property，私钥首启自动生成并持久化，无私钥时优雅降级
  - `/api/yggdrasil` metadata 端点返回 signaturePublickey / skinDomains / serverName / feature.nonEmailLogin
  - 材质 property 按 authlib-injector 规范 base64 JSON 序列化（含 timestamp / model），独立 YggdrasilError 错误映射层不污染全局 envelope

- **账户认证与会话**
  - `/auth/setup` 零用户引导创建首位管理员，`/auth/register` 普通注册（可运行时禁用），首个注册用户自动成为 Admin
  - 登录支持邮箱或用户名标识，argon2 密码校验，超级管理员（id=1）角色 / 状态不可被降级
  - JWT 双 token（HS256）：access 默认 15 分钟、refresh 默认 7 天，基于 auth_sessions 表轮换 jti 并检测重放
  - session_version 全量会话失效：改密 / 禁用用户 / revoke_all 立即作废所有现存 token
  - 会话列表、单会话撤销、撤销除当前外所有、过期会话后台清理，全部写审计

- **Passkey / WebAuthn 无密码登录**
  - 基于 webauthn-rs，强制 Discoverable Credential（resident_key=Required）+ user_verification=Required
  - 注册 / 登录两段式（challenge 写缓存 TTL 300s），支持 conditional UI（autofill）二次校验
  - Relying Party 由 public_site_url 推导 rp_id / rp_name，多 origin 白名单，localhost 允许 HTTP
  - 用户自助列出 / 重命名 / 删除 passkey，登录成功签发与密码登录相同的 JWT 双 token

- **第三方登录 / 外部认证**
  - 六种 provider：OIDC、通用 OAuth2、GitHub、Google、Microsoft、QQ，统一 trait + registry 注册派发
  - OIDC 走 openidconnect 4.0，支持 issuer 发现、PKCE、nonce 校验、ID Token claims
  - 登录 flow（state / nonce / pkce 持久化，TTL 5 分钟）支持自动开通用户与已验证邮箱自动绑定
  - 账号绑定 / 解绑（密码验证用恒定时间 argon2 + dummy hash 防枚举），邮箱域名白名单
  - provider 回调未带已验证邮箱时触发本地邮箱补验流程（TTL 30 分钟，幂等 confirm）

- **邮件系统**
  - lettre 0.11 SMTP（隐式 TLS 465 / STARTTLS / 明文），outbox 模式事务内只写表、后台批量 claim 投递
  - 阶梯退避重试（5 / 15 / 60 / 300 / 900 / 1800s，最多 6 次），stale 行多 worker 安全 claim
  - 七种内置模板（注册激活 / 邮箱变更确认 / 密码重置 / 重置通知 / 变更通知 / 邮箱补验 / 登录验证码），subject / html / text 三段式
  - 模板存 system_config 可运行时修改，投递成功 / 失败均写邮件审计，管理员可发测试邮件

- **材质 / 皮肤 / Profile / 衣柜**
  - skin + cape 上传，支持 default / slim model，multipart 流式落盘 + 实时超大小拒绝
  - image crate 解码后重写干净 RGBA8 PNG（防恶意元数据 / APNG），PNG bomb 像素数预检
  - 1.8（64x64）+ legacy（64x32）皮肤尺寸校验，legacy 22x17 cape 自动补齐到 64x32
  - SHA-256 内容指纹去重，引用计数删除（0 引用才删 blob），衣柜材质可绑定多 profile 复用
  - 头像支持 none / Gravatar / 本地上传，居中裁正方形生成 512 / 1024 WebP，版本化 + immutable 缓存
  - 维护工具：旧材质转衣柜、重复指纹合并、存储一致性巡检（缺失 / 不一致 / 孤儿 blob）

- **存储后端**
  - TextureStorage async trait 抽象（put / get_stream / delete / exists / metadata / list_keys）
  - LocalTextureStorage 本地文件系统实现，storage key 路径净化防目录穿越
  - S3 / minio 配置结构预留（显式拒绝初始化，接口契约与测试骨架就位）

- **数据库与迁移**
  - sea-orm-migration 组织 4 个迁移（foundation / Yggdrasil profiles / 材质 / Passkey）
  - 完整建模 18 张表，全部带外键级联与唯一索引
  - 多 DB 兼容：time.rs 统一时间列（MySQL datetime(6) / PG+SQLite timestamptz），迁移不写后端专属 SQL
  - Yggdrasil 域强约束：profile UUID / name 唯一、token 仅存哈希、ON DELETE SET NULL / CASCADE

- **后台任务引擎**
  - 持久化 background_tasks 表（kind / status / payload / result / steps / progress / attempt / lease）
  - lease-based 并发：processing_token CAS 认领 + 心跳续约 + lane 容量复核防超卖
  - 重试三档（Auto / Manual / Never），自动重试 5 / 15 / 60 / 300s 递增，优雅关闭释放回 Retry
  - 步骤级进度（TaskStepInfo），cleanup_expired 清理终态过期任务临时目录

- **审计日志**
  - 全异步 fire-and-forget：4096 缓冲 + 100 批量刷库，满则降级直写，不阻塞业务
  - 47 种 AuditAction × 14 种 EntityType，覆盖九大组（登录 / 配置 / 用户 / 管理 / 任务 / 邮件 / 外部认证 / Minecraft / Yggdrasil）
  - build_audit_presentation 生成 summary / target / detail 结构化消息供前端渲染
  - 运行时控制 recorded_actions + retention_days（默认 90）自动清理，管理端多维过滤分页

- **配置系统**
  - 静态 toml（data/config.toml，首启自动生成）+ ASTER\_\_SECTION\_\_KEY 环境变量覆盖 + 自动类型推断
  - 80+ 运行时配置持久化 system_config，ConfigDef 集中定义类型 / 默认 / requires_restart / is_sensitive，parking_lot::RwLock 内存热应用
  - normalize_system_value 做 trim / 类型 / 范围 / 跨字段校验（如 credentials=true 禁止 origins=*）
  - 路径沙箱（超 base_dir / SQLite URL 拒绝）、弃用键拒绝、Primary / Follower 节点模式

- **缓存**
  - 统一 CacheBackend trait + 三后端：memory（moka 64MiB + weigher + TTL）/ redis（ConnectionManager + 250ms 超时）/ Noop
  - set_bytes_if_absent 原子占位（ReservationSet + dashmap），once-only 抢占语义
  - Redis 故障熔断：5s 冷却期跳过 Redis 走内存，恢复只输出一次 transition 日志

- **监控指标**
  - metrics feature 下 Prometheus：HTTP 请求 / 耗时、DB 查询、认证事件、后台任务、pending 数等指标族
  - sysinfo 周期采集 RSS / CPU / uptime（primary 节点）
  - MetricsMiddleware 按 match_pattern 路由标签记录，关闭时短路跳过

- **运行时与日志**
  - tracing 结构化日志（text / json），级别 / 文件输出 / 按天轮转 / max_backups 可配
  - 优雅关停（SIGINT / SIGTERM）：后台任务（30s grace）→ audit flush → DB 关闭
  - panic hook 双通道记录，后台任务 catch_unwind 转 failed 结果
  - primary 启动 9+ 周期任务（mail / token / audit / task / texture 清理等），指数退避 + wakeup event + jitter
  - 可选 jemalloc 全局分配器（含 stats / profiling），默认 TrackingAlloc 统计堆峰值

- **HTTP 中间件**
  - 运行时 CORS（实时从 RuntimeConfig 构造策略，Vary 维护）
  - actix-governor 四档限流（auth / public / api / write），trusted_proxies CIDR 解析 XFF 取键，429 带 Retry-After
  - 安全响应头（X-Frame-Options / Referrer-Policy / X-Content-Type-Options）
  - Request-ID（UUID v4 + X-Request-ID 响应头 + info_span 关联日志）
  - CSRF 双提交（cookie vs X-CSRF-Token）+ Origin / Referer / Sec-Fetch-Site 来源校验
  - JWT cookie 优先 + Bearer 兜底，JwtAuth / RequireAdmin 中间件

- **错误处理**
  - 统一 AsterError + 稳定 AsterErrorCode（80+ snake_case 码，如 auth.credentials_failed / config.validation_failed）
  - Yggdrasil 协议端点走独立错误映射，不污染全局 envelope

- **管理后台 Admin API**
  - 用户管理（过滤 / 排序 / 分页 / CRUD / 头像 / 一键吊销全部会话）
  - Minecraft Profile 管理（查看 / 删除 / 按 hash 批量删材质）
  - 系统配置管理（列表 / 写入 / Schema / 模板变量元数据 / 动作如发测试邮件）
  - 审计日志查询、后台任务管理（重试 / 清理）、外部认证提供商 CRUD + 连接测试
  - 系统信息（版本 / 构建时间），Admin scope 强制 JWT + 管理员 + 可信代理限流

- **前端面板**
  - React 19 + TypeScript（tsgo 原生预览）+ Vite 8 + Tailwind CSS 4 + @base-ui/react（shadcn）
  - Zustand 5 状态、react-router-dom 7 路由、axios HTTP、vite-plugin-pwa 离线
  - skinview3d 3D 皮肤预览、react-image-crop 头像裁剪、i18next 中英双语
  - 页面：登录 / 初始化 / 外部认证回跳 / 个人设置 / Profile / 衣柜 / 工作台；管理端用户 / Profile / 配置 / 审计 / 任务 / 外部认证 / Yggdrasil 信息 / 关于
  - Vitest 单测 + Playwright E2E，openapi-typescript 反向生成 API 类型

- **API 文档**
  - utoipa 5 + utoipa-swagger-ui 9，debug + openapi feature 下挂载 Swagger UI 与 OpenAPI JSON
  - api-docs-macros 逐路由 path 注解，静态 openapi.json 生成 + 前端类型反向生成

- **测试套件**
  - Rust 集成测试 ~200+ 用例 / ~15000 行，覆盖认证（含 Passkey）/ OAuth2 / OIDC / Yggdrasil（47 用例）/ CORS / 缓存 / 配置 / 审计 / 管理任务 / 多数据库后端
  - 前端单测（stores / 页面 / i18n / CSS）+ Playwright E2E 全流程

- **部署与构建**
  - 多阶段 Dockerfile：Node 24 + Bun 构建前端，Rust 1 Alpine 构建后端（预编译依赖缓存层 + strip）
  - 最终基于 alpine:3.23，内置 SQLite，非 root（UID 10001）运行，内置 HEALTHCHECK
  - docker-compose 开箱即用（挂载 ./data），镜像 ghcr.io/astercommunity/asteryggdrasil
  - 前端 rust-embed 内嵌进二进制，支持 ./frontend-override 运行时覆盖，CSP / PWA / SPA fallback / 分级缓存
  - 文档站点中英双语（快速上手 / 配置 / 存储 / Profile / Yggdrasil API / 材质 / 启动器登录 / 审计任务 / Docker）

### Dependencies

- **Web**: actix-web 4.13, actix-governor 0.10, actix-multipart 0.7
- **ORM**: sea-orm 2.0.0-rc.40（SQLite / MySQL / PostgreSQL）+ sea-orm-migration
- **认证**: jsonwebtoken 10, argon2 0.5, webauthn-rs 0.6.1-dev, openidconnect 4.0
- **邮件**: lettre 0.11（rustls）
- **加密 / 签名**: rsa 0.10, sha1 0.11, sha2 0.11, md-5 0.11
- **缓存**: moka 0.12, redis 1.2
- **图像**: image 0.25（jpeg / png / gif / webp）
- **API 文档**: utoipa 5.5, utoipa-swagger-ui 9.0
- **运行时**: tokio 1, tracing 0.1, tracing-subscriber 0.3
- **分配器**: tikv-jemallocator 0.7（可选）
- **前端**: React 19, Vite 8, Tailwind CSS 4, @base-ui/react, Zustand 5, react-router-dom 7, skinview3d 3.4, axios, i18next 26

---

**统计数据**：
- 596 files changed, 130,592 insertions(+)
- 2 commits
- Rust ~55,500 行（src + migration + 宏），测试 ~15,300 行
- 前端 189 个 TS/TSX 文件
- Rust Edition 2024, MSRV 1.94.0

[Unreleased]: https://github.com/AptS-1547/AsterDrive/compare/v0.1.0-alpha.1...HEAD
[v0.1.0-alpha.1]: https://github.com/AptS-1547/AsterDrive/releases/tag/v0.1.0-alpha.1
