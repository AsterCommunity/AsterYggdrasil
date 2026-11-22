//! API data transfer objects.
//!
//! Route handlers should import request/response contracts from this module
//! instead of defining public API structs inline.

pub mod account;
pub mod admin;
pub mod auth;
pub mod external_auth;
pub mod profiles;
pub mod textures;
pub(crate) mod validation;

pub use account::{
    AccountAuditLogFilterQuery, AccountOverviewResp, AccountUserBanInfo, AccountUserBanListQuery,
};
pub use admin::{
    AdminMinecraftProfileListQuery, AdminTaskCleanupReq, AdminTaskListQuery, AdminUserBanListQuery,
    AdminUserListQuery, AdminYggdrasilSessionForwardServerListQuery, CreateAdminUserReq,
    CreateExternalAuthProviderReq, CreateUserBanReq, CreateUserInvitationReq,
    CreateYggdrasilSessionForwardServerReq, ExecuteConfigActionReq, ExecuteConfigActionResp,
    ExternalAuthProviderTestParamsReq, RemovedCountResponse, RevokeUserBanReq, SetConfigReq,
    UpdateAdminUserReq, UpdateExternalAuthProviderReq, UpdateUserBanReq,
    UpdateYggdrasilSessionForwardServerReq,
};
pub use auth::{
    AcceptUserInvitationReq, ActionMessageResp, ChangePasswordReq, CheckResp,
    ContactVerificationConfirmQuery, LoginReq, LogoutReq, LogoutResp, PasskeyLoginFinishReq,
    PasskeyLoginStartReq, PasskeyRegisterFinishReq, PasskeyRegisterStartReq,
    PasswordResetConfirmReq, PasswordResetRequestReq, PatchPasskeyReq, PublicCaptchaPolicyResp,
    RefreshReq, RegisterReq, RequestEmailChangeReq, ResendRegisterActivationReq, SetupReq,
    UpdateAvatarSourceReq, UpdateProfileReq,
};
pub use external_auth::{ExternalAuthCallbackQuery, StartExternalAuthReq};
pub use profiles::{
    CreateMinecraftProfileReq, CurrentMinecraftProfileListQuery, RenameMinecraftProfileReq,
};
pub use textures::{
    BindMinecraftTextureReq, CopyPublicTextureReq, CreateMinecraftTextureTagReq,
    CreateTextureReportReq, HandleTextureReportReq, ReplaceWardrobeTextureTagsReq,
    ReviewTextureLibraryTextureReq, UpdateMinecraftTextureTagReq, UpdateWardrobeTextureReq,
};
pub(crate) use validation::validate_request;
