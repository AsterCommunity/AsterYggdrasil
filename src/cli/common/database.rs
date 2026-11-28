//! CLI shared database helpers: backend naming, migration history checks,
//! identifier quoting, scalar reads, URL redaction, and offline-safe
//! connections.
//!
//! Mirrors the AsterDrive CLI contract so operational commands behave the
//! same way across the org's products.

use crate::errors::{AsterError, MapAsterErr, Result};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use std::path::Path;
use url::Url;

/// The SeaORM migration history table; every product's SeaORM install uses it.
pub(in crate::cli) const MIGRATION_TABLE: &str = "seaql_migrations";

/// Human-facing backend name, including the org's `postgres` alias for
/// PostgreSQL.
pub(in crate::cli) fn backend_name(backend: DbBackend) -> &'static str {
    match backend {
        DbBackend::MySql => "mysql",
        DbBackend::Postgres => "postgres",
        DbBackend::Sqlite => "sqlite",
        _ => "unknown",
    }
}

/// Reads the pending migration versions for a connected database. The
/// migration history must be a prefix of the current code's migration list
/// with no unknown versions; anything else is a validation error.
///
/// The history table is read with plain SELECTs: sea-orm's own
/// `get_applied_migrations` creates the history table as a side effect,
/// which would write to read-only operational connections (dry-run previews
/// against a fresh target must never mutate it). A missing history table
/// means a fresh database, so every known migration is pending.
pub(in crate::cli) async fn pending_migrations(
    db: &sea_orm::DatabaseConnection,
) -> Result<Vec<String>> {
    use aster_yggdrasil_migration::MigratorTrait as _;
    let known: Vec<String> = aster_yggdrasil_migration::Migrator::migrations()
        .iter()
        .map(|migration| migration.name().to_string())
        .collect();
    let backend = db.get_database_backend();
    let history_exists = match backend {
        DbBackend::Sqlite => format!(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = {}",
            quote_literal(MIGRATION_TABLE)
        ),
        // Schema must be scoped: shared test container instances host several
        // databases, and an unqualified lookup matches other databases'
        // history tables.
        DbBackend::Postgres => format!(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_name = {}",
            quote_literal(MIGRATION_TABLE)
        ),
        DbBackend::MySql => format!(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = DATABASE() AND table_name = {}",
            quote_literal(MIGRATION_TABLE)
        ),
        other => {
            return Err(AsterError::validation_error(format!(
                "unsupported backend: {}",
                backend_name(other)
            )));
        }
    };
    let exists = scalar_i64(db, backend, &history_exists).await?;
    let applied: Vec<String> = if exists == 0 {
        Vec::new()
    } else {
        let rows = db
            .query_all_raw(Statement::from_string(
                backend,
                format!(
                    "SELECT {} FROM {}",
                    quote_ident(backend, "version"),
                    quote_ident(backend, MIGRATION_TABLE)
                ),
            ))
            .await
            .map_aster_err(AsterError::database_operation)?;
        let mut versions = Vec::with_capacity(rows.len());
        for row in rows {
            let version: String = row
                .try_get_by_index(0)
                .map_aster_err(AsterError::database_operation)?;
            versions.push(version);
        }
        versions
    };
    validate_migration_history(&applied, &known)
}

/// Validation for migration history: the applied list must form a prefix of
/// the known migration order and must not contain unknown versions.
fn validate_migration_history(applied: &[String], known: &[String]) -> Result<Vec<String>> {
    if applied.len() > known.len() {
        return Err(AsterError::validation_error(
            "database contains unknown migration versions beyond the current set",
        ));
    }
    for (index, version) in applied.iter().enumerate() {
        if known[index] != version.as_str() {
            if known.contains(version) {
                return Err(AsterError::validation_error(
                    "database has a non-prefix migration history; \
                     applied versions diverge from the current migration order",
                ));
            }
            return Err(AsterError::validation_error(format!(
                "database contains unknown migration versions: {}",
                version
            )));
        }
    }
    Ok(known[applied.len()..].to_vec())
}

