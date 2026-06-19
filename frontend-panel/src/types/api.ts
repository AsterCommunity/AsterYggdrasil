// Re-export generated types for convenience
// IMPORTANT: Agent code should import from this file instead of api.generated.ts to avoid coupling to the codegen tool and to allow manual additions of types as needed.
// It is strictly prohibited to directly add new fields in this document.
import type {
	operations as ApiOperations,
	components,
} from "@/types/api.generated";

export type { operations, paths } from "@/types/api.generated";

type OperationJsonContent<
	Operation extends keyof ApiOperations,
	Status extends
		keyof OperationResponses<Operation> = 200 extends keyof OperationResponses<Operation>
		? 200
		: keyof OperationResponses<Operation>,
> = OperationResponses<Operation>[Status] extends {
	content: {
		"application/json": infer Body;
	};
}
	? NonNullable<Body>
	: never;

type OperationResponses<Operation extends keyof ApiOperations> =
	ApiOperations[Operation] extends { responses: infer Responses }
		? Responses
		: never;

export type OperationJsonResponse<
	Operation extends keyof ApiOperations,
	Status extends
		keyof OperationResponses<Operation> = 200 extends keyof OperationResponses<Operation>
		? 200
		: keyof OperationResponses<Operation>,
> = OperationJsonContent<Operation, Status>;

export type OperationData<
	Operation extends keyof ApiOperations,
	Status extends
		keyof OperationResponses<Operation> = 200 extends keyof OperationResponses<Operation>
		? 200
		: keyof OperationResponses<Operation>,
> =
	OperationJsonResponse<Operation, Status> extends { data?: infer Data }
		? NonNullable<Data>
		: never;

export type OperationQuery<Operation extends keyof ApiOperations> =
	ApiOperations[Operation] extends { parameters: { query?: infer Query } }
		? NonNullable<Query>
		: never;

export type OperationPath<Operation extends keyof ApiOperations> =
	ApiOperations[Operation] extends { parameters: { path: infer Path } }
		? NonNullable<Path>
		: never;

export type OperationRequestBody<Operation extends keyof ApiOperations> =
	ApiOperations[Operation] extends {
		requestBody: {
			content: {
				"application/json": infer Body;
			};
		};
	}
		? NonNullable<Body>
		: never;

export type AsterErrorCode = components["schemas"]["AsterErrorCode"];
export type ApiErrorInfo = components["schemas"]["ApiErrorInfo"];
export type AvatarInfo = components["schemas"]["AvatarInfo"];
export type AvatarSource = components["schemas"]["AvatarSource"];

export type ApiResponse<T = unknown> = {
	code: AsterErrorCode;
	msg: string;
	data?: T | null;
	error?: ApiErrorInfo | null;
};

export type PublicBranding = components["schemas"]["PublicBranding"];
export type PublicCaptchaConfig = components["schemas"]["PublicCaptchaConfig"];
export type PublicYggdrasilConfig =
	components["schemas"]["PublicYggdrasilConfig"];
export type PublicTextureLibraryConfig =
	components["schemas"]["PublicTextureLibraryConfig"];
export type PublicFrontendConfig =
	components["schemas"]["PublicFrontendConfig"];

export type AdminAuditLogQuery = OperationQuery<"list_audit_logs">;
export type AccountAuditLogQuery = OperationQuery<"list_account_audit_logs">;
export type AccountAuditLogPage = OperationData<"list_account_audit_logs">;
export type AccountOverview = OperationData<"get_account_overview">;
export type AuditAction = components["schemas"]["AuditAction"];
export type AuditEntityType = components["schemas"]["AuditEntityType"];
export type AuditLogEntry = components["schemas"]["AuditLogEntry"];
export type AuditLogPage = OperationData<"list_audit_logs">;
export type AuditPresentation = components["schemas"]["AuditPresentation"];
export type AuditPresentationMessage =
	components["schemas"]["AuditPresentationMessage"];
