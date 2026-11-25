# Changelog

All notable changes to AsterYggdrasil will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Prometheus 抓取端点支持 Basic 认证**：新增 `[observability.prometheus.basic_auth]`（`enabled` / `username` / `password_hash`）。密码存 Argon2id PHC 验证串，校验走实例独立的 Argon2 运行时（默认并发 2），预算耗尽立即拒绝，与登录验证互不挤占工作内存；未通过时返回 `401`、`WWW-Authenticate: Basic` 和 `Cache-Control: no-store`，响应体为空，成功响应同样禁止缓存。
- **Argon2 并发数可配置**：静态配置新增 `[auth] password_hash_max_concurrency`，默认 `2`（取自 `aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY`）。每个 Argon2 任务约占用 64 MiB 工作内存，默认最坏并发工作内存约 128 MiB。修改后需要重启。
- **audit 关机恢复**：进程关闭时审计写入器完成最终 drain；仍有未确认记录时，Forge 将其写入私有恢复目录 `data/audit-recovery/audit-{uuid}.json`（0600 文件、0700 目录），关机以错误收尾并报告路径，等待运维手工核对入库。系统不会自动 replay 这些记录。

### Changed

- **observability 配置接入 Forge 共享校验管线**：静态配置改用 Forge `aster_forge_metrics::ObservabilityConfig`（删除本地复制的默认值、脱敏 Debug、OTLP 映射与 provider 组装），根配置 derive `aster_forge_config::ValidateConfig`，加载走 `aster_forge_config::load_config`（段级校验 + 跨段部署规则一次执行，未编译 exporter 的配置段在反序列化时置零）。metrics 资源身份经 `MetricsIdentity` 提供，`service_instance_id` 取自 config-sync runtime id，profile namespace `com.astercosm.yggdrasil` 与 `astercosm-metrics/v1` schema token 由 Forge 常量统一。
- **Argon2 默认并发从 8 降到 2**：现有配置未显式设置 `password_hash_max_concurrency` 时，升级后每个进程的最坏并发 Argon2 工作内存从约 512 MiB 降到约 128 MiB。需要更高并发的部署请显式配置。
- **Forge #90 批次升级**：全部 `aster_forge_*` 依赖升级到已发布的 crates.io 批次（audit/auth/cache/config/crypto/db/db_migration/external_auth/http/logging/mail/metrics/middleware/observability/panic/runtime/tasks/test/utils）。panic hook 的 crash-log 祖先目录改由 bootstrap 显式创建（`data/`）；Linux 保持私有 crash log 行为，macOS 等平台 Forge 私有文件机制尚未支持，启动时降级为不可用并打印警告（fail closed），不影响正常服务。
- **替换 GPL 限流依赖**：移除 `actix-governor`（GPL-3.0-or-later），管理端 IP 限流改用 Forge `aster_forge_middleware` 0.1.0-alpha.6 的 GPL-free Actix 中间件（直接包装 MIT OR Apache-2.0 的 `governor` crate）。429 响应 envelope、burst/period 语义、trusted-proxy 客户端 IP 分桶和跨 worker 共享配额行为保持不变。`cargo deny check` 的 licenses 检查在无 exception 的情况下恢复通过。

### Security

- **修复未知账号登录的时序枚举**：未知标识符的虚拟 Argon2 验证改用与已存在账号相同的等待语义（`verify_password`）。此前使用非阻塞的 `try_verify_password`，许可池饱和时立即返回，攻击者可用并发请求制造饱和并凭响应时间区分账号是否存在。默认并发降为 2 后触发门槛更低。

## [v0.1.0-beta.1] - 2026-06-20

### Release Highlights

**AsterYggdrasil 进入 Beta 阶段！** 经过 6 个 alpha 版本的迭代，核心功能（Yggdrasil 全协议、皮肤站、多登录方式、材质库审核工作流、operator 细粒度管理）已趋于稳定，本版本转向 beta 通道：聚焦运营治理与可扩展性。

- **用户能力封禁系统** — 管理员可按 scope 精确封禁用户的特定能力（Yggdrasil 认证 / 加入服务器 / Profile 管理 / 材质上传 / 材质库互动），支持永久或限时、公开理由、撤销与 append-only 事件审计，无需禁用整个账号
- **Yggdrasil 上游 session 转发** — `hasJoined` 校验可编排本地与远程（含正版 Mojang）Yggdrasil 兼容 session 服务器，按 priority/weight 依次探测、可代理上游材质，支持混合认证架构
- **全系统 cursor 分页** — 所有列表端点从 offset 分页迁移到 cursor 分页，大数据量下性能显著提升（复合索引 + 固定排序）
- **外部认证增强** — 邮箱补验、密码绑定、auto-provisioning/linking、邮箱域名白名单、Microsoft tenant、OIDC claim 映射，第三方登录更灵活可控

### Breaking Changes

- **列表端点统一改为 cursor 分页**：审计日志、后台任务、用户、Profile、材质、会话、Passkey、邀请等列表端点从 offset 分页（`{ items, limit, offset, total }`）改为 cursor 分页。游标基于复合键 `(created_at, id)`（部分表 `(updated_at, id)`），响应新增 `next_cursor` / `has_more` 字段；查询参数移除 `sort_by` / `sort_order`，改为固定 `created_at DESC, id DESC` 排序，分页入参由 `limit` + `after_created_at` / `after_id`（或字符串游标）驱动。迁移 `m20260619_000001_cursor_pagination_indexes` 为受影响表补齐复合索引以支撑游标查询。按旧 offset / 排序参数调用列表端点的第三方脚本与自定义客户端需适配新响应结构与查询参数。

### Added

- **用户能力封禁系统（user capability bans）**：管理员可对用户发起细粒度能力封禁，按 scope 精确阻断特定操作而无需禁用整个账号。封禁 scope 共 5 个：`yggdrasil_access`（Yggdrasil 认证 / token validate）、`yggdrasil_join`（session join / hasJoined）、`minecraft_profile_manage`（profile 重命名）、`texture_upload`（皮肤 / 披风上传）、`texture_library_interact`（公共库复制 / 提交 / 举报）。封禁状态 `UserBanStatus`：`active` / `revoked` / `expired`；响应额外提供 `effective_status`（`active` 但已过 `expires_at` 时返回 `expired`）与 `effective` 布尔。封禁支持时间窗口 `starts_at` / `expires_at`（`expires_at` 留空即永久）、`reason`（必填，≤128 字符）、`public_reason` / `admin_note`（可选，≤1000 字符）、`revoke_note`。管理员 API：`GET /api/v1/admin/user-bans`（cursor 分页，支持 `user_id` / `scope` / `status` / `effective_only` 过滤）、`GET /api/v1/admin/user-bans/{ban_id}`、`POST /api/v1/admin/users/{user_id}/bans`（创建，事务内建封禁 + append-only `created` 事件）、`PATCH /api/v1/admin/user-bans/{ban_id}`（更新，仅 active 可改，改 scope 前校验同 scope 无其他有效封禁）、`POST /api/v1/admin/user-bans/{ban_id}/revoke`（撤销 + `revoked` 事件）、`GET /api/v1/admin/user-bans/{ban_id}/events`（append-only 事件历史）。强制执行：`yggdrasil_access` 在 `authenticate` / `validate` 拦截，`yggdrasil_join` 在 `join` / `hasJoined` / Minecraft Services 拦截，`minecraft_profile_manage` 在 profile 重命名拦截，`texture_upload` / `texture_library_interact` 在衣橱上传、公共库复制 / 提交 / 举报拦截，命中返回 `user_ban.forbidden`（`auth_forbidden` 系列）。审计动作 `AdminCreateUserBan` / `AdminUpdateUserBan` / `AdminRevokeUserBan`，新增 entity type `UserBan`，前端用户详情页新增 `UserDetailBanSection`（中英 i18n）。迁移 `m20260620_000002_user_bans` 新建两张表：`user_bans`（含 `user_id` CASCADE / `created_by` / `revoked_by` SET NULL 三组 FK，索引 `idx_user_bans_user_status` / `idx_user_bans_status_created` / `idx_user_bans_expires_at`）与 append-only `user_ban_events`（`ban_id` CASCADE / `actor` SET NULL，记录 created / updated / revoked 的前后状态 / scope / 过期时间，索引 `idx_user_ban_events_ban_created` / `idx_user_ban_events_actor_created`）。

