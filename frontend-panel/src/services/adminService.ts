import { withQuery } from "@/lib/query";
import type {
	AdminAuditLogQuery,
	AdminExternalAuthProviderInfo,
	AdminExternalAuthProviderListQuery,
	AdminExternalAuthProviderPage,
	AdminMinecraftProfileInfo,
	AdminOverview,
	AdminTaskCleanupRequest,
	AdminTaskListQuery,
	AdminTaskPage,
	AdminTextureLibraryPage,
	AdminTextureLibraryQuery,
	AdminTextureLibraryTagPage,
	AdminTextureLibraryTagQuery,
	AdminTextureReportPage,
	AdminTextureReportQuery,
	AdminUserInvitationInfo,
	AdminUserInvitationPage,
	AdminUserInvitationQuery,
	AdminUserListQuery,
	AdminUserMinecraftProfileQuery,
	AdminUserPage,
	AdminYggdrasilSessionForwardServerInfo,
	AdminYggdrasilSessionForwardServerPage,
	AdminYggdrasilSessionForwardServerQuery,
	AuditLogPage,
	ConfigListQuery,
	ConfigSchemaItem,
	CreateAdminUserRequest,
	CreateExternalAuthProviderRequest,
	CreateMinecraftTextureTagRequest,
	CreateUserInvitationRequest,
	CreateYggdrasilSessionForwardServerRequest,
	ExecuteConfigActionRequest,
	ExecuteConfigActionResponse,
	ExternalAuthProviderKindInfo,
	ExternalAuthProviderTestParamsRequest,
	ExternalAuthProviderTestResult,
	HandleTextureReportRequest,
	MinecraftTextureMetadata,
	OperationData,
	OperationPath,
	OperationRequestBody,
	RemovedCountResponse,
	RenameMinecraftProfileRequest,
	ReviewTextureLibraryTextureRequest,
	SetConfigRequest,
	SetConfigResponse,
	SystemConfig,
	SystemConfigPage,
	SystemConfigValue,
	SystemInfoResponse,
	TemplateVariableGroup,
	UpdateAdminUserRequest,
	UpdateExternalAuthProviderRequest,
	UpdateMinecraftTextureTagRequest,
	UpdateYggdrasilSessionForwardServerRequest,
	YggdrasilProfilePage,
} from "@/types/api";
import { api } from "./http";

type AdminConfigPath = OperationPath<"get_config">;
type AdminExternalAuthProviderPath =
	OperationPath<"admin_get_external_auth_provider">;
type AdminRetryTaskPath = OperationPath<"admin_retry_task">;
type AdminUserPath = OperationPath<"admin_get_user">;
type AdminTextureLibraryTagPath =
	OperationPath<"admin_update_texture_library_tag">;
type AdminTextureLibraryTexturePath =
	OperationPath<"admin_get_texture_library_texture">;
type AdminTextureReportPath = OperationPath<"admin_get_texture_library_report">;
type AdminYggdrasilSessionForwardServerPath =
	OperationPath<"admin_get_yggdrasil_session_forward_server">;

export const adminAuditService = {
	list: (params: AdminAuditLogQuery = {}) =>
		api.get<AuditLogPage>(
			withQuery("/admin/audit-logs", {
				limit: params.limit,
				user_id: params.user_id,
				action: params.action,
				entity_type: params.entity_type,
				entity_id: params.entity_id,
				after: params.after,
				before: params.before,
				after_created_at: params.after_created_at,
				after_id: params.after_id,
			}),
		),
};