export type AuthTokenResponse = components["schemas"]["AuthTokenResponse"];
export type ChangePasswordRequest = components["schemas"]["ChangePasswordReq"];
export type RegisterResponse = components["schemas"]["RegisterResponse"];
export type AuthUserInfo = components["schemas"]["AuthUserInfo"];
export type AdminUserInfo = components["schemas"]["AdminUserInfo"];
export type AdminUserListQuery = OperationQuery<"admin_list_users">;
export type AdminUserPage = OperationData<"admin_list_users">;
export type AdminUserInvitationInfo =
	components["schemas"]["AdminUserInvitationInfo"];
export type AdminUserInvitationPage =
	OperationData<"admin_list_user_invitations">;
export type AdminUserInvitationQuery =
	OperationQuery<"admin_list_user_invitations">;
export type AdminOverview = OperationData<"get_admin_overview">;
export type AdminOverviewServiceStatus =
	components["schemas"]["AdminOverviewServiceStatus"];
export type AdminOverviewServiceStatusKind =
	components["schemas"]["AdminOverviewServiceStatusKind"];
export type AdminOverviewSummary =
	components["schemas"]["AdminOverviewSummary"];
export type AdminOverviewSystemHealthComponent =
	components["schemas"]["AdminOverviewSystemHealthComponent"];
export type AdminOverviewSystemHealthStatus =
	components["schemas"]["AdminOverviewSystemHealthStatus"];
export type AdminOverviewSystemHealthSummary =
	components["schemas"]["AdminOverviewSystemHealthSummary"];
export type AdminOverviewTrendPoint =
	components["schemas"]["AdminOverviewTrendPoint"];
export type UserInvitationStatus =
	components["schemas"]["UserInvitationStatus"];
export type OperatorScope = components["schemas"]["OperatorScope"];
export type PublicUserInvitationInfo =
	components["schemas"]["PublicUserInvitationInfo"];
export type AcceptUserInvitationRequest =
	OperationRequestBody<"accept_user_invitation">;
export type CreateUserInvitationRequest =
	OperationRequestBody<"admin_create_user_invitation">;
export type AdminMinecraftProfileInfo =
	components["schemas"]["MinecraftProfileInfo"];
export type AdminMinecraftProfilePage =
	OperationData<"admin_list_minecraft_profiles">;
export type CreateAdminUserRequest =
	components["schemas"]["CreateAdminUserReq"];
export type UpdateAdminUserRequest =
	components["schemas"]["UpdateAdminUserReq"];
export type AdminExternalAuthProviderInfo =
	components["schemas"]["AdminExternalAuthProviderInfo"];
export type AdminExternalAuthProviderPage =
	components["schemas"]["OffsetPage_AdminExternalAuthProviderInfo"];
export type AdminExternalAuthProviderListQuery =
	OperationQuery<"admin_list_external_auth_providers">;
export type AdminYggdrasilSessionForwardServerInfo =
	components["schemas"]["AdminYggdrasilSessionForwardServerInfo"];
export type AdminYggdrasilSessionForwardServerPage =
	OperationData<"admin_list_yggdrasil_session_forward_servers">;
export type AdminYggdrasilSessionForwardServerQuery =
	OperationQuery<"admin_list_yggdrasil_session_forward_servers">;
export type CreateYggdrasilSessionForwardServerRequest =
	OperationRequestBody<"admin_create_yggdrasil_session_forward_server">;
export type UpdateYggdrasilSessionForwardServerRequest =
	OperationRequestBody<"admin_update_yggdrasil_session_forward_server">;
export type YggdrasilSessionForwardProviderKind =
	components["schemas"]["YggdrasilSessionForwardProviderKind"];
export type YggdrasilSessionForwardServerSortBy =
	components["schemas"]["YggdrasilSessionForwardServerSortBy"];
export type AdminTaskCleanupRequest =
	components["schemas"]["AdminTaskCleanupReq"];
