# 审计与后台任务

AsterYggdrasil 的管理员操作和协议关键行为会写入 audit log。周期维护工作使用 runtime task 记录和展示，不另起一套 worker。

## 审计范围

已接入审计的 Yggdrasil 相关动作包括：

- 创建和删除 Minecraft profile。
- 上传和删除 texture。
- authenticate、refresh、invalidate、signout。
- join server。
- 管理端删除 profile 或 texture。
- Yggdrasil 签名 key rotate config action。

审计 details 使用结构化类型生成，敏感信息不会写入 details：

- access token 不记录明文。
- client token 不作为可泄露凭据记录。
- 签名私钥不出现在 audit details。
- serverId 会使用 hash 形式记录。

## 管理 API

```text
GET /api/v1/admin/audit-logs
GET /api/v1/admin/tasks
POST /api/v1/admin/tasks/cleanup
POST /api/v1/admin/tasks/{id}/retry
```

任务和审计都有 presentation 字段，前端应使用 presentation 中的稳定 code、title 和 detail，不要解析内部 payload 来判断展示文案。

## Runtime Tasks

服务实例会运行周期任务：

```text
background-task-dispatch
mail-outbox-dispatch
system-health-check
auth-session-cleanup
external-auth-flow-cleanup
yggdrasil-token-cleanup
audit-cleanup
task-cleanup
yggdrasil-storage-consistency-check
yggdrasil-texture-cleanup
```

Yggdrasil 相关任务：

- `yggdrasil-token-cleanup`: 删除过期或已吊销 token。
- `yggdrasil-storage-consistency-check`: 检查 texture DB 记录指向的对象是否缺失，以及 object storage key 是否仍匹配记录的 hash。
- `yggdrasil-texture-cleanup`: 删除 storage 中没有 DB 引用的孤儿对象。

## 多实例边界

周期维护任务、邮件 outbox、审计清理和材质一致性检查都带有全局副作用。当前运行时假设只有一个任务 owner；生产环境先按单实例运行，或在外层部署确保只有一个实例启动完整后台任务。

多实例高可用后续应通过 Forge runtime 的租约/锁能力承载，而不是在产品配置里恢复旧的主从分支。

## 运维建议

- 定期查看 `yggdrasil-storage-consistency-check` 的失败记录。
- 如果 consistency check 报 missing object，不要直接重跑清理；先确认 storage 是否被人工删除或挂载丢失。
- 如果 orphan cleanup 删除数量异常增大，检查 profile/texture 删除流程是否被批量触发。
- key rotate 后如果服务端验签失败，优先让客户端或服务端刷新 metadata。
