import { withQuery } from "@/lib/query";
import type {
	AdminAuditLogQuery,
	AdminExternalAuthProviderInfo,
	AdminExternalAuthProviderListQuery,
	AdminExternalAuthProviderPage,
	AdminMinecraftProfileInfo,
	AdminTaskCleanupRequest,
	AdminTaskListQuery,
	AdminTaskPage,
	AdminUserListQuery,
	AdminUserMinecraftProfileQuery,
	AdminUserPage,
	AuditLogPage,
	ConfigListQuery,
	ConfigSchemaItem,
	CreateAdminUserRequest,
	CreateExternalAuthProviderRequest,
	ExecuteConfigActionRequest,
	ExecuteConfigActionResponse,
	ExternalAuthProviderKindInfo,
	ExternalAuthProviderTestParamsRequest,
	ExternalAuthProviderTestResult,
	MinecraftTextureMetadata,
	OperationData,
	OperationPath,
	OperationRequestBody,
	RemovedCountResponse,
	RenameMinecraftProfileRequest,
	SetConfigRequest,
	SetConfigResponse,
	SystemConfig,
	SystemConfigPage,
	SystemInfoResponse,
	TemplateVariableGroup,
	UpdateAdminUserRequest,
	UpdateExternalAuthProviderRequest,
	YggdrasilProfilePage,
} from "@/types/api";
import { api } from "./http";

type AdminConfigPath = OperationPath<"get_config">;
type AdminExternalAuthProviderPath =
	OperationPath<"admin_get_external_auth_provider">;
type AdminRetryTaskPath = OperationPath<"admin_retry_task">;
type AdminUserPath = OperationPath<"admin_get_user">;

export const adminAuditService = {
	list: (params: AdminAuditLogQuery = {}) =>
		api.get<AuditLogPage>(
			withQuery("/admin/audit-logs", {
				limit: params.limit,
				offset: params.offset,
				user_id: params.user_id,
				action: params.action,
				entity_type: params.entity_type,
				entity_id: params.entity_id,
				after: params.after,
				before: params.before,
				sort_by: params.sort_by ?? "created_at",
				sort_order: params.sort_order ?? "desc",
			}),
		),
};

export const adminSystemService = {
	getInfo: () => api.get<SystemInfoResponse>("/admin/system-info"),
};

export const adminConfigService = {
	list: (params: ConfigListQuery = {}) =>
		api.get<SystemConfigPage>(
			withQuery("/admin/config", {
				limit: params.limit,
				offset: params.offset,
			}),
		),
	schema: () => api.get<ConfigSchemaItem[]>("/admin/config/schema"),
	get: (key: AdminConfigPath["key"]) =>
		api.get<SystemConfig>(`/admin/config/${encodeURIComponent(key)}`),
	set: (key: AdminConfigPath["key"], data: SetConfigRequest) =>
		api.put<SetConfigResponse, OperationRequestBody<"set_config">>(
			`/admin/config/${encodeURIComponent(key)}`,
			data,
		),
	templateVariables: () =>
		api.get<TemplateVariableGroup[]>("/admin/config/template-variables"),
	action: (key: AdminConfigPath["key"], data: ExecuteConfigActionRequest) =>
		api.post<
			ExecuteConfigActionResponse,
			OperationRequestBody<"execute_config_action">
		>(`/admin/config/${encodeURIComponent(key)}/action`, data),
	sendTestEmail: (targetEmail?: string) =>
		api.post<
			ExecuteConfigActionResponse,
			OperationRequestBody<"execute_config_action">
		>("/admin/config/mail/action", {
			action: "send_test_email",
			target_email: targetEmail?.trim() || null,
		}),
	rotateYggdrasilSignatureKey: () =>
		api.post<
			ExecuteConfigActionResponse,
			OperationRequestBody<"execute_config_action">
		>("/admin/config/yggdrasil/action", {
			action: "rotate_yggdrasil_signature_key",
		}),
	delete: (key: AdminConfigPath["key"]) =>
		api.delete<void>(`/admin/config/${encodeURIComponent(key)}`),
};

export const adminTaskService = {
	list: (params: AdminTaskListQuery = {}) =>
		api.get<AdminTaskPage>(
			withQuery("/admin/tasks", {
				limit: params.limit,
				offset: params.offset,
				kind: params.kind,
				status: params.status,
				sort_by: params.sort_by ?? "updated_at",
				sort_order: params.sort_order ?? "desc",
			}),
		),
	cleanup: (data: AdminTaskCleanupRequest) =>
		api.post<RemovedCountResponse, OperationRequestBody<"admin_cleanup_tasks">>(
			"/admin/tasks/cleanup",
			data,
		),
	retry: (id: AdminRetryTaskPath["id"]) =>
		api.post<OperationData<"admin_retry_task">>(`/admin/tasks/${id}/retry`),
};

