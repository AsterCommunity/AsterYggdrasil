# AsterYggdrasil

AsterYggdrasil 是自建 Minecraft 皮肤站 + Yggdrasil 认证服务器项目。

当前代码来自通用 Rust + React 服务模板，已经有 HTTP 服务、本地认证、外部认证、运行时配置、邮件、审计、后台任务、OpenAPI、嵌入式前端和部署基础设施。不要把模板 README 或旧前端 demo 当成最终产品定义，后续开发目标是围绕 authlib-injector/Yggdrasil API、玩家档案、材质管理和管理后台落地。

## 工作前必须先看

- 先读现有代码模式，再动手。看不清模式就停下问 1547，别凭感觉硬写。
- Yggdrasil/API 行为以 `tmp/authlib-injector/wiki` 为参考入口。该目录当前只包含 wiki 索引和拖拽添加服务器示例，完整规范文件可能尚未同步；涉及协议细节时必须补齐依据，不要臆造字段或响应。
- 当前 `frontend-panel/` 是模板/demo 级后台，不是产品形态。前端可以参考技术栈和服务层写法，但不要照抄页面结构、视觉风格或信息架构；很可能要整体重写。
- 这个仓库是模板代码来源演进来的项目。修改时要区分“基础设施能力”和“产品域能力”，别把云盘、文件管理、团队分享之类旧模板概念带回来。

## 项目结构

```text
src/                         Rust 后端
src/api/                     路由、DTO、OpenAPI 注册、中间件、响应封装
src/cache/                   cache trait 以及 memory/noop/Redis 实现
src/config/                  静态配置、运行时配置定义、配置规范化
src/db/                      数据库连接、重试、事务、repository
src/entities/                SeaORM Entity
src/metrics/                 metrics feature 下的 Prometheus 实现
src/runtime/                 AppState、启动、关闭、日志、后台任务循环
src/services/                Auth、external auth、config、mail、audit、task、health、example
src/types/                   共享枚举和 DB wrapper 类型
src/utils/                   crypto、ID、path、number、email、RAII 等工具
migration/                   SeaORM migration crate
api-docs-macros/             OpenAPI 辅助宏
frontend-panel/              React + Vite 管理前端，目前只是模板/demo
developer-docs/              开发说明
docs/                        用户/部署文档站
tests/                       集成测试和 OpenAPI 导出测试
tmp/authlib-injector/wiki/   authlib-injector/Yggdrasil 参考资料入口
```

## 技术栈

- 后端: Rust 2024, actix-web 4, SeaORM 2.0-rc, tokio, jsonwebtoken, argon2
- 数据库: SQLite 默认，兼容 MySQL/PostgreSQL
- 缓存: memory/noop/Redis 后端
- 前端: React 19, Vite, TypeScript, Tailwind CSS 4, shadcn/ui, Biome, Vitest, Playwright
- OpenAPI: `utoipa` + `api-docs-macros`
- 嵌入: `rust-embed` 将 `frontend-panel/dist/` 编译进二进制

## 开发命令

```bash
# 后端
cargo run
cargo check
cargo test
cargo test --lib <test_filter>
cargo test --test <test_name> <test_filter>
cargo test --features openapi --test generate_openapi
cargo test --features metrics

# 指定集成测试数据库后端
ASTER_TEST_DATABASE_BACKEND=sqlite cargo test --test test_database_backends
ASTER_TEST_DATABASE_BACKEND=postgres cargo test --test test_database_backends
ASTER_TEST_DATABASE_BACKEND=mysql cargo test --test test_database_backends

# 前端
cd frontend-panel
bun install
bun run dev
bun run build
bun run check
bun run test
bun run test:e2e
```

跑单元测试时优先缩小范围，避免没必要地编译全包。批量修复后再跑 `cargo check` 和相关测试。

## 当前已有能力

- 本地认证: setup/register/login/refresh/logout/me/sessions
- 外部认证: provider 配置、登录流程、回调基础结构
- 管理 API: runtime config、audit logs、external auth providers、background tasks
- 邮件: SMTP runtime 配置、模板变量、持久 outbox、测试邮件、邮件审计
- 审计: buffered async audit、展示层元数据、管理端查询
- 后台任务: task record、dispatch、lease/heartbeat、retry、cleanup、presentation
- 运行时: Forge runtime 组件装配、优雅关闭、健康检查、metrics、CORS、CSRF、安全头、限流

