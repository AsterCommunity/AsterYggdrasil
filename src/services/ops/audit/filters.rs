use chrono::{DateTime, Utc};
use serde::Deserialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};

use aster_yggdrasil_model::types::audit::AuditEntityType;

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
/// Query-string filters for the admin audit log listing endpoint.
pub struct AuditLogFilterQuery {
    /// Only records authored by this user ID.
    pub user_id: Option<i64>,
    /// Only records of this audit action (wire string).
    pub action: Option<String>,
    /// Only records targeting this entity class.
    pub entity_type: Option<AuditEntityType>,
    /// Only records targeting this entity row ID.
    pub entity_id: Option<i64>,
    /// Inclusive lower time bound, RFC 3339.
    pub after: Option<String>,
    /// Inclusive upper time bound, RFC 3339.
    pub before: Option<String>,
    /// Cursor pagination anchor: creation timestamp of the last seen record.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Cursor pagination anchor: ID of the last seen record, breaking
    /// creation-timestamp ties.
    pub after_id: Option<i64>,
}

/// Parsed service-layer filters consumed by the audit log repository.
pub struct AuditLogFilters {
    /// Only records authored by this user ID.
    pub user_id: Option<i64>,
    /// Only records of this audit action.
    pub action: Option<String>,
    /// Only records targeting this entity class.
    pub entity_type: Option<AuditEntityType>,
    /// Only records targeting this entity row ID.
    pub entity_id: Option<i64>,
    /// Inclusive lower time bound (UTC).
    pub after: Option<DateTime<Utc>>,
    /// Inclusive upper time bound (UTC).
    pub before: Option<DateTime<Utc>>,
}

impl AuditLogFilters {
    /// Converts query filters, parsing the RFC 3339 time bounds; unparseable
    /// bounds are dropped rather than rejected.
    pub fn from_query(query: &AuditLogFilterQuery) -> Self {
        Self {
            user_id: query.user_id,
            action: query.action.clone(),
            entity_type: query.entity_type,
            entity_id: query.entity_id,
            after: query
                .after
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|datetime| datetime.with_timezone(&Utc)),
            before: query
                .before
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|datetime| datetime.with_timezone(&Utc)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuditLogFilterQuery, AuditLogFilters};
    use aster_yggdrasil_model::types::audit::AuditEntityType;

    #[test]
    fn from_query_preserves_scalar_filters_and_parses_rfc3339_bounds() {
        let filters = AuditLogFilters::from_query(&AuditLogFilterQuery {
            user_id: Some(42),
            action: Some("config_update".to_string()),
            entity_type: Some(AuditEntityType::SystemConfig),
            entity_id: Some(7),
            after: Some("2026-06-06T01:02:03+08:00".to_string()),
            before: Some("2026-06-07T01:02:03Z".to_string()),
            after_created_at: None,
            after_id: None,
        });

        assert_eq!(filters.user_id, Some(42));
        assert_eq!(filters.action.as_deref(), Some("config_update"));
        assert_eq!(filters.entity_type, Some(AuditEntityType::SystemConfig));
        assert_eq!(filters.entity_id, Some(7));
        assert_eq!(
            filters.after.unwrap().to_rfc3339(),
            "2026-06-05T17:02:03+00:00"
        );
        assert_eq!(
            filters.before.unwrap().to_rfc3339(),
            "2026-06-07T01:02:03+00:00"
        );
    }

    #[test]
    fn from_query_ignores_invalid_timestamp_bounds() {
        let filters = AuditLogFilters::from_query(&AuditLogFilterQuery {
            user_id: None,
            action: None,
            entity_type: None,
            entity_id: None,
            after: Some("not-a-date".to_string()),
            before: Some("2026-99-99T00:00:00Z".to_string()),
            after_created_at: None,
            after_id: None,
        });

        assert_eq!(filters.user_id, None);
        assert_eq!(filters.action, None);
        assert_eq!(filters.entity_type, None);
        assert_eq!(filters.entity_id, None);
        assert_eq!(filters.after, None);
        assert_eq!(filters.before, None);
    }
}