export type AdminTaskListQuery = OperationQuery<"admin_list_tasks">;
export type AdminTaskPage = OperationData<"admin_list_tasks">;
export type BackgroundTaskKind = components["schemas"]["BackgroundTaskKind"];
export type BackgroundTaskStatus =
	components["schemas"]["BackgroundTaskStatus"];
export type CheckResp = components["schemas"]["CheckResp"];
export type CaptchaChallengeResponse =
	components["schemas"]["CaptchaChallengeResponse"];
export type PublicCaptchaPolicyResponse =
	components["schemas"]["PublicCaptchaPolicyResp"];
export type ConfigSchemaItem = components["schemas"]["ConfigSchemaItem"];
export type ConfigActionType = components["schemas"]["ConfigActionType"];
export type ConfigListQuery = OperationQuery<"list_config">;
export type CreateExternalAuthProviderRequest =
	components["schemas"]["CreateExternalAuthProviderReq"];
export type ExternalAuthProviderKindInfo =
	components["schemas"]["ExternalAuthProviderKindInfo"];
export type ExternalAuthProviderOptions =
	components["schemas"]["ExternalAuthProviderOptions"];
export type ExternalAuthProviderTestParamsRequest =
	components["schemas"]["ExternalAuthProviderTestParamsReq"];
export type ExternalAuthProviderTestResult =
	components["schemas"]["ExternalAuthProviderTestResult"];
export type ExternalAuthKind =
	components["schemas"]["ExternalAuthProviderKind"];
export type ExternalAuthPublicProvider =
	components["schemas"]["ExternalAuthPublicProvider"];
export type ExternalAuthPublicProviderQuery =
	OperationQuery<"auth_external_auth_list_providers">;
export type ExternalAuthPublicProviderPage =
	OperationData<"auth_external_auth_list_providers">;
export type ExternalAuthPublicProviderInfo =
	ExternalAuthPublicProviderPage["items"][number];
export type ExternalAuthPublicProviderByKindQuery =
	OperationQuery<"auth_external_auth_list_providers_by_kind">;
export type ExternalAuthPublicProviderByKindPage =
	OperationData<"auth_external_auth_list_providers_by_kind">;
export type ExternalAuthLinkQuery =
	OperationQuery<"auth_external_auth_list_links">;
export type ExternalAuthLinkPage =
	OperationData<"auth_external_auth_list_links">;
export type ExternalAuthLinkInfo = ExternalAuthLinkPage["items"][number];
export type ExternalAuthStartLoginRequest =
	components["schemas"]["StartExternalAuthReq"];
export type ExternalAuthStartLoginResponse =
	components["schemas"]["ExternalAuthStartLoginResponse"];
export type ExternalAuthEmailVerificationStartRequest =
	components["schemas"]["ExternalAuthEmailVerificationStartRequest"];
export type ExternalAuthEmailVerificationStartResponse =
	OperationData<"auth_external_auth_start_email_verification">;
export type ExternalAuthFinishLoginResponse =
	components["schemas"]["ExternalAuthFinishLoginResponse"];
export type ExternalAuthPasswordLinkRequest =
	components["schemas"]["ExternalAuthPasswordLinkRequest"];
export type HealthResponse = components["schemas"]["HealthResponse"];
export type SystemInfoResponse = components["schemas"]["SystemInfoResponse"];
export type LoginRequest = components["schemas"]["LoginReq"];
export type LogoutRequest = components["schemas"]["LogoutReq"];
export type LogoutResponse = components["schemas"]["LogoutResp"];
export type PasswordResetConfirmRequest =
	OperationRequestBody<"confirm_password_reset">;
export type PasswordResetRequest =
	OperationRequestBody<"request_password_reset">;
export type MinecraftTextureModel =
	components["schemas"]["MinecraftTextureModel"];
export type MinecraftTextureMetadata =
	components["schemas"]["MinecraftTextureMetadata"];
export type MinecraftTextureLibraryStatus =
	components["schemas"]["MinecraftTextureLibraryStatus"];
export type MinecraftTextureTagInfo =
	components["schemas"]["MinecraftTextureTagInfo"];