这些是地基，不等于 Minecraft/Yggdrasil 域已经完成。新增 Yggdrasil 能力时应该复用这些基础设施，而不是另起一套认证、配置、任务或邮件系统。

## 需要实现的产品域

后续新增代码应围绕这些方向组织，命名要直接表达 Minecraft/Yggdrasil 语义：

- Yggdrasil 认证服务: metadata、authenticate、refresh、validate、invalidate、signout、join、hasJoined 等协议端点
- authlib-injector 兼容: API 根地址、拖拽添加 URI、签名密钥、材质签名策略、启动器/服务端兼容性
- 玩家档案: Minecraft UUID、玩家名、角色/档案绑定、名称唯一性和删除策略
- 材质系统: skin、cape、elytra 等材质上传、哈希、MIME/尺寸校验、公开读取、缓存头
- 管理后台: 用户、玩家档案、材质、认证客户端、密钥、审计、系统配置

不要用“文件存储/分享/团队/回收站/缩略图”这类旧云盘领域名来表达新业务。需要保存材质文件时，也要从 Minecraft 材质域建模，而不是复活云盘的文件模型。

## 玩家档案生命周期

- Minecraft profile name 当前支持受控改名。用户和管理员 rename API 必须保持 UUID、材质绑定和审计链路不变，并临时失效已绑定该 profile 的 Yggdrasil token，让启动器通过 refresh 获取新名称。
- 删除 profile 时必须同时处理材质记录、存储对象、启动器 token 失效和审计。
- profile name 唯一性以当前存活 profile 为准。当前没有名称历史表；后续如果引入软删除、保留删除记录或名称历史，必须先明确旧名称是否释放，不能顺手加表。

## API 约定

### 项目 API

模板管理 API 使用统一响应体：

```json
{ "code": "success", "msg": "", "data": { } }
```

失败使用稳定字符串错误码，定义在 `src/api/error_code.rs` 的 `AsterErrorCode`。内部错误类型是 `src/errors.rs` 的 `AsterError`，通过 `ResponseError` 统一转 HTTP 响应和日志。

新增项目后台/管理 API 应继续使用这套 envelope 和 `AsterErrorCode`。

### Yggdrasil 协议 API

Yggdrasil/authlib-injector 兼容端点必须优先满足协议响应格式，不能为了项目内部 envelope 破坏客户端兼容性。也就是说：

- 协议端点按 Yggdrasil/authlib-injector 规范返回字段、状态码和错误体。
- 管理端和站点自身 API 才使用 Aster envelope。
- 如果协议错误格式与 `AsterError` 不一致，单独建协议错误映射层，不要污染全局错误系统。
- 所有协议字段必须有测试覆盖，尤其是 UUID 格式、accessToken/clientToken 行为、selectedProfile、textures property、签名和时间戳。

## 后端代码约定

- 路由模块放在 `src/api/routes/`，每个模块暴露 `configure(cfg: &mut web::ServiceConfig)` 或 `routes()`，按现有注册方式接入 `src/api/routes/mod.rs`。
- DTO 放在 `src/api/dto/`，领域共享类型放在 `src/types/`，不要在 handler 里散落匿名 JSON 拼装。
- 业务逻辑放 `src/services/`，数据库访问放 `src/db/repository/`，handler 只做认证、参数提取、调用 service、返回响应。
- 新表必须有 SeaORM entity 和 migration，测试覆盖 SQLite；涉及数据库差异时同时考虑 MySQL/PostgreSQL。
- 配置项统一定义在 `src/config/definitions.rs`，由 `system_config_service::ensure_defaults()` 初始化。不要在业务代码里写散落默认值。
- 运行时共享状态走现有 `AppState`/runtime 初始化路径，不要引入全局可变单例。
- 需要后台异步处理的用户可见任务，优先复用 `task_service` 的 task record/dispatch/retry/presentation 结构。
- fire-and-forget 操作用 `if let Err(error) = ... { tracing::warn!(...) }`，不要静默 `let _ =`。
- 数据库事务失败不用手写多余 `rollback()`；SeaORM transaction drop 会自动回滚。

## 类型和数据库约定

