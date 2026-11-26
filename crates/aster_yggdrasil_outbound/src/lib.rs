//! AsterYggdrasil outbound HTTP policy.
//!
//! Single construction point for every outbound HTTP client the product
//! makes, mirroring `aster_drive_outbound`: a purpose [`Profile`] selects
//! timeout/proxy defaults and participates in telemetry, forge keeps
//! redirects and automatic retries disabled, and callers may only narrow the
//! policy through [`ClientConfig`]. Provider protocols and credentials stay
//! with callers.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::todo
    )
)]

use aster_forge_http::{HttpClient, HttpObservation, HttpObserver};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Canonical product identity, shared by all Yggdrasil workspace packages at
/// the same version. Keep this in the outbound policy crate so transport does
/// not depend on the main application.
pub const OUTBOUND_HTTP_USER_AGENT: &str = concat!("AsterYggdrasil/", env!("CARGO_PKG_VERSION"));

/// Purpose is mandatory and participates in telemetry; profiles never permit redirects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Querying external Yggdrasil session servers (hasJoined forwarding and
    /// upstream profile reads). Callers attach their per-upstream total
    /// timeout per request; the profile only bounds unattended stalls.
    SessionForward,
}

impl Profile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SessionForward => "session_forward",
        }
    }
}

pub struct ClientConfig {
    profile: Profile,
    user_agent: String,
    connect_timeout: Duration,
    read_timeout: Option<Duration>,
    timeout: Option<Duration>,
    proxy: ProxyPolicy,
    roots: Vec<reqwest::Certificate>,
    pinned: Option<(String, Vec<SocketAddr>)>,
    observer: Option<Arc<dyn HttpObserver>>,
}

enum ProxyPolicy {
    System,
    Disabled,
    Explicit(Box<reqwest::Proxy>),
}

impl ClientConfig {
    /// Select a purpose and inherit the standard AsterYggdrasil product identity.
    pub fn new(profile: Profile) -> Self {
        let (connect_timeout, read_timeout, timeout) = match profile {
            Profile::SessionForward => (5, Some(30), None),
        };
        Self {
            profile,
            user_agent: OUTBOUND_HTTP_USER_AGENT.to_owned(),
            connect_timeout: Duration::from_secs(connect_timeout),
            read_timeout: read_timeout.map(Duration::from_secs),
            timeout: timeout.map(Duration::from_secs),
            proxy: ProxyPolicy::System,
            roots: Vec::new(),
            pinned: None,
            observer: None,
        }
    }

    /// Override product identity for this client without changing its transport profile.
    /// Invalid header values fail at build time, before any request is sent.
    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.user_agent = value.into();
        self
    }

    pub fn connect_timeout(mut self, value: Duration) -> Self {
        self.connect_timeout = value;
        self
    }

    pub fn read_timeout(mut self, value: Duration) -> Self {
        self.read_timeout = Some(value);
        self
    }

    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    pub fn without_read_timeout(mut self) -> Self {
        self.read_timeout = None;
        self
    }

    pub fn without_total_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    pub fn no_proxy(mut self) -> Self {
        self.proxy = ProxyPolicy::Disabled;
        self
    }

    pub fn proxy(mut self, proxy: reqwest::Proxy) -> Self {
        self.proxy = ProxyPolicy::Explicit(Box::new(proxy));
        self
    }

    pub fn add_root_certificate(mut self, certificate: reqwest::Certificate) -> Self {
        self.roots.push(certificate);
        self
    }

    pub fn resolve_to_addrs(mut self, host: &str, addrs: &[SocketAddr]) -> Self {
        self.pinned = Some((host.to_owned(), addrs.to_vec()));
        self
    }

    pub fn observer(mut self, observer: Arc<dyn HttpObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn build(self) -> Result<HttpClient, reqwest::Error> {
        let mut builder = HttpClient::builder()
            .user_agent(self.user_agent)
            .connect_timeout(self.connect_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(8);
        if let Some(value) = self.read_timeout {
            builder = builder.read_timeout(value);
        }
        if let Some(value) = self.timeout {
            builder = builder.timeout(value);
        }
        // DNS pinning is authoritative even if proxy() was called after resolve_to_addrs().
        // Forge enforces the same rule; express the product policy here as well.
        let proxy = if self.pinned.is_some() {
            ProxyPolicy::Disabled
        } else {
            self.proxy
        };
        builder = match proxy {
            ProxyPolicy::System => builder,
            ProxyPolicy::Disabled => builder.no_proxy(),
            ProxyPolicy::Explicit(proxy) => builder.proxy(*proxy),
        };
        for certificate in self.roots {
            builder = builder.add_root_certificate(certificate);
        }
        if let Some((host, addrs)) = self.pinned {
            builder = builder.resolve_to_addrs(&host, &addrs);
        }
        builder
            .observer(Arc::new(Observer {
                profile: self.profile,
                sink: self.observer,
            }))
            .build()
    }
}

struct Observer {
    profile: Profile,
    sink: Option<Arc<dyn HttpObserver>>,
}

impl HttpObserver for Observer {
    fn observe(&self, event: HttpObservation) {
        tracing::debug!(target: "aster_yggdrasil_outbound", profile = self.profile.as_str(), method = %event.method,
            status = event.status, outcome = ?event.outcome, duration_seconds = event.duration.as_secs_f64(),
            "outbound HTTP exchange completed");
        if let Some(sink) = &self.sink {
            sink.observe(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientConfig, OUTBOUND_HTTP_USER_AGENT, Profile};

    #[test]
    fn session_forward_profile_sets_stall_bounds() {
        let client = ClientConfig::new(Profile::SessionForward)
            .build()
            .expect("session forward client should build");
        let _ = client;
        assert_eq!(Profile::SessionForward.as_str(), "session_forward");
        assert!(OUTBOUND_HTTP_USER_AGENT.starts_with("AsterYggdrasil/"));
    }
}