/// Reads a single integer cell, accepting i64, i32 and bool(1/0) shapes.
pub(in crate::cli) async fn scalar_i64<C>(db: &C, backend: DbBackend, sql: &str) -> Result<i64>
where
    C: ConnectionTrait,
{
    let row = db
        .query_one_raw(Statement::from_string(backend, sql))
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::database_operation(format!("query returned no rows: {sql}")))?;
    if let Ok(value) = row.try_get_by_index::<i64>(0) {
        return Ok(value);
    }
    if let Ok(value) = row.try_get_by_index::<i32>(0) {
        return Ok(i64::from(value));
    }
    if let Ok(value) = row.try_get_by_index::<bool>(0) {
        return Ok(i64::from(value));
    }
    Err(AsterError::database_operation(format!(
        "query returned a non-integer value: {sql}"
    )))
}

/// Quotes an identifier for the target backend (backticks for MySQL, double
/// quotes elsewhere; the quote character itself is doubled).
pub(in crate::cli) fn quote_ident(backend: DbBackend, ident: &str) -> String {
    if backend == DbBackend::MySql {
        format!("`{}`", ident.replace('`', "``"))
    } else {
        format!("\"{}\"", ident.replace('"', "\"\""))
    }
}

/// Quotes a string literal (single quotes doubled).
pub(in crate::cli) fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Connects to a database for operational use. Offline safety for SQLite
/// sources: read-only access forces `mode=ro`, which maps to sqlx read-only
/// connections, so the source file and its journal mode are never touched,
/// not even for SELECTs. Operational connections are always a single
/// connection with statement logging off.
pub(in crate::cli) async fn connect_existing_database(
    url: &str,
    access: DatabaseAccess,
) -> Result<sea_orm::DatabaseConnection> {
    // Read-write access keeps the caller's URL verbatim so `mode=rwc` can
    // create a fresh target; read-only access forces `mode=ro` and requires
    // the file to already exist (previews never bootstrap a deployment).
    let rewritten = match (access, url.starts_with("sqlite://")) {
        (DatabaseAccess::ReadOnly, true) => force_sqlite_mode(url, "ro")?,
        _ => url.to_string(),
    };
    if access == DatabaseAccess::ReadOnly
        && let Some(file) = sqlite_file_path(&rewritten)
        && !file.exists()
    {
        return Err(AsterError::config_error(format!(
            "SQLite database does not exist: {}",
            file.display()
        )));
    }
    let mut options = sea_orm::ConnectOptions::new(rewritten);
    options.max_connections(1);
    options.sqlx_logging(false);
    sea_orm::Database::connect(options)
        .await
        .map_err(|error| AsterError::database_connection(error.to_string()))
}

/// SQLite access mode forced onto the connection URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cli) enum DatabaseAccess {
    ReadOnly,
    ReadWrite,
}

/// Rewrites the `mode` query parameter of a SQLite URL, leaving `:memory:`
/// URLs untouched.
fn force_sqlite_mode(url: &str, mode: &str) -> Result<String> {
    let body = url
        .strip_prefix("sqlite://")
        .ok_or_else(|| AsterError::config_error("not a sqlite URL"))?;
    if body == ":memory:" || body == "file::memory:" || sqlite_query_is_memory(query_of(body)) {
        return Ok(url.to_string());
    }
    let (path_and_query, fragment) = body
        .split_once('#')
        .map_or((body, None), |(rest, fragment)| (rest, Some(fragment)));
    let (path, query) = path_and_query
        .split_once('?')
        .map_or((path_and_query, None), |(rest, query)| (rest, Some(query)));
    let mut rewritten_query = Vec::new();
    let mut had_mode = false;
    if let Some(query) = query {
        for pair in query.split('&') {
            if pair
                .split_once('=')
                .is_some_and(|(key, _)| key.eq_ignore_ascii_case("mode"))
            {
                had_mode = true;
                rewritten_query.push(format!("mode={mode}"));
            } else {
                rewritten_query.push(pair.to_string());
            }
        }
    }
    if !had_mode {
        rewritten_query.push(format!("mode={mode}"));
    }
    let mut rewritten = format!("sqlite://{path}?{}", rewritten_query.join("&"));
    if let Some(fragment) = fragment {
        rewritten.push('#');
        rewritten.push_str(fragment);
    }
    Ok(rewritten)
}