- **Yggdrasil 上游 session 转发（session forwarding）**：`sessionserver/hasJoined` 校验时可编排本地与远程 Yggdrasil 兼容 session 服务器。新表 `yggdrasil_session_forward_servers`（迁移 `m20260620_000001`，内置本地 provider + 一个禁用的 Mojang 远程条目作为种子）。session forward service 按 `priority` / `weight` 依次选择服务器调用上游 hasJoined，命中后可选经新的 `forwardedTextures` 路由代理上游材质。管理员 CRUD API `GET/POST /api/v1/admin/yggdrasil/session-forward-servers`、`GET/PATCH/DELETE /api/v1/admin/yggdrasil/session-forward-servers/{id}`（统一受 `settings` operator scope 约束），前端新增 `AdminYggdrasilForwardingPage`（排序、健康状态、enable / texture-forward 开关）。审计：forwarding server 的 create / update / delete 及每次转发检查均落审计（中英 i18n）。新增文档页《Yggdrasil Forwarding》（中英）并接入文档站导航。

- **外部认证增强**：外部认证身份补齐已验证邮箱的本地校验流程（provider 回调未带已验证邮箱时走本地邮箱补验）；支持基于密码绑定外部身份；新增 auto-provisioning / auto-linking 配置开关、provider 的 allowed email domains 过滤、Microsoft tenant 配置（`consumers` / `organizations` / `common` / custom）、OIDC claim mappings 自定义；外部认证成功后提供 redirect toast 提示。

- **cursor 分页基础设施**：新增 `DateTimeIdCursor` / `DateTimeStringCursor` 游标类型与 `CursorPage` 响应结构（`items` / `total` / `limit` / `next_cursor` / `has_more`），为全系统列表端点的游标分页提供统一抽象。

- **审计动作与错误码扩展**：新增审计动作 `admin_create_user_ban` / `admin_update_user_ban` / `admin_revoke_user_ban`（group `admin`）与 Yggdrasil session forwarding server 管理及转发检查动作；新增 entity type `UserBan`。新增错误码：`user_ban.not_found` / `user_ban.already_active` / `user_ban.not_active` / `user_ban.duration_invalid` / `user_ban.reason_invalid` / `user_ban.forbidden`。

### Changed

- **依赖升级与 MSRV 调整**：`sysinfo` 升至 0.39，`rust-version`（MSRV）提升至 1.95.0；顺带升级 `aws-sdk-s3` / `redis` / `sea-orm` / `sea-orm-migration` / `@base-ui/react` / `@types/node` / `@typescript/native-preview`，移除未使用的 `papaparse` 依赖及其 vite chunk。

---

**统计数据**：
- 213 files changed, 20810 insertions(+), 4222 deletions(-)
- 4 commits（3 功能 + 1 依赖升级）
- 新增 3 个数据库迁移（`m20260619_000001` cursor 分页索引、`m20260620_000001` `yggdrasil_session_forward_servers`、`m20260620_000002` `user_bans` + `user_ban_events`）

## [v0.1.0-alpha.6] - 2026-06-19

### Breaking Changes

- **存储后端统一，旧头像与本地材质路径需迁移**：texture 与 avatar 存储合并到单一 `ObjectStorage` 后端。① 头像 storage_key 统一加 `avatar/` 前缀（`avatar/user/{id}/v{version}/{size}.webp`），升级后数据库里旧 key 的头像将无法读取（返回 `AvatarNotFound`），**用户需重新上传头像**；② `config.toml` 段 `[texture_storage]` 更名为 `[object_storage]`（旧段名经 `#[serde(alias = "texture_storage")]` 仍可加载），默认 `local_root` 由 `textures` 改为 `storage`——**本地部署升级前需把 `textures/` 目录迁移到 `storage/`，或显式配置 `local_root`**；③ 头像存储现在也支持 S3/MinIO（此前仅本地）。

- **管理员鉴权模型变更**：所有 `/api/v1/admin/*` 端点不再仅要求 `Admin` 角色，改为 `Admin` **或** 持有对应 scope 的 `Operator`。使用普通 admin token 的第三方脚本不受影响；新增 operator 账号可访问原本 admin-only 端点的子集。

- **管理员 avatar 端点路径迁移**：`GET /api/v1/admin/users/{id}/avatar/{size}` → `GET /api/v1/admin/avatars/users/{id}/{size}`，调用方需更新路径（前端 `adminService` 已同步）。

- **系统配置分类细分**：`site` / `auth` / `network` / `audit` / `runtime` / `yggdrasil` 拆为多个子分类（如 `auth.session` / `auth.registration` / `auth.recovery` / `auth.login` / `auth.email_policy` / `yggdrasil.metadata` 等），管理后台设置 UI 分类与 `SYSTEM_CONFIG_ALLOWED_CATEGORIES` 白名单随之变化——按旧分类名查询的外部脚本需适配。

### Added

