//! Shared CLI helpers.

#[cfg(feature = "operator-cli")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared foundation for upcoming operator commands (#70)"
    )
)]
pub(in crate::cli) mod context;
pub(in crate::cli) mod error;
pub(in crate::cli) mod help;
#[cfg(feature = "operator-cli")]
pub(in crate::cli) mod input;
#[cfg(feature = "operator-cli")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "shared foundation for upcoming operator commands (#70)"
    )
)]
pub(in crate::cli) mod output;