export type MinecraftTextureType =
	components["schemas"]["MinecraftTextureType"];
export type MinecraftTextureVisibility =
	components["schemas"]["MinecraftTextureVisibility"];
export type TextureTagSearchMethod =
	components["schemas"]["TextureTagSearchMethod"];
export type DateTimeIdCursor = components["schemas"]["DateTimeIdCursor"];
export type IdCursor = components["schemas"]["IdCursor"];
export type MinecraftWardrobeTextureMetadata =
	components["schemas"]["MinecraftWardrobeTextureMetadata"];
export type MinecraftWardrobeTextureQuery =
	OperationQuery<"list_current_user_wardrobe_textures">;
export type MinecraftWardrobeTexturePage =
	OperationData<"list_current_user_wardrobe_textures">;
export type MinecraftTextureTagList =
	OperationData<"list_current_user_texture_library_tags">["items"];
export type MinecraftTextureTagPage =
	OperationData<"list_current_user_texture_library_tags">;
export type MinecraftTextureTagQuery =
	OperationQuery<"list_current_user_texture_library_tags">;
export type PublicTextureLibraryTagPage =
	OperationData<"list_public_texture_library_tags">;
export type PublicTextureLibraryTagQuery =
	OperationQuery<"list_public_texture_library_tags">;
export type AdminTextureLibraryTagPage =
	OperationData<"admin_list_texture_library_tags">;
export type AdminTextureLibraryTagQuery =
	OperationQuery<"admin_list_texture_library_tags">;
export type AdminTextureLibraryPage =
	OperationData<"admin_list_texture_library_textures">;
export type AdminTextureLibraryQuery =
	OperationQuery<"admin_list_texture_library_textures">;
export type ReviewTextureLibraryTextureRequest =
	OperationRequestBody<"admin_approve_texture_library_texture">;
export type MinecraftTextureReportReason =
	components["schemas"]["MinecraftTextureReportReason"];
export type MinecraftTextureReportStatus =
	components["schemas"]["MinecraftTextureReportStatus"];
export type TextureReportInfo = components["schemas"]["TextureReportInfo"];
export type AdminTextureReportPage =
	OperationData<"admin_list_texture_library_reports">;
export type AdminTextureReportQuery =
	OperationQuery<"admin_list_texture_library_reports">;
export type CreateTextureReportRequest =
	OperationRequestBody<"create_public_texture_library_texture_report">;
export type HandleTextureReportRequest =
	OperationRequestBody<"admin_accept_texture_library_report">;
export type CreateMinecraftTextureTagRequest =
	OperationRequestBody<"admin_create_texture_library_tag">;
export type UpdateMinecraftTextureTagRequest =
	OperationRequestBody<"admin_update_texture_library_tag">;
export type ReplaceWardrobeTextureTagsRequest =
	OperationRequestBody<"replace_current_user_wardrobe_texture_tags">;
export type PublicTextureLibraryQuery =
	OperationQuery<"list_public_texture_library_textures">;
export type PublicTextureLibraryPage =
	OperationData<"list_public_texture_library_textures">;
export type PublicTextureLibraryTextureMetadata =
	OperationData<"get_public_texture_library_texture">;
export type CopyPublicTextureRequest =
	OperationRequestBody<"copy_public_texture_library_texture_to_wardrobe">;
export type UpdateWardrobeTextureRequest =
	OperationRequestBody<"update_current_user_wardrobe_texture">;
export type CreateMinecraftProfileRequest =
	components["schemas"]["CreateMinecraftProfileReq"];
export type RenameMinecraftProfileRequest =
	components["schemas"]["RenameMinecraftProfileReq"];
export type RefreshRequest = components["schemas"]["RefreshReq"];
export type RegisterRequest = components["schemas"]["RegisterReq"];
export type RemovedCountResponse =
	components["schemas"]["RemovedCountResponse"];
