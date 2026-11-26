# 日志规范

本文定义 AsterYggdrasil 的日志纪律：什么进日志、什么绝不进日志、级别怎么选、日志和审计的分工。新代码按这里执行；review 里发现违规按本文纠正。

机制层（声明式字段合同、错误分类、稳定 JSON 封套）由 Forge 负责：跟踪 [AsterForge#50](https://github.com/AsterCommunity/AsterForge/issues/50)（字段合同与封套）与 [#45](https://github.com/AsterCommunity/AsterForge/issues/45)（OTEL tracing 与生命周期）。本页是产品侧策略——#50 明确不定义产品字段语义与隐私策略，那部分归我们。

## 日志和审计是两个系统

| | 日志（tracing） | 审计（`audit_writer`） |
| --- | --- | --- |
| 受众 | 运维（排障进程行为） | 管理员/用户（谁对什么做了什么） |
| 载体 | stdout / 日志文件，易失 | DB 表，管理端可查 |
| 标识 | 技术上下文（task_id、user_id） | actor + entity + action |
| 内容 | 技术事件与失败 | 业务与安全事件 |

同一个事件两边都可能记，但字段和受众不同。绝不用日志替代审计，也不把审计内容搬进日志流。

## 基础设施

`aster_forge_logging::init_logging`（bootstrap 调用，全局只装一次）：

- `level` 默认 `info`；`RUST_LOG` 优先级更高，且冲突时以 warning 说明哪个生效
- `format`：`text` / `json`
- `file` 为空写 stdout；非空走日轮转 + `max_backups` 保留数
- 文件写失败降级 stdout 并报告 warning；重复安装 subscriber 不 panic，只 warning

不需要为单条日志做任何配置；排障调 `RUST_LOG` 级别即可。

## 级别语义

| 级别 | 用法 | 例子 |
| --- | --- | --- |
| `warn` | 可恢复异常、降级、忽略的输入——运维要看见但不用报警 | mail outbox dispatch 失败后重试、配置项解析失败被忽略 |
| `debug` | 日常操作细节；排障时 `RUST_LOG` 打开 | 登录拒绝、token 创建、清理任务执行了多少行 |
| `info` | 进程里程碑，**默认部署下 info 日志要能讲清进程的一生** | 启动、配置加载、任务入队提交、密码哈希策略升级 |
| `error` | 需要报警的失败 | 运行时组件启动失败 |
| `trace` | 几乎不用 | — |

纪律基线：`info` 极少（当前全库 15 条，全部是生命周期事件）；`debug` 是主力；`warn` 用于一切"失败了但进程继续"。逐请求 access log **不存在**——请求可观测走 Prometheus 指标（`MetricsMiddleware`）+ request id 中间件，不要重新引入 `middleware::Logger` 之类的逐请求日志。

## 字段纪律

- **结构化字段优先于字符串插值**：`tracing::debug!(user_id, "login rejected")`，不要 `tracing::debug!("login rejected for user {username}")`。
- **标识只用数字 ID**（`user_id` / `task_id` / `texture_id`），不用 username / email / profile name。需要人的信息去审计表关联。
- **存在性代替值**：敏感上下文只记是否存在，不记内容——`has_user_agent = user_agent.is_some()`、`has_ip_address = ip_address.is_some()`（token/session 路径已是此做法）。
- **错误对象进 warn/error**：`error = %error`，让 tracing 结构化承载，不要手工拼接。
- **生命周期事件分段命名**：事务边界要可见，例如任务入队区分 `record_inserted`（事务可能未提交）与 `enqueue_committed`（已提交）。

## 禁记清单

以下内容不进日志、错误消息、审计明文（与 AGENTS.md 安全约定一致）：

- access/refresh/client token、会话 secret、JWT、签名私钥
- 密码、MFA secret、外部认证凭据、SMTP/存储凭据
- 邮箱地址、IP 地址、User-Agent 的**值**（存在性布尔可以）
- 业务 payload、文件名、材质 hash 之外的存储 key 内容

**两个已知的坑，新代码必须避开：**

1. **错误文本会嵌输入原文。** `aster_forge_validation::ValidationError` 的 Display 形如 `invalid email policy domain '{value}'`；serde_json 的 invalid-type 分支也嵌值片段。透传 `error = %error` 等于把输入原文写进日志。对含个人数据的输入（邮箱、玩家名），记闭集分类（如 `parse/empty/length/non_ascii/malformed`）+ 长度/序号，不透传错误全文。参考 `src/config/local_email_policy.rs` 的做法与它的 `redaction_tests`。
2. **Debug 输出不是脱敏边界。** `SecretString` 的 Debug 是脱敏的，但普通结构体的 Debug 会带出全部字段——不要用 `%?` / `{:?}` 打印含用户数据的结构体。

## 脱敏回归测试

涉及"日志里不能出现 X"的行为，用 `aster_forge_test::logs::CapturedLogs` 写回归测试：捕获日志文本，断言不含敏感原文。参考 `src/config/local_email_policy.rs` 的 `redaction_tests` 模块。改日志字段时先跑对应的捕获测试。

## 机制层路线（Forge）

- **#50 字段合同**：字段白名单（未声明字段不进记录）、resource/scope/record 三层 JSON 封套 + 版本化 mapping 标识、错误以类别/错误码出现、落盘失败有界计数。产品侧在本页维护字段语义与隐私策略，落地时把它们映射进声明。
- **#45 OTEL tracing**：trace provider/propagator/生命周期与本地日志关联。产品负责 service 身份、采样信任与业务 span。
- 两票交付前，本页的运行时纪律就是全部防线；交付后按其迁移说明切换，本页保留产品策略部分。
