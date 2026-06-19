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
pub mod yggdrasil;

pub use account::{AccountAuditLogFilterQuery, AccountOverviewResp};
pub use admin::{
    AdminMinecraftProfileListQuery, AdminTaskCleanupReq, AdminTaskListQuery, AdminUserListQuery,
    AdminYggdrasilSessionForwardServerListQuery, CreateAdminUserReq, CreateExternalAuthProviderReq,
    CreateUserInvitationReq, CreateYggdrasilSessionForwardServerReq, ExecuteConfigActionReq,
    ExecuteConfigActionResp, ExternalAuthProviderTestParamsReq, RemovedCountResponse, SetConfigReq,
    UpdateAdminUserReq, UpdateExternalAuthProviderReq, UpdateYggdrasilSessionForwardServerReq,
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