- **公共材质库（public texture library）**：面向所有用户的公共皮肤 / 披风浏览与选用体系。公共浏览无需登录：`GET /api/v1/texture-library/tags`（标签列表，支持 keyword 搜索 + 分页）、`GET /api/v1/texture-library/textures`（公共材质列表，支持 `keyword`、`texture_type`、`tag_ids` 逗号分隔最多 16 个、`tag_search_method`=`all`|`any` 默认 `all`）、`GET /api/v1/texture-library/textures/{texture_id}`（详情）。公共库过滤条件为 `is_wardrobe_item = true AND visibility = Public`。登录用户可 `POST /api/v1/texture-library/textures/{texture_id}/copy` 将公共材质复制到自己衣橱（body `CopyPublicTextureReq { display_name?: NullablePatch<String> }`，后端按 hash + user_id + texture_type + model 命中既有衣橱条目则复用，否则建 `visibility=Private` 副本并复制源材质全部标签绑定）。衣橱端点扩展：`GET /api/v1/wardrobe/tags`、`PATCH /api/v1/wardrobe/textures/{texture_id}`（更新 display_name / visibility）、`PUT /api/v1/wardrobe/textures/{texture_id}/tags`（批量替换标签），`POST /api/v1/wardrobe/textures/{texture_type}` 上传 multipart 新增可选 `name` / `display_name` 字段。数据层迁移 `m20260618_000004_texture_library_metadata`：给 `minecraft_textures` 加 `display_name`（VARCHAR 96，null）、`library_status`（VARCHAR 24，default `private`，索引 `idx_minecraft_textures_library_status`）；新建 `minecraft_texture_tags`（`normalizedName` unique、`color` default `#64748b`、`sortOrder`，索引 `idx_minecraft_texture_tags_name_unique` / `idx_minecraft_texture_tags_sort`）与 `minecraft_texture_tag_binding`（双向 CASCADE，复合 unique `idx_minecraft_texture_tag_bindings_unique(texture_id, tag_id)`）两张表。

- **材质库审核工作流（review）**：`MinecraftTextureLibraryStatus` 状态机 `private` / `pending_review` / `published` / `rejected`。普通用户提交：`review_required=true` 走 `private`→`pending_review`，`false` 直接 →`published`（立即写 `library_reviewed_at`，reviewer 留空）；撤回回到 `private` 并清空时间戳。管理员（`RequireAdminOrScope(OperatorScope::TextureLibrary)`）端点：`GET /api/v1/admin/texture-library/textures`（支持 keyword / texture_type / visibility / library_status / published / tag_ids / tag_search_method 过滤）、`GET /api/v1/admin/texture-library/textures/{texture_id}`、`POST .../approve`（body `ReviewTextureLibraryTextureReq { review_note?, tag_ids? }`，要求 `visibility=public`）、`POST .../reject`（`review_note` 必填，≤512 字符）、`POST .../unpublish`（note 可选，副作用：调用 `handle_pending_for_texture` 把该材质所有 `pending` 举报自动置为 `accepted`）。迁移 `m20260618_000006_texture_library_review` 给 `minecraft_textures` 加 `library_submitted_at` / `library_reviewed_at` / `library_reviewer_user_id`（i64 null）/ `library_review_note`（text null）四列 + `idx_minecraft_textures_library_public` / `idx_minecraft_textures_library_review_queue` / `idx_minecraft_textures_user_library_status` 三索引。

- **材质库举报工作流（reports）**：新表 `minecraft_texture_reports`（迁移 `m20260618_000007_texture_library_reports`，三 FK：texture CASCADE / reporter CASCADE / handler SET NULL，索引 `idx_texture_reports_status_created` / `idx_texture_reports_texture_status` / `idx_texture_reports_reporter_status`）。`MinecraftTextureReportReason`：`inappropriate` / `offensive` / `copyright` / `misleading` / `broken` / `spam` / `other`；`MinecraftTextureReportStatus`：`pending` / `accepted` / `rejected`。用户端点 `POST /api/v1/texture-library/textures/{texture_id}/reports`（body `CreateTextureReportReq { reason 必填, message? ≤1000 字符 }`），前置校验：材质须 `published` + `visibility=public`、不能举报自己、同用户同材质仅一条 `pending`。管理员端点 `GET /api/v1/admin/texture-library/reports`、`GET .../reports/{report_id}`、`POST .../reports/{report_id}/accept`（副作用：把被举报材质下架 `library_status`→`private`，即 accept = 自动 unpublish）、`POST .../reports/{report_id}/reject`（仅改举报状态）；accept/reject 均要求举报当前为 `pending`。

- **衣橱→公共库提交工作流（wardrobe submission）**：普通用户把衣橱材质提交进公共库，挂 `/api/v1/wardrobe` scope——`POST /api/v1/wardrobe/textures/{texture_id}/library-submission`（前置 `texture_library_enabled=true`、材质须为当前用户衣橱项且 `visibility=public`，按 `texture_library_review_required` 决定进 `pending_review` 还是直接 `published`）、`DELETE /api/v1/wardrobe/textures/{texture_id}/library-submission`（撤回，清空审核字段回 `private`）。

- **Operator 角色与细粒度管理 scope**：`UserRole` 新增 `Operator`（DB 值 `"operator"`），形成 `Admin` > `Operator` > `User` 三级。`OperatorScope` 枚举 8 值（snake_case，DB 长度 32）：`overview` / `users` / `profiles` / `texture_library` / `audit` / `tasks` / `settings` / `external_auth`。迁移 `m20260618_000005_operator_scopes` 新建 `user_operator_scopes` 表（`userId` + `scope` + 复合 unique `(user_id, scope)`，FK→users CASCADE）。新增鉴权中间件 `RequireAdminOrScope(OperatorScope)`：`role.is_admin() || (role == Operator && operator_scopes.contains(scope))`，否则 `auth_admin_required`；`/admin/*` 每个子路由独立挂载对应 scope。安全规则 `normalize_role_and_scopes`：请求同时给 `role=Admin` 且带 `operator_scopes` 时自动降级为 Operator（避免带 scope 的 super admin）；super admin（`id=1`）角色 / 状态 / scopes 修改一律拒绝且不可删除；`AuthUserInfo` 仅对 `Operator` 加载 scopes。前端经 `GET /auth/check` 的 `operator_scopes` 字段驱动导航。

- **自托管图片验证码（captcha）系统**：基于 `captcha-rs` 本地渲染 JPEG base64（180×54~64 px），不依赖第三方服务；字符集剔除易混淆字符（仅 `2-9 A C D E F G H J K M N P Q R T W X Y`）。校验由 `RuntimeCaptchaPolicy` 在 `POST /auth/login`、`POST /auth/register`、`POST /auth/users/invitations/{token}/accept`、`POST /auth/register/resend-activation` 四端点按需触发 `verify_if_required`。挑战端点（公开）：`GET /auth/captcha/policy`（返回 `PublicCaptchaPolicyResp`，各 `*_required` = `enabled && 子开关`）、`POST /auth/captcha`（返回 `CaptchaChallengeResponse { challenge_id, image_base64, mime: "image/jpeg", expires_in }`）。挑战存缓存 key `auth:captcha:{challenge_id}`（SHA-256 answer hash + `attempts`/`max_attempts`/`expires_at`），失败累加达上限删除，answer 归一化（去空白 + 转大写）。配置组 `captcha`：`auth_captcha_enabled`（默认 `false`）、`auth_captcha_login_required` / `auth_captcha_register_required` / `auth_captcha_invitation_accept_required` / `auth_captcha_register_activation_resend_required`（默认均 `true`）、`auth_captcha_preset`（`readable` / `balanced` 默认 / `hardened`）、`auth_captcha_ttl_secs`（默认 120）、`auth_captcha_length`（4–8，默认 5）、`auth_captcha_max_attempts`（1–10，默认 3）。新增 config action `preview_captcha` 供管理后台预览不同参数下的图片（不写审计）。前端 `useCaptchaChallenge` + `CaptchaField` 组件接入登录 / 邀请 / 注册 / 重发激活表单。