- 枚举字段优先使用 `DeriveActiveEnum` 或明确的强类型 wrapper，禁止魔法字符串在 service/repo 间传来传去。
- 数据库列不要为了省事直接上 JSON。除非确实需要数据库侧 JSON 查询、索引或约束，否则结构化内容用 `TEXT` 存储，并在代码层用强类型 DTO + serde 校验。
- 禁止跨层裸写 `as i32` / `as usize` / `as i64` 做静默截断；使用 `src/utils/numbers.rs` 的 checked conversion helper。
- 多数据库 SQL 要保守：
  - 不用 SQLite-only 标量 `MAX(a, b)`，改用 `CASE WHEN ...`
  - 多表 join 下 `COUNT`/`GROUP BY` 必须显式限定列来源
  - 原子计数优先封装在 repo 函数里
- UUID、token、hash、key id 等安全敏感字段要用专门类型或清晰命名，避免 `String` 到处裸传导致用错。

## 安全约定

- access token、refresh token、client token、会话 secret、签名私钥只能存哈希或加密后的必要形式；日志、审计、错误消息不得泄露明文。
- Yggdrasil 材质签名涉及私钥管理。私钥加载、轮换、导出公钥、签名算法必须有明确配置和测试。
- 登录、注册、认证协议端点必须接入现有限流/安全头/CORS/CSRF 策略；协议兼容需要豁免时必须写清楚理由并加测试。
- 用户名、玩家名、邮箱、URL、MIME、图片尺寸、文件大小都要在 DTO/service 边界校验。
- 公开材质读取接口要设置合理 Cache-Control，但不能让私有或未发布资源被缓存泄露。

## 前端约定

`frontend-panel/` 当前只是模板 demo。可以保留以下技术栈和底层工具：

- Vite + React + TypeScript
- Biome lint/format
- Vitest/jsdom
- Playwright e2e
- shadcn/ui 基础组件
- 生成的 OpenAPI 类型和 service 层思路

但不要继承 demo 的产品设计：

- 页面结构、导航、视觉风格、文案、信息架构都可以重写。
- 新前端应围绕皮肤站和认证服务器真实工作流设计：登录/注册、角色档案、材质上传与预览、authlib-injector 接入、管理员配置、用户与审计。
- 不要做营销落地页当首屏；这是一个工具/管理系统，首屏应该是可用工作台或认证入口。
- 管理界面要信息密度适中、可扫描、可重复操作，不要堆装饰性卡片。
- 图标优先用项目已有 icon 封装或组件库，不要手写 SVG。
- TypeScript 保持 `erasableSyntaxOnly` 思路：不要用 TS enum，用 `as const` 对象；类型导入使用 `import type`。

## 测试要求

- 新增后端行为至少补对应单元测试或集成测试。协议兼容、鉴权、安全、数据库迁移必须有测试。
- 新增 Yggdrasil 端点时，测试要覆盖成功响应、错误响应、token 生命周期、profile/texture 字段、兼容性边界。
- 修改迁移、repo 或 SQL 时，至少跑 SQLite 相关测试；跨数据库逻辑要考虑 `ASTER_TEST_DATABASE_BACKEND=postgres|mysql`。
- 修改前端服务层或关键 UI 流程时，跑 `bun run test`；涉及页面流程时补/跑 Playwright。
- code review comments 进来时，先分辨真问题还是误报；真实问题分批修，修完每批都编译/测试。别被机器人牵着鼻子走。

## 文档和命名

- 文档可以更新，但不要主动写长篇使用说明，除非任务明确要求。
- README/docs 里仍有模板描述时，修改相关功能时顺手纠正直接相关部分，不做无关大清洗。
- 命名要面向领域：`yggdrasil`, `minecraft_profile`, `texture`, `skin`, `cape`, `authlib_injector`。不要使用旧云盘词汇伪装新业务。
- 保留 MIT 许可证约束。可参考 Cloudreve 等项目的领域概念，但不得复制 GPL 代码。

## 参考资料

- authlib-injector wiki: `tmp/authlib-injector/wiki`
- authlib-injector 拖拽添加示例: `tmp/authlib-injector/wiki/src/yggdrasil-server-dnd-example.html`
- 开发文档: `developer-docs/README.md`
- 用户文档: `docs/index.md`
- 配置示例: `config.example.toml`
