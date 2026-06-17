use crate::types::MinecraftTextureModel;

pub const STEVE_HASH: &str = "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133";
pub const ALEX_HASH: &str = "394b483392052fb28d6271c736ba0df0394223c93b6348f1f1d135fdb7412daa";

const STEVE_BYTES: &[u8] = include_bytes!("../../assets/default_skins/steve.png");
const ALEX_BYTES: &[u8] = include_bytes!("../../assets/default_skins/alex.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultSkin {
    pub hash: &'static str,
    pub bytes: &'static [u8],
    pub model: MinecraftTextureModel,
}

pub fn for_profile_uuid(uuid: &str) -> DefaultSkin {
    if default_skin_model(uuid) == MinecraftTextureModel::Slim {
        return alex();
    }
    steve()
}

pub fn by_hash(hash: &str) -> Option<DefaultSkin> {
    match hash {
        STEVE_HASH => Some(steve()),
        ALEX_HASH => Some(alex()),
        _ => None,
    }
}

const fn steve() -> DefaultSkin {
    DefaultSkin {
        hash: STEVE_HASH,
        bytes: STEVE_BYTES,
        model: MinecraftTextureModel::Default,
    }
}

const fn alex() -> DefaultSkin {
    DefaultSkin {
        hash: ALEX_HASH,
        bytes: ALEX_BYTES,
        model: MinecraftTextureModel::Slim,
    }
}

fn default_skin_model(uuid: &str) -> MinecraftTextureModel {
    let Ok(uuid) = uuid::Uuid::parse_str(uuid) else {
        return MinecraftTextureModel::Default;
    };
    if uuid.as_u128() & 1 == 1 {
        MinecraftTextureModel::Slim
    } else {
        MinecraftTextureModel::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_skin_selection_is_stable_from_profile_uuid() {
        assert_eq!(
            for_profile_uuid("00000000000000000000000000000000").model,
            MinecraftTextureModel::Default
        );
        assert_eq!(
            for_profile_uuid("00000000000000000000000000000001").model,
            MinecraftTextureModel::Slim
        );
    }

    #[test]
    fn default_skin_hashes_match_embedded_bytes() {
        assert_eq!(crate::utils::hash::sha256_hex(STEVE_BYTES), STEVE_HASH);
        assert_eq!(crate::utils::hash::sha256_hex(ALEX_BYTES), ALEX_HASH);
    }
}