- **CPU 材质预览渲染**：新增独立 crate `aster-texture-renderer`（CPU-only，无 GPU 依赖），为材质库提供 2D / 3D 预览 PNG。公开端点 `GET /api/v1/texture-previews/{hash}/{file_name}` 返回 `image/png`（走 `conditional_bytes_response`）。新增 `texture_preview_*` 运行时配置：`texture_preview_engine`（`skin-3d` 默认 / `skin-2d`）、`texture_preview_quality_profile`（`fast` / `default` / `quality`）、`texture_preview_width` / `texture_preview_height` / `texture_preview_background` / `texture_preview_show_outer_layer`，及 3D 参数（`scale` / `pitch` / `front_yaw` / `back_yaw` / `spacing` / `x_offset` / `y_offset` / `center_y` / `supersampling`）与 2D 参数（`padding` / `spacing`）。

- **审计动作与错误码扩展**：新增审计动作 `minecraft_texture_library_submit` / `_withdraw`（group `user`）、`minecraft_texture_library_approve` / `_reject` / `_unpublish`（group `admin`）、`minecraft_texture_report_create`（group `user`）、`minecraft_texture_report_accept` / `_reject`（group `admin`）。新增错误码：`auth.captcha_required` / `auth.captcha_invalid` / `auth.captcha_expired`、`wardrobe.texture_name_invalid` / `wardrobe.texture_name_taken`、`texture_library.tag_not_found` / `tag_name_invalid` / `tag_color_invalid` / `tag_name_taken` / `texture_not_found` / `disabled` / `texture_not_public` / `texture_not_pending` / `texture_not_published` / `review_note_invalid`、`texture_report.texture_not_reportable` / `self_report_not_allowed` / `pending_exists` / `message_invalid` / `not_found` / `not_pending`。

- **材质库运行时配置**：新增 `texture_library_enabled`（默认 `true`）与 `texture_library_review_required`（默认 `true`），经 `PublicTextureLibraryConfig` 暴露到 `/api/v1/frontend-config`，前端据此控制侧栏 / 页脚 / 首页 CTA 可见性。

### Changed

- **存储后端统一（object_storage）**：`TextureStorage` trait 改名为 `ObjectStorage`，metadata 结构 `TextureBlobMetadata` → `ObjectBlobMetadata`，工厂 `create_object_storage()` 接受 `""` / `local` / `s3` / `minio`；`AppState.texture_storage` → `object_storage`，runtime trait `TextureStorageRuntimeState` → `ObjectStorageRuntimeState`。`avatar_storage.rs` 由 174 行瘦身到 74 行，删除本地目录清理逻辑（`cleanup_empty_avatar_dirs` / `normalize_absolute_path`），改用 `object_storage().put_file` / `get_stream` / `delete`。错误码 `MinecraftTextureStorageFailed` → `MinecraftObjectStorageFailed`，wire code 字符串 `minecraft_texture.storage_failed` 保持不变（协议层兼容）。无 schema 迁移，配置层变更经 `ensure_defaults` 的 `delete_deprecated_keys()` 启动清理。升级影响见上方 Breaking Changes。

- **衣橱材质元数据响应结构**：`MinecraftWardrobeTextureMetadata` 新增 `name` / `display_name` / `library_status` / `tags` / `preview_url` 字段；上传 multipart 新增可选 `display_name` / `name` 字段。

- **PWA 更新检测增强**：`usePwaUpdate` 抽出 `requestServiceWorkerUpdate(reg, reason)`（按 `registered` / `interval` / `visibility` reason 打 debug 日志并捕获 `update()` 失败），注册成功后立即检查一次，新增 `visibilitychange` 回前台检查，effect teardown 清理 interval 与监听；`vite.config.ts` 的 `VitePWA.registerType` 由 `prompt` 改为 `autoUpdate`，新版自动应用而非提示用户确认（附带修复了旧版注册 interval 永不清理的资源泄漏）。

- **conditional_bytes_response 增加 `Content-Length` 响应头**，影响所有 bytes 响应（avatar / 静态资源 / texture / 默认皮肤）的客户端缓存与范围请求行为。

- **新增系统配置值类型 `StringEnum`**（含 `string_enum_set` 存储），供 captcha preset 等单选枚举使用；直接读 `system_config` 表的外部工具需感知新类型。

- **孤儿材质清理匹配收紧**：`maintenance.rs` 仅匹配合法的两位 hex 前缀 + 64 位 hex hash 的 storage key，避免误删。

- **健壮性加固**：`src`、`main`、`migration`、`aster-texture-renderer` 四个 crate 启用 `clippy::panic` deny，shutdown / mail sender / rate limiter / external auth providers / config helpers 中的 `expect` / `unwrap` / `panic` 替换为错误传播。

### Removed

- 移除系统配置项 `avatar_dir`（含 `normalize_avatar_dir_config_value`、`DEFAULT_AVATAR_DIR`），启动时由 `delete_deprecated_keys()` 自动清理；头像根目录现由 `[object_storage]` 统一决定。

---

**统计数据**：
- 275 files changed, 33,142 insertions(+), 3,866 deletions(-)
- 4 commits（3 功能 + 1 重构）
- 新增 4 个数据库迁移（`m20260618_000004` ~ `m20260618_000007`），新表 `minecraft_texture_tags` / `minecraft_texture_tag_binding` / `user_operator_scopes` / `minecraft_texture_reports`，新字段 `minecraft_textures.display_name` / `library_status` / `library_submitted_at` / `library_reviewed_at` / `library_reviewer_user_id` / `library_review_note`

## [v0.1.0-alpha.5] - 2026-06-18

### Added

