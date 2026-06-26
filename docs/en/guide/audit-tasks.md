# Audit and Background Tasks

AsterYggdrasil writes audit logs for admin operations and important protocol actions. Periodic maintenance uses the runtime task record and presentation system instead of a separate worker stack.

## Audit Scope

Yggdrasil-related audited actions include:

- Creating and deleting Minecraft profiles.
- Uploading and deleting textures.
- authenticate, refresh, invalidate, signout.
- join server.
- Admin profile or texture deletion.
- Yggdrasil signing key rotation config action.

Audit details are generated from structured types, and sensitive values are not written:

- access tokens are not logged in plaintext.
- client tokens are not treated as exposed credentials.
- signing private keys never appear in audit details.
- serverId is recorded as a hash.

## Admin API

```text
GET /api/v1/admin/audit-logs
GET /api/v1/admin/tasks
POST /api/v1/admin/tasks/cleanup
POST /api/v1/admin/tasks/{id}/retry
```

Tasks and audit logs include presentation fields. Frontends should use stable presentation codes, titles, and details instead of parsing internal payloads to build display text.

## Runtime Tasks

The service instance runs periodic tasks:

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

Yggdrasil-specific tasks:

- `yggdrasil-token-cleanup`: deletes expired or revoked tokens.
- `yggdrasil-storage-consistency-check`: checks whether texture DB rows point to missing objects and whether object storage keys still match their recorded hashes.
- `yggdrasil-texture-cleanup`: deletes orphan objects with no DB references.

## Multiple Instance Boundary

Periodic maintenance, mail outbox dispatch, audit cleanup, and texture consistency checks have global side effects. The current runtime assumes a single task owner; production deployments should run a single service instance or ensure externally that only one instance starts the full background task set.

Future multi-instance high availability should be carried by Forge runtime lease/lock support instead of reintroducing product-level instance-role branches.

## Operational Advice

- Review failures from `yggdrasil-storage-consistency-check` regularly.
- If consistency check reports missing objects, do not immediately run cleanup; first verify whether storage was manually deleted or unmounted.
- If orphan cleanup deletes an unexpected number of objects, check whether profile or texture deletion was triggered in bulk.
- After key rotation, if servers fail to verify textures properties, refresh metadata on the client or server side first.