export const adminOverviewService = {
	get: () => api.get<AdminOverview>("/admin/overview"),
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
			values: {
				target_email: targetEmail?.trim() || "",
			},
		}),
	previewCaptcha: (values: Record<string, SystemConfigValue>) =>
		api.post<
			ExecuteConfigActionResponse,
			OperationRequestBody<"execute_config_action">
		>("/admin/config/auth_captcha/action", {
			action: "preview_captcha",
			values,
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
				kind: params.kind,
				status: params.status,
				after_updated_at: params.after_updated_at,
				after_id: params.after_id,
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

export const adminTextureLibraryService = {
	listTextures: (params: AdminTextureLibraryQuery = {}) =>
		api.get<AdminTextureLibraryPage>(
			withQuery("/admin/texture-library/textures", {
				limit: params.limit,
				after_updated_at: params.after_updated_at,
				after_id: params.after_id,
				keyword: params.keyword,
				texture_type: params.texture_type,
				visibility: params.visibility,
				library_status: params.library_status,
				published: params.published,
				tag_ids: params.tag_ids,
				tag_search_method: params.tag_search_method,
			}),
		),
	getTexture: (textureId: AdminTextureLibraryTexturePath["texture_id"]) =>
		api.get<OperationData<"admin_get_texture_library_texture">>(
			`/admin/texture-library/textures/${textureId}`,
		),
	deleteTexture: (textureId: AdminTextureLibraryTexturePath["texture_id"]) =>
		api.delete<void>(`/admin/texture-library/textures/${textureId}`),
	approveTexture: (
		textureId: AdminTextureLibraryTexturePath["texture_id"],
		data: ReviewTextureLibraryTextureRequest = {},
	) =>
		api.post<
			OperationData<"admin_approve_texture_library_texture">,
			OperationRequestBody<"admin_approve_texture_library_texture">
		>(`/admin/texture-library/textures/${textureId}/approve`, data),
	rejectTexture: (
		textureId: AdminTextureLibraryTexturePath["texture_id"],
		data: ReviewTextureLibraryTextureRequest,
	) =>
		api.post<
			OperationData<"admin_reject_texture_library_texture">,
			OperationRequestBody<"admin_reject_texture_library_texture">
		>(`/admin/texture-library/textures/${textureId}/reject`, data),
	unpublishTexture: (
		textureId: AdminTextureLibraryTexturePath["texture_id"],
		data: ReviewTextureLibraryTextureRequest = {},
	) =>
		api.post<
			OperationData<"admin_unpublish_texture_library_texture">,
			OperationRequestBody<"admin_unpublish_texture_library_texture">
		>(`/admin/texture-library/textures/${textureId}/unpublish`, data),
	listReports: (params: AdminTextureReportQuery = {}) =>
		api.get<AdminTextureReportPage>(
			withQuery("/admin/texture-library/reports", {
				limit: params.limit,
				status: params.status,
				reason: params.reason,
				texture_id: params.texture_id,
				after_created_at: params.after_created_at,
				after_id: params.after_id,
			}),
		),
	getReport: (reportId: AdminTextureReportPath["report_id"]) =>
		api.get<OperationData<"admin_get_texture_library_report">>(
			`/admin/texture-library/reports/${reportId}`,
		),
	acceptReport: (
		reportId: AdminTextureReportPath["report_id"],
		data: HandleTextureReportRequest = {},
	) =>
		api.post<
			OperationData<"admin_accept_texture_library_report">,
			OperationRequestBody<"admin_accept_texture_library_report">
		>(`/admin/texture-library/reports/${reportId}/accept`, data),
	rejectReport: (
		reportId: AdminTextureReportPath["report_id"],
		data: HandleTextureReportRequest = {},
	) =>
		api.post<
			OperationData<"admin_reject_texture_library_report">,
			OperationRequestBody<"admin_reject_texture_library_report">
		>(`/admin/texture-library/reports/${reportId}/reject`, data),
	listTags: (params: AdminTextureLibraryTagQuery = {}) =>
		api.get<AdminTextureLibraryTagPage>(
			withQuery("/admin/texture-library/tags", {
				limit: params.limit,
				offset: params.offset,
			}),
		),
	createTag: (data: CreateMinecraftTextureTagRequest) =>
		api.post<
			OperationData<"admin_create_texture_library_tag">,
			OperationRequestBody<"admin_create_texture_library_tag">
		>("/admin/texture-library/tags", data),
	updateTag: (
		tagId: AdminTextureLibraryTagPath["tag_id"],
		data: UpdateMinecraftTextureTagRequest,
	) =>
		api.patch<
			OperationData<"admin_update_texture_library_tag">,
			OperationRequestBody<"admin_update_texture_library_tag">
		>(`/admin/texture-library/tags/${tagId}`, data),
	deleteTag: (tagId: AdminTextureLibraryTagPath["tag_id"]) =>
		api.delete<void>(`/admin/texture-library/tags/${tagId}`),
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
				withQuery(`/admin/users/${userId}/minecraft-profiles`, {
					limit: params.limit,
					after_id: params.after_id,
				}),
			)
			.then((page) => page.items),
	listByUserPage: (
		userId: number,
		params: AdminUserMinecraftProfileQuery = {},
	) =>
		api.get<YggdrasilProfilePage>(
			withQuery(`/admin/users/${userId}/minecraft-profiles`, {
				limit: params.limit,
				after_id: params.after_id,
			}),
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
				keyword: params.keyword,
				role: params.role,
				status: params.status,
				after_created_at: params.after_created_at,
				after_id: params.after_id,
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
	delete: (id: AdminUserPath["id"]) => api.delete<void>(`/admin/users/${id}`),
	revokeSessions: (id: AdminUserPath["id"]) =>
		api.post<OperationData<"admin_revoke_user_sessions">>(
			`/admin/users/${id}/sessions/revoke`,
		),
	listInvitations: (params: AdminUserInvitationQuery = {}) =>
		api.get<AdminUserInvitationPage>(
			withQuery("/admin/users/invitations", {
				limit: params.limit,
				after_created_at: params.after_created_at,
				after_id: params.after_id,
			}),
		),
	createInvitation: (data: CreateUserInvitationRequest) =>
		api.post<AdminUserInvitationInfo, CreateUserInvitationRequest>(
			"/admin/users/invitations",
			data,
		),
	revokeInvitation: (id: number) =>
		api.post<AdminUserInvitationInfo>(`/admin/users/invitations/${id}/revoke`),
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

export const adminYggdrasilSessionForwardService = {
	list: (params: AdminYggdrasilSessionForwardServerQuery = {}) =>
		api.get<AdminYggdrasilSessionForwardServerPage>(
			withQuery("/admin/yggdrasil/session-forward-servers", {
				limit: params.limit,
				offset: params.offset,
				sort_by: params.sort_by,
			}),
		),
	get: (id: AdminYggdrasilSessionForwardServerPath["id"]) =>
		api.get<AdminYggdrasilSessionForwardServerInfo>(
			`/admin/yggdrasil/session-forward-servers/${id}`,
		),
	create: (data: CreateYggdrasilSessionForwardServerRequest) =>
		api.post<
			OperationData<"admin_create_yggdrasil_session_forward_server", 201>,
			OperationRequestBody<"admin_create_yggdrasil_session_forward_server">
		>("/admin/yggdrasil/session-forward-servers", data),
	update: (
		id: AdminYggdrasilSessionForwardServerPath["id"],
		data: UpdateYggdrasilSessionForwardServerRequest,
	) =>
		api.patch<
			AdminYggdrasilSessionForwardServerInfo,
			OperationRequestBody<"admin_update_yggdrasil_session_forward_server">
		>(`/admin/yggdrasil/session-forward-servers/${id}`, data),
	delete: (id: AdminYggdrasilSessionForwardServerPath["id"]) =>
		api.delete<void>(`/admin/yggdrasil/session-forward-servers/${id}`),
};