- **S3 / MinIO 存储后端**：`texture_storage.backend` 配置为 `s3` 或 `minio`（两者等价）时启用 `S3TextureStorage`，`local` 保持本地实现。基于 `aws-sdk-s3`，兼容 AWS S3 / MinIO / RustFS 等任意 S3 兼容服务（经 `endpoint` + `force_path_style` 适配），默认 region `us-east-1`，超时 connect 5s / read 30s / operation 60s。新增配置项 `[texture_storage.s3].base_path`（桶内前缀，默认空）。**关键行为：数据库里的 storage_key 永远不含 base_path 前缀**——S3 后端在每次对象存储调用时动态拼接 `base_path/key`，返回上层仍为裸 key，本地与 S3 后端可无损切换。上传走服务端 streaming（`ByteStream::from_path`，content-type 固定 `image/png`），不提供 presigned URL 客户端上传。key 净化拒绝绝对路径 / `\` / 尾斜杠 / `..` / 空段。404 统一映射 `record_not_found`，其余 SDK 错误带 `http_status`/`code`/`message`/`source chain` 完整记录。集成测试用 rustfs testcontainers 验证。

- **管理员概览工作台**：新增 `GET /api/v1/admin/overview`（`JwtAuth` + `RequireAdmin`），返回汇总 / 服务状态 / 系统健康 / 活跃趋势 / 最近活动 / 系统信息。汇总含 7 个计数（总用户、Minecraft profile 数、材质数、活跃 session、活跃 Yggdrasil token、processing / pending 后台任务）；服务状态含 5 项固定巡检（database / yggdrasil / session / texture_storage 显示 backend 名 / background_tasks，pending>0 降级 warning）；活跃趋势固定 7 天 UTC 日聚合，每天 4 指标（活跃用户去重、活跃玩家按 Yggdrasil 认证动作、新增材质、Yggdrasil API 调用次数）；系统健康合并最新 `SystemHealthCheck` 与 `YggdrasilStorageConsistencyCheck` 任务结果（worst status 合并，failed→Unhealthy、pending→Degraded）。前端新增 `AdminOverviewPage`（路由 `/admin`：hero 健康状态 + 健康横幅 + 汇总卡片 + 趋势图 + 最近审计 + 服务状态 + 快捷入口 + 系统信息）与 `OverviewTrendChartContent`（Recharts 折线图，4 条线 lazy 加载），配套 `StatusIndicator` 组件与中英 i18n。

- **CDN / 对象存储 texture URL**：新增运行时配置 `yggdrasil_texture_public_base_url`（默认空，`requires_restart=false`）。对已上传材质（有 storage_key），textures property 的 URL 优先使用 `{texture_public_base_url}/{storage_key}`，未配置则回退既有 `{public_base_url}/textures/{hash}` 或 `/api/yggdrasil/textures/{hash}`，三者全空时报 config error。`texture_public_base_url` 的 host 自动并入 skinDomains 白名单（去重）。配合 S3/MinIO 后端可将材质直走 CDN / 对象存储公网域名，绕开 Yggdrasil API 路由。

- **通用 HTTP base URL 规范化工具**：新增 `utils/url.rs::normalize_http_base_url`（trim + 去尾斜杠 + 校验 http/https + host，可选禁止 query/fragment）。`normalize_public_base_url`、`normalize_required_public_base_url`、`normalize_gravatar_base_url_config_value`、S3 `endpoint` 规范化统一改用该工具。

- **审计日志活跃聚合索引**：新增迁移 `m20260618_000003_audit_log_activity_indexes`，在 `audit_logs` 上创建复合索引 `idx_audit_logs_action_created_user(action, created_at, user_id)`，覆盖管理员概览 7 天活跃趋势的 `COUNT(DISTINCT user_id) WHERE action IN (...) AND created_at BETWEEN ...` 等聚合查询，避免全表扫描。

- **文档站导航重排**：文档站从 2 大组拆为 6 大组（开始 / 玩家使用 / 接入协议 / 管理维护 / 部署 / 项目参考），新增双语页面：使用指南总览、管理员指南、常见问题速查、故障排查（按症状分流）、启动器填写、文档贡献说明、部署总览（上线前检查 / 公开 URL / 反向代理 / 持久化 / 备份 / 验收）。`storage.md` S3/minio 从「预留未实现」补全为完整使用说明（含 base_path、CORS 要求、CDN URL），`configuration.md` 补充 `yggdrasil_texture_public_base_url` 与 CORS 要求。

### Changed

- **存储一致性检查改为只校验 storage key 格式**（性能 + 安全）：旧实现逐个 texture 调 `exists()` + `get_stream()` 重算 SHA-256 比对（N 次 streaming）；新实现改用一次 `list_keys("")` 拉全量 key 集 + `HashSet` 判存在性，hash 比对改为只校验 storage_key 是否符合 `{hash前2位}/{hash}.png` 格式。S3 后端下避免 N+1 次 head/get_object，大幅提速。`TextureErrorKind::Storage` 的 protocol_message 统一为 "Texture storage failed."，不再透传 endpoint/bucket 等内部细节（安全加固）。

- **textures property URL 优先级变化**：已上传材质的 URL 现在优先走对象存储 URL——配置 `yggdrasil_texture_public_base_url` 后路径形态从 `/textures/{hash}` 变为 `{base}/{storage_key}`，客户端 / 启动器缓存的旧 property 会自然刷新，skinDomains 已自动覆盖新 host。

- **base URL 校验收紧（breaking）**：`public_site_url`、`yggdrasil_public_base_url`、gravatar base URL、S3 endpoint 现在统一拒绝带 query 或 fragment 的 URL（此前 public_base_url 会静默忽略）。既有带 `?` / `#` 的配置在保存时会报校验错误。

- **gravatar base URL 空白回退默认值**：runtime 空白值现在回退 `https://www.gravatar.com/avatar`，此前只 trim 返回空串。

- **一致性检查失败摘要增强**：失败 summary 现在列出最多 5 条具体 issue（`missing object: texture #N, key K, expected hash H` / `hash/key mismatch: ...`），超出显示 "and N more"。

- **failed runtime task 健康状态回退**：`system_health` 为 None 时回退错误状态文案，不再显示空状态。

### Fixed

- **外部认证 provider 查询 N+1**：`external_auth_provider_repo` 新增 `find_by_ids`，`list_links_paginated` 由原 `find_all`（全表扫描）+ 内存过滤改为单次 `WHERE id IN (...)` 查询，provider 表较大时性能显著改善。

---

**统计数据**：
- 126 files changed, 8,042 insertions(+), 401 deletions(-)
- 3 commits（1 功能 + 1 重构 + 1 文档）
- 新增 1 个数据库迁移（`m20260618_000003`）

## [v0.1.0-alpha.4] - 2026-06-18

### Added

- **邮箱注册激活流程**：`POST /auth/register` 按 `auth_policy.register_activation_enabled` 分叉——开启且非首位用户时创建 `Active` 但 `email_verified_at` 为空的用户，事务内签发 `RegisterActivation` 校验 token 并 outbox 投递激活邮件，返回 `{ expires_in: 0, requires_activation: true }` 且不下发 cookie；首位用户（自动成为 Admin）与关闭激活的部署仍走旧即时登录路径。新增 `POST /auth/register/resend`（带 `register_activation_resend` 节流，响应不泄露账号是否存在），`GET /auth/contact-verification/confirm` 按 `VerificationPurpose` 分发：`RegisterActivation` 激活账号写 `UserConfirmRegistration` 审计并 302 到 `/login?contact_verification=register-activated`，`ContactChange` 提交 `pending_email`，`PasswordReset` 不在此消费。

- **密码重置与改密**：新增 `POST /auth/password/reset/request`（按 email 发 `PasswordReset` token，受 `auth_password_reset_request_cooldown` 节流，响应固定文案不泄露账号存在性）、`POST /auth/password/reset/confirm`（消费 token，token 过期返回 410，写 `UserConfirmPasswordReset`）、`PUT /auth/password`（已登录改密，需账号已验证邮箱、校验当前密码、禁止新旧相同，事务内 `update_password_in_connection` + `revoke_sessions_for_user` 并重发 token，逻辑集中在新模块 `auth_service/password_change`，审计 `UserChangePassword`）。配套前端 `/reset-password`、`/force-password-change` 路由与状态横幅。