export type SetConfigRequest = components["schemas"]["SetConfigReq"];
export type SetConfigResponse = OperationData<"set_config">;
export type ExecuteConfigActionRequest =
	components["schemas"]["ExecuteConfigActionReq"];
export type ExecuteConfigActionResponse =
	components["schemas"]["ExecuteConfigActionResp"];
export type SetupRequest = components["schemas"]["SetupReq"];
export type SortOrder = components["schemas"]["SortOrder"];
export type SystemConfig = components["schemas"]["SystemConfig"];
export type SystemConfigPage = components["schemas"]["OffsetPage_SystemConfig"];
export type SystemConfigValue = components["schemas"]["SystemConfigValue"];
export type SystemConfigVisibility =
	components["schemas"]["SystemConfigVisibility"];
export type TemplateVariableGroup =
	components["schemas"]["TemplateVariableGroup"];
export type TemplateVariableItem =
	components["schemas"]["TemplateVariableItem"];
export type TaskInfo = components["schemas"]["TaskInfo"];
export type TaskCreatorSummary = components["schemas"]["TaskCreatorSummary"];
export type TaskPresentation = components["schemas"]["TaskPresentation"];
export type TaskPresentationCode =
	components["schemas"]["TaskPresentationCode"];
export type TaskPresentationMessage =
	components["schemas"]["TaskPresentationMessage"];
export type TaskStepInfo = components["schemas"]["TaskStepInfo"];
export type TaskStepStatus = components["schemas"]["TaskStepStatus"];
export type UpdateExternalAuthProviderRequest =
	components["schemas"]["UpdateExternalAuthProviderReq"];
export type UpdateAvatarSourceRequest =
	components["schemas"]["UpdateAvatarSourceReq"];
export type RequestEmailChangeRequest =
	OperationRequestBody<"request_email_change">;
export type UpdateProfileRequest = components["schemas"]["UpdateProfileReq"];
export type UserProfileInfo = components["schemas"]["UserProfileInfo"];
export type UserRole = components["schemas"]["UserRole"];
export type UserStatus = components["schemas"]["UserStatus"];
export type DateTimeStringCursor =
	components["schemas"]["DateTimeStringCursor"];
export type AuthSessionQuery = OperationQuery<"list_auth_sessions">;
export type AuthSessionPage = OperationData<"list_auth_sessions">;
export type AuthSessionInfo = AuthSessionPage["items"][number];
export type PasskeyQuery = OperationQuery<"list_passkeys">;
export type PasskeyPage = OperationData<"list_passkeys">;
export type PasskeyInfo = PasskeyPage["items"][number];
export type PasskeyRegisterStartRequest =
	OperationRequestBody<"start_passkey_registration">;
export type PasskeyRegisterStartResponse =
	OperationData<"start_passkey_registration">;
export type PasskeyRegisterFinishRequest =
	OperationRequestBody<"finish_passkey_registration">;
export type PasskeyLoginStartRequest =
	OperationRequestBody<"start_passkey_login">;
export type PasskeyLoginStartResponse = OperationData<"start_passkey_login">;
export type PatchPasskeyRequest = OperationRequestBody<"rename_passkey">;
export type RevokeOtherAuthSessionsResponse =
	OperationData<"revoke_other_auth_sessions">;
export type YggdrasilErrorBody = components["schemas"]["YggdrasilErrorBody"];
export type YggdrasilMetadata = OperationJsonResponse<"yggdrasil_metadata">;
export type YggdrasilProfileQuery =
	OperationQuery<"list_current_user_minecraft_profiles">;
export type YggdrasilProfilePage =
	OperationData<"list_current_user_minecraft_profiles">;
export type YggdrasilProfile = YggdrasilProfilePage["items"][number];
export type AdminUserMinecraftProfileQuery =
	OperationQuery<"admin_list_user_minecraft_profiles">;
export type YggdrasilProfileProperty =
	components["schemas"]["YggdrasilProfileProperty"];
export type YggdrasilProfileByUuidQuery =
	OperationQuery<"yggdrasil_profile_by_uuid">;