/// Extracts the on-disk file path from a SQLite URL, if any.
fn sqlite_file_path(url: &str) -> Option<std::path::PathBuf> {
    let body = url.strip_prefix("sqlite://")?;
    if body == ":memory:" || body == "file::memory:" || sqlite_query_is_memory(query_of(body)) {
        return None;
    }
    let path = body.split('?').next().unwrap_or(body);
    let path = path.strip_prefix("file:").unwrap_or(path);
    Some(Path::new(path).to_path_buf())
}

/// The query string of a sqlite URL body, if any.
fn query_of(body: &str) -> Option<&str> {
    let without_fragment = body.split('#').next().unwrap_or(body);
    without_fragment.split('?').nth(1)
}

/// True only when the parsed query carries `mode=memory` as a complete
/// parameter value (substring matching would misread file names).
fn sqlite_query_is_memory(query: Option<&str>) -> bool {
    query.is_some_and(|query| {
        query.split('&').any(|pair| {
            pair.split_once('=')
                .is_some_and(|(key, value)| key.eq_ignore_ascii_case("mode") && value == "memory")
        })
    })
}

/// Redacts credentials and sensitive query parameters from a database URL for
/// reports and checkpoint rows.
pub(in crate::cli) fn redact_database_url(url: &str) -> String {
    if url == "sqlite::memory:" {
        return url.to_string();
    }
    if let Some(body) = url.strip_prefix("sqlite:") {
        return redact_sqlite_url(body);
    }
    match Url::parse(url) {
        Ok(mut parsed) => {
            if !parsed.username().is_empty() {
                let _ = parsed.set_username("***");
            }
            let _ = parsed.set_password(None);
            redact_query_pairs(parsed.as_str())
        }
        Err(_) => redact_unparsable_url(url),
    }
}

/// Redacts a `sqlite:` URL: keep only the file name, redact query parameters.
fn redact_sqlite_url(body: &str) -> String {
    let (body, _) = body
        .split_once('#')
        .map_or((body, None), |(rest, fragment)| (rest, Some(fragment)));
    let (path, query) = body
        .split_once('?')
        .map_or((body, None), |(rest, query)| (rest, Some(query)));
    let file_display = match path.rsplit('/').next() {
        Some("") | None => "***".to_string(),
        Some(file) => format!(".../{file}"),
    };
    let mut redacted = format!("sqlite:{file_display}");
    if let Some(query) = query {
        let pairs: Vec<String> = query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => {
                    if query_key_is_sensitive(key) {
                        format!("{key}=***")
                    } else {
                        format!("{key}={value}")
                    }
                }
                None => pair.to_string(),
            })
            .collect();
        redacted.push('?');
        redacted.push_str(&pairs.join("&"));
    }
    redacted
}

/// Fallback redaction for URLs that do not parse: mask the userinfo and
/// redact query parameters manually.
fn redact_unparsable_url(url: &str) -> String {
    let (body, _) = url
        .split_once('#')
        .map_or((url, None), |(rest, fragment)| (rest, Some(fragment)));
    let redacted = match body.split_once("://") {
        Some((scheme, rest)) => match rest.rsplit_once('@') {
            Some((_, host)) => format!("{scheme}://***@{host}"),
            None => body.to_string(),
        },
        None => body.to_string(),
    };
    redact_query_pairs(&redacted)
}