- **用户邀请系统**：仅管理员可发起。新表 `user_invitations`（`token_hash` 唯一、`status` pending/accepted/expired/revoked、`invited_by` Cascade、`accepted_user_id` Set Null、四组索引），TTL 由新配置 `auth_user_invitation_ttl_secs`（默认 7 天）控制。管理端 `POST/GET /admin/users/invitations` 与 `POST /admin/users/invitations/{id}/revoke`，创建时事务内归一化邮箱 + 校验本地邮箱策略 + 生成 token + 吊销同邮箱旧 pending + outbox 投递邀请邮件，响应含 `invitation_url` 与 `mail_queued`；公开端 `GET /auth/invitations/{token}`（无需登录，返回 email / expires_at）、`POST /auth/invitations/{token}/accept`（无需登录，事务内校验 pending/未过期/邮箱策略/邮箱未占用，创建 `Active` 且邮箱已验证的普通用户，CAS `mark_accepted_if_pending` 防并发，审计 `UserRegister`，HTTP 201）。错误码 `AuthInvitationInvalid/Expired/Accepted/Revoked`。审计动作 `AdminCreateInvitation`/`AdminRevokeInvitation`，新增 `AuditEntityType::Invitation`。

- **邀请邮件模板**：新增 `MailTemplateCode::UserInvitation`（`user_invitation`），模板文件 `user_invitation.html` + `user_invitation.subject.txt`，占位符 `{{email}}`、`{{invitation_url}}`、`{{site_name}}`、`{{expires_in}}`；新增可配置项 `mail_template_user_invitation_subject`（String）与 `mail_template_user_invitation_html`（Multiline），管理后台邮件模板分组可编辑，过期时长用 `format_mail_duration_seconds` 渲染。

- **Contact verification token 基础设施**：新表 `contact_verification_tokens`（`channel` email / `purpose` register_activation|contact_change|password_reset / `token_hash` 唯一 / `consumed_at`），含 `idx_contact_verification_tokens_single_active` 表达式唯一索引（每用户每 channel+purpose 同时仅一个未消费 token，三库兼容写法）。Repository `contact_verification_token_repo` 提供 `create`/`find_by_token_hash`/`find_latest_active_for_user`/`delete_active_for_user`/`mark_consumed_if_unused`（CAS 防并发消费）/`delete_expired`。`VerificationChannel`、`VerificationPurpose` 落 `DeriveActiveEnum`。

- **邮箱修改流程**：`users` 表新增 `pending_email`（唯一索引）。`POST /auth/email/change` 校验当前账号已验证邮箱、新邮箱通过本地策略且未被占用，事务内写 `pending_email` + 签发 `ContactChange` token + outbox 投递确认邮件，审计 `UserRequestEmailChange`；`POST /auth/email/change/resend` 带 resend 节流，审计 `UserResendEmailChange`；在 `confirm` 端点完成验证后提交 `pending_email` 为正式 email 并清空，审计 `UserConfirmEmailChange`。

- **强制改密（must_change_password）**：`users` 表新增 `must_change_password`（独立迁移 `m20260618_000002`）。新模块 `auth_service/token_scope`：`must_change_password=true` 时签发的 token 带 `password_change` claim，请求仅放行 `(GET /auth/me, PUT /auth/password, POST /auth/logout)` 白名单，其余返回 `AuthPasswordChangeRequired`（错误码 `auth.password_change_required`）；`auth_service` 双向校验 claim 与 DB 状态一致。前端 `/force-password-change` 路由 + `AuthenticatedGate`/`GuestOnlyGate` 在检测到该标记时引导跳转。

- **管理员创建用户：生成密码 + 强制改密**：`POST /admin/users` 的 `CreateAdminUserReq.password` 改为 `Option<String>`（不传则后端生成临时密码并强制 `must_change_password=true`），username 校验改用 `validate_auth_username`（与普通注册一致的 4-16 位规则）。响应改为 `CreateAdminUserOutput { user, generated_password }`。审计 details 增加 `temporary_password_generated`。前端 `GeneratedPasswordDialog` 一次性展示明文临时密码（只读 + 复制，提示不再展示）。`UpdateAdminUserReq` 同步支持 `must_change_password` 与统一的 8-128 密码上下限。admin user 写路径从 `auth_service` 抽到新模块 `admin_user_service`，并保护 `id=1` 超管不可改 role/status。

- **用户删除（硬删 + 级联）**：`DELETE /admin/users/{id}` 调 `admin_user_service::delete_user`，`id=1` 超管禁止删除。级联清理顺序：删本地 avatar 文件 + 清字段 → 遍历该用户 minecraft profile 逐个 `delete_profile_for_user`（含解绑并清无引用 texture blob、撤销选中该 profile 的 yggdrasil token）→ 删全部衣柜材质（含 blob）→ 撤销所有浏览器 session → 撤销所有 launcher token → `users` 表删除（其余表由 FK `on_delete` 级联）。响应 `DeleteAdminUserOutput` 返回各级联清理计数。审计 `AdminDeleteUser`，前端 `audit.ts` 标为 `danger` tone。

- **Yggdrasil 默认皮肤**：`steve.png` / `alex.png` 经 `include_bytes!` 内嵌进二进制（`texture_service/default_skin`），`for_profile_uuid` 按 UUID 最低位奇偶返回 Alex（Slim）/ Steve（Default），解析失败回退 Steve。`texture_metadata_for_profile` 在无 skin binding 时自动追加默认皮肤（`source = Default`），`properties.rs::texture_property_value` 改为无条件推 textures property 保证协议层可见，`texture_by_hash` 在 DB 未命中时以内嵌默认皮肤兜底返回（200 + `image/png` + immutable ETag + 304 支持）。默认皮肤 SHA-256 常量由测试守护。

- **Minecraft Services 兼容层**：新增 Bearer token 鉴权、Mojang 风格 `{"path": "..."}` 错误体的兼容端点——`POST /minecraftservices/player/certificates`（即时生成 2048 位 RSA profile key，`expiresAt = now+48h`、`refreshedAfter = now+36h`，签名项用 dummy）、`GET /minecraftservices/privileges`（`permissive_privileges` 默认全开）、`GET /minecraftservices/player/attributes`、`GET /minecraftservices/privacy/blocklist`（空）、`GET /sessionserver/blockedservers`（404），未匹配路径统一 404。新增运行时配置开关 `yggdrasil_enable_profile_key`（默认 true）与 `yggdrasil_enable_mojang_anti_features`（默认 true）。

- **Yggdrasil metadata 扩展**：`YggdrasilMeta` 新增 `links: { homepage, register? }`（homepage 基于 public_site_url；register 仅当 `allow_user_registration` 时出现），`feature` 新增 `enable_profile_key`、`enable_mojang_anti_features`、`username_check`（固定 true）。

- **材质 source 字段 + ETag/条件请求**：`MinecraftTextureMetadata` 新增 `source`（`bound` / `default`）枚举。新模块 `api/cache` 提供 `weak_etag_for_*`、`request_etag_matches`（认弱/强 ETag、`*`、列表）、`conditional_bytes_response`（自动 ETag、命中 304）。前端静态资源、avatar、texture、默认皮肤全部接入 ETag + `If-None-Match` 304。

