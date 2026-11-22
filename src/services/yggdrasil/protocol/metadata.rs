use crate::config::{RuntimeConfig, auth_runtime, site_url, yggdrasil::RuntimeYggdrasilPolicy};
use crate::runtime::RuntimeConfigRuntimeState;
use crate::services::yggdrasil::signature;
use aster_yggdrasil_protocol::{YggdrasilMeta, YggdrasilMetaLinks, YggdrasilMetaResp};

pub fn metadata<S: RuntimeConfigRuntimeState>(state: &S) -> YggdrasilMetaResp {
    let runtime_config = state.runtime_config();
    let policy = RuntimeYggdrasilPolicy::from_runtime_config(runtime_config);
    let signature_publickey = signature::signature_public_key(&policy).unwrap_or_else(|error| {
        tracing::warn!(
            error = %error,
            "failed to resolve yggdrasil signature public key; using configured public key fallback"
        );
        policy.signature_public_key.clone()
    });
    YggdrasilMetaResp {
        meta: YggdrasilMeta {
            server_name: policy.server_name,
            implementation_name: "AsterYggdrasil".to_string(),
            implementation_version: env!("CARGO_PKG_VERSION").to_string(),
            links: meta_links(runtime_config),
            feature_non_email_login: policy.allow_profile_name_login,
            feature_enable_profile_key: policy.enable_profile_key,
            feature_enable_mojang_anti_features: policy.enable_mojang_anti_features,
            // TODO(profile-name-policy): drive this from the effective profile
            // name rule when custom profile name validation is supported.
            feature_username_check: true,
        },
        skin_domains: policy.skin_domains,
        signature_publickey,
    }
}

fn meta_links(runtime_config: &RuntimeConfig) -> Option<YggdrasilMetaLinks> {
    let public_site_url = site_url::public_site_url(runtime_config)?;
    let auth_policy = auth_runtime::RuntimeAuthPolicy::from_runtime_config(runtime_config);

    Some(YggdrasilMetaLinks {
        homepage: site_url::join_origin_and_path(&public_site_url, "/"),
        register: auth_policy
            .allow_user_registration
            .then(|| site_url::join_origin_and_path(&public_site_url, "/register")),
    })
}