/// Redacts sensitive key/value pairs in the query part of an already
/// scheme-prefixed URL string.
fn redact_query_pairs(url: &str) -> String {
    match url.split_once('?') {
        Some((prefix, query)) => {
            let pairs: Vec<String> = query
                .split('&')
                .map(|pair| match pair.split_once('=') {
                    Some((key, value)) => {
                        if query_key_is_sensitive(key) {
                            format!("{key}=***")
                        } else {
                            format!("{key}={value}")
                        }
                    }
                    None => pair.to_string(),
                })
                .collect();
            format!("{prefix}?{}", pairs.join("&"))
        }
        None => url.to_string(),
    }
}

/// Whether a query parameter key carries a credential-like value. The key is
/// percent-decoded first so encoded names (`%70assword`) still match.
fn query_key_is_sensitive(key: &str) -> bool {
    let decoded = percent_decode_lower_ascii(key);
    let normalized: String = decoded
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    matches!(
        normalized.as_str(),
        "pass"
            | "password"
            | "pwd"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "secret"
            | "apikey"
            | "key"
            | "authorization"
            | "accesskeyid"
            | "credential"
            | "credentials"
    ) || normalized.contains("token")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("credential")
}

/// Decodes percent-escapes into lowercase ASCII characters; other characters
/// pass through lowercase where possible. Percent-decoding failures keep the
/// original bytes (the sensitivity classes are ASCII words, so undecodable
/// input cannot hide a match).
fn percent_decode_lower_ascii(key: &str) -> String {
    let bytes = key.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            decoded.push(u8::try_from((high * 16) + low).unwrap_or(0));
            index += 3;
        } else {
            decoded.push(bytes[index].to_ascii_lowercase());
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_names_use_org_aliases() {
        assert_eq!(backend_name(DbBackend::Postgres), "postgres");
        assert_eq!(backend_name(DbBackend::MySql), "mysql");
        assert_eq!(backend_name(DbBackend::Sqlite), "sqlite");
    }

    #[test]
    fn migration_history_must_be_a_known_prefix() {
        use aster_yggdrasil_migration::MigratorTrait as _;
        let known: Vec<String> = aster_yggdrasil_migration::Migrator::migrations()
            .iter()
            .map(|migration| migration.name().to_string())
            .collect();
        assert!(validate_migration_history(&known, &known).is_ok());
        assert!(validate_migration_history(&[], &known).is_ok());
        let unknown = vec![known[0].clone(), "99999999_000000_unknown".to_string()];
        assert!(validate_migration_history(&unknown, &known).is_err());
        let diverged = vec![known[known.len() - 1].clone()];
        assert!(validate_migration_history(&diverged, &known).is_err());
    }

    #[test]
    fn redaction_masks_userinfo_and_sensitive_query_parameters() {
        assert_eq!(
            redact_database_url("postgres://user:secret@db.example.com:5432/ygg"),
            "postgres://***@db.example.com:5432/ygg"
        );
        assert_eq!(
            redact_database_url(
                "mysql://root:hunter2@127.0.0.1/ygg?sslmode=disabled&password=leak"
            ),
            "mysql://***@127.0.0.1/ygg?sslmode=disabled&password=***"
        );
        assert_eq!(
            redact_database_url("sqlite://data/asteryggdrasil.db?mode=rwc"),
            "sqlite:.../asteryggdrasil.db?mode=rwc"
        );
        assert_eq!(redact_database_url("sqlite::memory:"), "sqlite::memory:");
    }

    #[test]
    fn sqlite_mode_is_forced_onto_existing_parameters() {
        let ro = force_sqlite_mode("sqlite://data/db.sqlite?mode=rwc", "ro").unwrap();
        assert!(ro.contains("mode=ro"));
        assert!(!ro.contains("rwc"));
        let rw = force_sqlite_mode("sqlite://data/db.sqlite", "rw").unwrap();
        assert!(rw.ends_with("mode=rw"));
        let memory = force_sqlite_mode("sqlite://:memory:", "ro").unwrap();
        assert_eq!(memory, "sqlite://:memory:");
    }
}