- **材质尺寸校验**：后端 `validate_texture_dimensions` 按 type 校验——skin 需 64×32 或 64×64 等比放大，cape 需 64×32 或 22×17（22×17 旧式 cape 仍自动 padding 到 64×32）。新增前端 `minecraftTextureValidation.ts` 上传前预校验（MIME/大小/PNG 签名/像素数/尺寸，镜像后端规则），`PublicYggdrasilConfig` 新增 `max_texture_upload_bytes` 与 `max_texture_pixels` 透传给前端预校验。

- **Profile / Wardrobe 搜索过滤**：`GET /profiles/minecraft` 新增 `query`（最多 64）走名字模糊匹配；`GET /wardrobe/textures` 新增 `keyword`（最多 96）与 `texture_type`，keyword 走 hash/mimetype 模糊 + 归一化后类型/模型名快捷匹配（输入 `skin`/`slim` 等触发对应列过滤）。query 解析错误经 `project_query_config` 转为 `RequestMalformed`，避免泄露 actix 原始错误体。

- **皮肤头像组件**：新增 `MinecraftSkinAvatar`（从 skin texture URL 用两个 8 倍放大 + 像素化的 `<img>` 叠加裁出头部 8×8 区域，失败回退 User 图标并防重复重试），用于 account profiles 与 admin 用户/Profile 详情页；`MinecraftPreview` 改为 `lazy` 加载（three/skinview3d 拆独立 vendor chunk），`MinecraftPreviewPanel` 提供 Suspense + Skeleton 占位。

- **公开法律页面**：新增 `/tos`、`/privacy` 路由与通用 `LegalPage`（含目录、Last updated 标签、中英 i18n `public.json.legal.*`），Footer 增加链接。

- **侧边栏折叠**：桌面端 shell 侧边栏支持折叠/展开并持久化到 localStorage，新增 `AnimatedCollapsible` 高度动画容器用于 filter toolbar 与子项。

- **前端基础设施**：新增 `lib/storage.ts`（统一 localStorage/sessionStorage JSON 访问 + SSR 守护）、`lib/validation.ts`（基于 `zod/v4` 的 username/email/password schema 集合）、`lib/contactVerificationRedirect.ts` / `passwordResetRedirect.ts`（URL 状态解析横幅）、`lib/user.ts`（`getUserDisplayName`）、`DateTimeText`（`<time dateTime title>` 包装的可访问时间组件，替换全站内联 `formatDateTime`）、`AuthFormPrimitives`（登录/初始化/重置/强制改密/邀请表单共享原子）、`GeneratedPasswordDialog`。

- **Yggdrasil API 文档**：新增 `developer-docs/{en,zh-CN}/yggdrasil-api.md`（完整协议实现指南：路由根、metadata、authserver/sessionserver/texture API、Minecraft Services 兼容层、wire 字段、错误形状、运行时配置、OpenAPI、测试边界），`docs/guide/profiles.md` 改为推荐 controlled rename API 并警告禁止直改数据库。

### Changed

- **密码长度上限收紧（breaking）**：`validate_password` 由 8-256 改为 8-128（前端 `passwordSchema` 同步），admin 创建/更新用户密码校验统一走 `validate_optional_auth_password`。

- **管理员创建用户响应结构（breaking）**：`POST /admin/users` 响应由 `AdminUserInfo` 改为 `CreateAdminUserOutput { user, generated_password }`，前端消费方需改读 `.user`。

- **邮箱统一小写（breaking）**：`normalize_email` 在注册、邀请、admin 创建用户、email change 等入口统一归一化为小写后再落库与查询。

- **Yggdrasil 认证要求已验证邮箱（breaking）**：`yggdrasil_service/auth::authenticate` 在密码校验前拒绝 `email_verified_at` 为空的账号（未激活账号无法走 Yggdrasil 登录），对外仍返回 `InvalidCredentials` 避免泄露状态。

- **移除粗粒度外部认证开关（breaking）**：移除运行时配置项 `external_auth_enabled`、`external_auth_auto_register` 及 `external_auth` 分类常量（i18n key 同步删除）。外部认证 provider 本身的管理（AdminExternalAuthPage、相关路由与 repo）仍保留可用，依赖这两个 key 的现有部署需迁移。

- **缓存策略统一**：`avatar_image_response` helper 删除，`AVATAR_CACHE_CONTROL`/`AVATAR_CONTENT_TYPE`/`TEXTURE_CACHE_CONTROL` 提升可见性，所有 avatar、静态资源、texture 路由改走 `conditional_bytes_response`。

- **公开 Yggdrasil 配置**：`PublicYggdrasilConfig` 新增 `max_texture_upload_bytes`、`max_texture_pixels`，前端 `frontendConfigStore` 同步暴露驱动预校验。

- **本地邮箱策略错误码**：allowlist/blocklist 拒绝改用带错误码的 `validation_error_code`（`AuthEmailNotAllowlisted`/`AuthEmailBlocked`），前端可针对性提示。

- **审计动作扩展**：新增 9 个审计动作（`user_confirm_registration`、`user_request_email_change`、`user_resend_email_change`、`user_confirm_email_change`、`user_request_password_reset`、`user_confirm_password_reset`、`admin_create_invitation`、`admin_revoke_invitation`、`admin_delete_user`）与 `Invitation` entity type。

- **注册端点响应类型**：前端 `authService.register` 由 `AuthTokenResponse` 改为 `RegisterResponse { expires_in, requires_activation }`，调用方需按 `requires_activation` 决定展示激活提示还是进工作台。

- **properties 构建**：`build_profile_properties` 不再判断 `textures.is_empty()`，无条件构建 textures property 以支持默认皮肤注入。

- **Admin settings / about 页面**：settings 页移除分类描述卡片与每个 group 标题旁的数量 Badge；about 页改链外部文档站、调整 brand mark 尺寸与资源链接图标。

### Fixed

- **错误响应一致性**：列表 query 解析错误统一经 `project_query_config` 转 `RequestMalformed`；本地邮箱策略拒绝改用带错误码响应，避免泄露 actix 原始错误体。

- **可访问性**：全站时间戳改用 `<time dateTime title>` 包装（`DateTimeText`），屏幕阅读器与浏览器可获取机器可读时间，可见文本仍按 i18n locale 格式化。

- **路由滚动行为**：`AppLayout` 路由切换（无 hash）自动滚顶，带 hash 锚点的链接保留位置。

- **认证链路日志**：`auth::check`/`auth::me` 新增 `tracing::debug!`，便于排查 token 链路。

### Removed

- 移除配置项 `external_auth_enabled`、`external_auth_auto_register` 与 `external_auth` 分类（含对应 i18n key）。
- 移除 `avatar_image_response` 内联 helper、全站内联 `formatDateTime` 调用（统一改 `DateTimeText`）。

---