export const adminMinecraftProfileService = {
	get: (uuid: string) =>
		api.get<AdminMinecraftProfileInfo>(`/admin/minecraft-profiles/${uuid}`),
	rename: (uuid: string, data: RenameMinecraftProfileRequest) =>
		api.put<
			AdminMinecraftProfileInfo,
			OperationRequestBody<"admin_rename_minecraft_profile">
		>(`/admin/minecraft-profiles/${uuid}/name`, data),
	listTextures: (uuid: string) =>
		api.get<MinecraftTextureMetadata[]>(
			`/admin/minecraft-profiles/${uuid}/textures`,
		),
	listByUser: (userId: number, params: AdminUserMinecraftProfileQuery = {}) =>
		api
			.get<YggdrasilProfilePage>(
				withQuery(`/admin/users/${userId}/minecraft-profiles`, params),
			)
			.then((page) => page.items),
	listByUserPage: (
		userId: number,
		params: AdminUserMinecraftProfileQuery = {},
	) =>
		api.get<YggdrasilProfilePage>(
			withQuery(`/admin/users/${userId}/minecraft-profiles`, params),
		),
	delete: (uuid: string) =>
		api.delete<void>(`/admin/minecraft-profiles/${uuid}`),
	deleteTexture: (uuid: string, textureType: "skin" | "cape") =>
		api.delete<void>(
			`/admin/minecraft-profiles/${uuid}/textures/${textureType}`,
		),
};

export const adminUserService = {
	list: (params: AdminUserListQuery = {}) =>
		api.get<AdminUserPage>(
			withQuery("/admin/users", {
				limit: params.limit,
				offset: params.offset,
				keyword: params.keyword,
				role: params.role,
				status: params.status,
				sort_by: params.sort_by ?? "created_at",
				sort_order: params.sort_order ?? "desc",
			}),
		),
	get: (id: AdminUserPath["id"]) =>
		api.get<OperationData<"admin_get_user">>(`/admin/users/${id}`),
	create: (data: CreateAdminUserRequest) =>
		api.post<
			OperationData<"admin_create_user", 201>,
			OperationRequestBody<"admin_create_user">
		>("/admin/users", data),
	update: (id: AdminUserPath["id"], data: UpdateAdminUserRequest) =>
		api.patch<
			OperationData<"admin_update_user">,
			OperationRequestBody<"admin_update_user">
		>(`/admin/users/${id}`, data),
	revokeSessions: (id: AdminUserPath["id"]) =>
		api.post<OperationData<"admin_revoke_user_sessions">>(
			`/admin/users/${id}/sessions/revoke`,
		),
};

export const adminExternalAuthService = {
	kinds: () =>
		api.get<ExternalAuthProviderKindInfo[]>(
			"/admin/external-auth/provider-kinds",
		),
	list: (params: AdminExternalAuthProviderListQuery = {}) =>
		api.get<AdminExternalAuthProviderPage>(
			withQuery("/admin/external-auth/providers", {
				limit: params.limit,
				offset: params.offset,
			}),
		),
	get: (id: AdminExternalAuthProviderPath["id"]) =>
		api.get<AdminExternalAuthProviderInfo>(
			`/admin/external-auth/providers/${id}`,
		),
	create: (data: CreateExternalAuthProviderRequest) =>
		api.post<
			OperationData<"admin_create_external_auth_provider", 201>,
			OperationRequestBody<"admin_create_external_auth_provider">
		>("/admin/external-auth/providers", data),
	update: (
		id: AdminExternalAuthProviderPath["id"],
		data: UpdateExternalAuthProviderRequest,
	) =>
		api.patch<
			AdminExternalAuthProviderInfo,
			OperationRequestBody<"admin_update_external_auth_provider">
		>(`/admin/external-auth/providers/${id}`, data),
	delete: (id: AdminExternalAuthProviderPath["id"]) =>
		api.delete<void>(`/admin/external-auth/providers/${id}`),
	testParams: (data: ExternalAuthProviderTestParamsRequest) =>
		api.post<
			ExternalAuthProviderTestResult,
			OperationRequestBody<"admin_test_external_auth_provider_params">
		>("/admin/external-auth/providers/test", data),
	test: (id: AdminExternalAuthProviderPath["id"]) =>
		api.post<ExternalAuthProviderTestResult>(
			`/admin/external-auth/providers/${id}/test`,
		),
};