**统计数据**：
- 258 files changed, 29,992 insertions(+), 5,818 deletions(-)
- 4 commits（4 功能）
- 新增 2 个数据库迁移（`m20260618_000001`、`m20260618_000002`），新表 `contact_verification_tokens`、`user_invitations`，新字段 `users.pending_email`、`users.must_change_password`

## [v0.1.0-alpha.3] - 2026-06-16

### Added

- **列表端点统一分页**：将 profiles、wardrobe、会话、passkey、外部认证 link / provider 等列表端点从裸数组改为 offset 分页响应。受影响端点返回 `OffsetPage<T>`（含 `items` / `limit` / `offset` / `total`）：`GET /profiles/minecraft`、`GET /wardrobe/textures`、`GET /auth/external-auth/providers`、`GET /auth/external-auth/links`、`GET /admin/users/{id}/minecraft-profiles`；auth_session / minecraft_texture / external_auth_identity / passkey 仓储新增 `list_by_user_paginated`，并补对应服务层分页包装。

- **账户区 API 与工作台**：新增 `/account` 路由组。`GET /account/overview` 返回当前用户的 profile 数量与最近 5 条活动（审计日志）；`GET /account/audit-logs` 提供当前用户审计日志的分页 + 过滤（action / entity_type / entity_id / 时间范围）+ 排序查询，user_id 边界在服务端强制，不被前端覆盖。前端配套 `accountService`、`AccountOverviewPage` 工作台与 `AccountAuditPage`（`/account/audit`），中英 i18n 同步。

- **前端导航与路由架构重构**：统一 shell —— `AccountShell` 与 `AdminShell` 合并为单一参数化 `AppShell`；路由守卫抽成独立组件 `AuthenticatedGate` / `GuestOnlyGate` / `AdminOnlyGate` / `InitGate`；用 `RouteAccessState` 取代静默 redirect，访问被拒时展示上下文提示而非无感跳转；所有路径集中到 `routePaths.ts` 并提供动态段助手。导航改为按账户 / 管理分区的 `ShellNavSection` 可折叠结构。主题切换改为双图标交叉淡入动画并补 `data-theme-surface`（AsterDrive 风格柔和过渡）；密码强度计改为红 / 琥珀 / 翠绿分档配色；移除 topbar 搜索栏并为大体积 chunk 配置 PWA precache 排除清单。

### Changed

- **路由与命名规范化（breaking）**：`/dashboard` → `/account`，`/dashboard/admin/*` → `/admin/*`；移除 `/auth`、`/dashboard/launcher`、legacy `/app/*` 等旧路由，不再保留自动重定向。页面命名统一：`ProfilesPage` → `MinecraftProfilesPage`、`WardrobePage` → `TextureWardrobePage` 等；通用组件从 `panel/` 命名空间迁到 `common/`；i18n 由 `dashboard.json` 拆出 `account.json`，并新增路由状态文案。

- **列表响应格式（breaking）**：上述列表端点响应从裸数组变为 `{ items, limit, offset, total }` 分页对象，第三方 / 自定义客户端需适配。

- **审计排序类型共享**：`AdminAuditLogSortBy` 重命名为 `AuditLogSortBy`，排序查询处理移入 `audit_service::filters`，用户与管理端共用同一套排序定义。

### Removed

- 移除面向开发者的旧概念页面：Yggdrasil 信息页、诊断面板（`ServiceDiagnosticsPanel` / `useServiceDiagnostics` / `diagnosticsService`）、API catalog（`EndpointCard` / `SectionTitle` / `AccessBadge`）、forge 引导（`FoundationSummary` / `ModuleRail`）、`ExternalAuthPage`、`AuthPage`、`AdminConfigPage`、`AdminYggdrasilPage`、`WorkbenchPage`，以及 `legacyRedirects`。

---

**统计数据**：
- 148 files changed, 6,840 insertions(+), 4,050 deletions(-)
- 2 commits（2 功能）
- 无新增数据库迁移

## [v0.1.0-alpha.2] - 2026-06-16

### Added

- **Profile 重命名与 token 临时失效**：用户与管理员均可重命名 Minecraft profile（用户端 `PUT /api/v1/profiles/minecraft/{uuid}/name`，管理端 `PUT /api/v1/admin/minecraft-profiles/{uuid}/name`）。重命名时绑定该 profile 的 Yggdrasil token 进入临时失效态——阻止 `validate` / `join`，但允许 `refresh`（清除失效并签发带新名字的新 token），强制启动器在游戏前拉取更新后的 profile 元数据。名称唯一性校验（拒绝重复、同名视为 no-op），新增 `m20260616_000001` 迁移给 `yggdrasil_tokens` 加 `temporarily_invalidated_at` 列，记录 `MinecraftProfileRename` 审计事件（旧名 / 新名 / 失效 token 数）；前端用户 profile 页与管理员 profile 详情页加重命名对话框，配套中英 i18n 与审计展示。

- **Yggdrasil 协议限流**：为 `authserver/authenticate` 与 `authserver/signout` 新增独立令牌桶限流（防暴力撞库）。基于 governor keyed 限流器，按规范化用户名（trim + 小写）分桶，`authenticate` 与 `signout` 各自独立桶互不影响；触发时返回 `429` + `Retry-After` 头，错误体遵循 authlib-injector 协议格式（`TooManyRequestsException`）。复用 `rate_limit.auth` 档位配置。

### Fixed

- **Yggdrasil 协议**：`uploadableTextures` profile property 此前硬编码 `signature: None`，违反 authlib-injector 规范。现在 `unsigned=false` 与 sessionserver `hasJoined` 服务端验证场景下，该 property 会与 textures property 一样经 RSA 私钥签名，服务端可完整验证全部 profile properties。

### Changed

- **限流默认开启**：`RateLimitConfig.enabled` 默认值由 `false` 改为 `true`，`config.example.toml` 同步。新部署默认对认证类请求启用限流；已有部署 `config.toml` 的显式值优先，不受影响。

- **文档**：修正 `clientToken` 在 `invalidate` 中的行为描述 —— invalidate 按 Yggdrasil 规范只校验 `accessToken`，`clientToken` 不参与吊销判定（端点行为未变，仅修正此前误导性文档，中英同步）。
- **文档**：`hasJoined` 与 `profile/{uuid}` 的签名表述由「textures property 签名」泛化为「profile properties 签名」，与新签名行为对齐（中英同步）。

---

**统计数据**：
- 64 files changed, 2,108 insertions(+), 50 deletions(-)
- 4 commits（3 功能 + 1 CI）
- 新增 1 个数据库迁移（`m20260616_000001`）

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

[Unreleased]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-beta.1...HEAD
[v0.1.0-beta.1]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.6...v0.1.0-beta.1
[v0.1.0-alpha.6]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.5...v0.1.0-alpha.6
[v0.1.0-alpha.5]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.4...v0.1.0-alpha.5
[v0.1.0-alpha.4]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.3...v0.1.0-alpha.4
[v0.1.0-alpha.3]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.2...v0.1.0-alpha.3
[v0.1.0-alpha.2]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[v0.1.0-alpha.1]: https://github.com/AsterCommunity/AsterYggdrasil/releases/tag/v0.1.0-alpha.1
