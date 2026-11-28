//! Two-stage value conversion: source decode into a backend-neutral
//! [`CellValue`], then target encode per the target column's binding kind.

use super::super::plan::ColumnSchema;
use crate::errors::{AsterError, Result};
use sea_orm::QueryResult;
use std::collections::BTreeMap;

/// Normalized column type used to decode source cells and encode target
/// values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::cli) enum BindingKind {
    Bool,
    Int32,
    Int64,
    Float64,
    String,
    Json,
    Timestamp,
    Bytes,
}

/// Backend-neutral intermediate representation for one cell value.
#[derive(Debug, Clone)]
pub(in crate::cli) enum CellValue {
    Null,
    Bool(bool),
    Int64(i64),
    Float64(f64),
    String(String),
    Json(serde_json::Value),
    Timestamp(chrono::DateTime<chrono::FixedOffset>),
    Bytes(Vec<u8>),
}

/// Converts one decoded source row into target-shaped sea-orm values, using
/// the target column hints for encoding.
pub(in crate::cli) fn decode_row_values(
    row: &QueryResult,
    plan: &super::super::plan::TablePlan,
    target_hints: &BTreeMap<String, BindingKind>,
) -> Result<Vec<sea_orm::Value>> {
    let mut values = Vec::with_capacity(plan.columns.len());
    for (index, column) in plan.columns.iter().enumerate() {
        let source_cell = decode_source_cell(row, index, column, plan)?;
        let target_kind = target_hints
            .get(&column.name)
            .copied()
            .unwrap_or(column.binding_kind);
        values.push(cell_into_target_value(
            source_cell,
            target_kind,
            plan,
            column,
        )?);
    }
    Ok(values)
}

/// Decodes one raw cell according to the source column's binding kind, with
/// per-type fallbacks for cross-backend shape drift.
fn decode_source_cell(
    row: &QueryResult,
    index: usize,
    column: &ColumnSchema,
    plan: &super::super::plan::TablePlan,
) -> Result<CellValue> {
    let context = format!("{}.{} as '{}'", plan.name, column.name, column.raw_type);
    let invalid =
        |reason: &str| AsterError::database_operation(format!("cannot decode {context}: {reason}"));
    let cell = match column.binding_kind {
        BindingKind::Bool => {
            if let Ok(value) = row.try_get_by_index::<Option<bool>>(index) {
                value.map(CellValue::Bool).unwrap_or(CellValue::Null)
            } else {
                let value: Option<i32> = row
                    .try_get_by_index(index)
                    .map_err(|_| invalid("expected bool or integer"))?;
                value
                    .map(|raw| CellValue::Bool(raw != 0))
                    .unwrap_or(CellValue::Null)
            }
        }
        BindingKind::Int64 => {
            if let Ok(value) = row.try_get_by_index::<Option<i64>>(index) {
                value.map(CellValue::Int64).unwrap_or(CellValue::Null)
            } else if let Ok(value) = row.try_get_by_index::<Option<i32>>(index) {
                value
                    .map(|raw| CellValue::Int64(i64::from(raw)))
                    .unwrap_or(CellValue::Null)
            } else if let Ok(value) = row.try_get_by_index::<Option<u32>>(index) {
                value
                    .map(|raw| CellValue::Int64(i64::from(raw)))
                    .unwrap_or(CellValue::Null)
            } else if let Ok(value) = row.try_get_by_index::<Option<bool>>(index) {
                value
                    .map(|raw| CellValue::Int64(i64::from(raw)))
                    .unwrap_or(CellValue::Null)
            } else {
                return Err(invalid("expected integer"));
            }
        }
        BindingKind::Int32 => decode_source_cell(
            row,
            index,
            &ColumnSchema {
                binding_kind: BindingKind::Int64,
                ..column.clone()
            },
            plan,
        )?,
        BindingKind::Float64 => {
            if let Ok(value) = row.try_get_by_index::<Option<f64>>(index) {
                value.map(CellValue::Float64).unwrap_or(CellValue::Null)
            } else if let Ok(value) = row.try_get_by_index::<Option<f32>>(index) {
                value
                    .map(|raw| CellValue::Float64(f64::from(raw)))
                    .unwrap_or(CellValue::Null)
            } else if let Ok(value) = row.try_get_by_index::<Option<i64>>(index) {
                value
                    .map(|raw| CellValue::Float64(raw as f64))
                    .unwrap_or(CellValue::Null)
            } else {
                return Err(invalid("expected float"));
            }
        }
        BindingKind::String => {
            let value: Option<String> = row
                .try_get_by_index(index)
                .map_err(|_| invalid("expected text"))?;
            value.map(CellValue::String).unwrap_or(CellValue::Null)
        }
        BindingKind::Json => {
            if let Ok(value) = row.try_get_by_index::<Option<serde_json::Value>>(index) {
                value.map(CellValue::Json).unwrap_or(CellValue::Null)
            } else {
                let value: Option<String> = row
                    .try_get_by_index(index)
                    .map_err(|_| invalid("expected json or text"))?;
                match value {
                    None => CellValue::Null,
                    Some(raw) => CellValue::Json(
                        serde_json::from_str(&raw)
                            .map_err(|error| invalid(&format!("invalid json text: {error}")))?,
                    ),
                }
            }
        }
        BindingKind::Timestamp => {
            let value: Option<chrono::DateTime<chrono::FixedOffset>> = row
                .try_get_by_index(index)
                .map_err(|_| invalid("expected timestamp"))?;
            value.map(CellValue::Timestamp).unwrap_or(CellValue::Null)
        }
        BindingKind::Bytes => {
            let value: Option<Vec<u8>> = row
                .try_get_by_index(index)
                .map_err(|_| invalid("expected bytes"))?;
            value.map(CellValue::Bytes).unwrap_or(CellValue::Null)
        }
    };
    Ok(cell)
}

/// Parses the string encodings accepted for boolean targets.
fn parse_bool(raw: &str) -> Option<bool> {
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Encodes one decoded cell into the sea-orm value for the target kind.
fn cell_into_target_value(
    cell: CellValue,
    target: BindingKind,
    plan: &super::super::plan::TablePlan,
    column: &ColumnSchema,
) -> Result<sea_orm::Value> {
    let context = format!("{}.{}", plan.name, column.name);
    let unsupported = || {
        AsterError::database_operation(format!(
            "unsupported source type for {target:?} target at {context}"
        ))
    };
    let value = match (cell, target) {
        // Nulls must adopt the target column's parameter type: Postgres
        // rejects a bigint-typed NULL against a timestamptz parameter.
        (CellValue::Null, BindingKind::Bool) => sea_orm::Value::Bool(None),
        (CellValue::Null, BindingKind::Int32) => sea_orm::Value::Int(None),
        (CellValue::Null, BindingKind::Int64) => sea_orm::Value::BigInt(None),
        (CellValue::Null, BindingKind::Float64) => sea_orm::Value::Double(None),
        (CellValue::Null, BindingKind::String) => sea_orm::Value::String(None),
        (CellValue::Null, BindingKind::Json) => sea_orm::Value::Json(None),
        (CellValue::Null, BindingKind::Timestamp) => sea_orm::Value::ChronoDateTimeUtc(None),
        (CellValue::Null, BindingKind::Bytes) => sea_orm::Value::Bytes(None),
        (CellValue::Bool(raw), BindingKind::Bool) => sea_orm::Value::Bool(Some(raw)),
        (CellValue::Bool(raw), BindingKind::Int32) => sea_orm::Value::Int(Some(i32::from(raw))),
        (CellValue::Bool(raw), BindingKind::Int64) => sea_orm::Value::BigInt(Some(i64::from(raw))),
        (CellValue::Bool(raw), BindingKind::Float64) => {
            sea_orm::Value::Double(Some(f64::from(u8::from(raw))))
        }
        (CellValue::Bool(raw), BindingKind::String) => {
            sea_orm::Value::String(Some(if raw { "true" } else { "false" }.to_string()))
        }
        (CellValue::Int64(raw), BindingKind::Bool) => sea_orm::Value::Bool(Some(raw != 0)),
        (CellValue::Int64(raw), BindingKind::Int32) => {
            sea_orm::Value::Int(Some(i32::try_from(raw).map_err(|_| {
                AsterError::database_operation(format!("value {raw} overflows int32 at {context}"))
            })?))
        }
        (CellValue::Int64(raw), BindingKind::Int64) => sea_orm::Value::BigInt(Some(raw)),
        (CellValue::Int64(raw), BindingKind::Float64) => sea_orm::Value::Double(Some(raw as f64)),
        (CellValue::Int64(raw), BindingKind::String) => {
            sea_orm::Value::String(Some(raw.to_string()))
        }
        (CellValue::Float64(raw), BindingKind::Float64) => sea_orm::Value::Double(Some(raw)),
        (CellValue::Float64(raw), BindingKind::String) => {
            sea_orm::Value::String(Some(raw.to_string()))
        }
        (CellValue::String(raw), BindingKind::Bool) => {
            let parsed = parse_bool(&raw).ok_or_else(|| {
                AsterError::database_operation(format!(
                    "cannot interpret '{raw}' as bool at {context}"
                ))
            })?;
            sea_orm::Value::Bool(Some(parsed))
        }
        (CellValue::String(raw), BindingKind::Int32) => {
            let parsed: i32 = raw.parse().map_err(|_| {
                AsterError::database_operation(format!(
                    "cannot interpret '{raw}' as int32 at {context}"
                ))
            })?;
            sea_orm::Value::Int(Some(parsed))
        }
        (CellValue::String(raw), BindingKind::Int64) => {
            let parsed: i64 = raw.parse().map_err(|_| {
                AsterError::database_operation(format!(
                    "cannot interpret '{raw}' as int64 at {context}"
                ))
            })?;
            sea_orm::Value::BigInt(Some(parsed))
        }
        (CellValue::String(raw), BindingKind::Float64) => {
            let parsed: f64 = raw.parse().map_err(|_| {
                AsterError::database_operation(format!(
                    "cannot interpret '{raw}' as float at {context}"
                ))
            })?;
            sea_orm::Value::Double(Some(parsed))
        }
        (CellValue::String(raw), BindingKind::String) => sea_orm::Value::String(Some(raw)),
        (CellValue::String(raw), BindingKind::Json) => {
            let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
                AsterError::database_operation(format!("invalid json text at {context}: {error}"))
            })?;
            sea_orm::Value::Json(Some(Box::new(parsed)))
        }
        (CellValue::String(raw), BindingKind::Timestamp) => {
            let parsed: chrono::DateTime<chrono::FixedOffset> =
                chrono::DateTime::parse_from_rfc3339(&raw).map_err(|error| {
                    AsterError::database_operation(format!(
                        "cannot interpret '{raw}' as timestamp at {context}: {error}"
                    ))
                })?;
            sea_orm::Value::ChronoDateTimeUtc(Some(parsed.into()))
        }
        (CellValue::Json(raw), BindingKind::Json) => sea_orm::Value::Json(Some(Box::new(raw))),
        (CellValue::Json(raw), BindingKind::String) => {
            sea_orm::Value::String(Some(raw.to_string()))
        }
        (CellValue::Timestamp(raw), BindingKind::Timestamp) => {
            sea_orm::Value::ChronoDateTimeUtc(Some(raw.into()))
        }
        (CellValue::Timestamp(raw), BindingKind::String) => {
            sea_orm::Value::String(Some(raw.to_rfc3339()))
        }
        (CellValue::Bytes(raw), BindingKind::Bytes) => sea_orm::Value::Bytes(Some(raw)),
        (CellValue::Bytes(_), BindingKind::String) => {
            return Err(AsterError::database_operation(format!(
                "cannot losslessly convert bytes into string at {context}"
            )));
        }
        _ => return Err(unsupported()),
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::database_migration::plan::ColumnSchema;

    fn column(kind: BindingKind) -> ColumnSchema {
        ColumnSchema {
            name: "value".to_string(),
            raw_type: "test".to_string(),
            pk_order: 0,
            binding_kind: kind,
        }
    }

    fn plan_with(kind: BindingKind) -> super::super::super::plan::TablePlan {
        super::super::super::plan::TablePlan {
            name: "t".to_string(),
            columns: vec![column(kind)],
            primary_key: Vec::new(),
            source_rows: 0,
            sequence_reset: false,
        }
    }

    fn string_cell(raw: &str) -> sea_orm::Value {
        cell_into_target_value(
            CellValue::String(raw.to_string()),
            BindingKind::Bool,
            &plan_with(BindingKind::Bool),
            &column(BindingKind::Bool),
        )
        .expect("conversion should succeed")
    }

    #[test]
    fn string_bool_cells_convert_into_bool_values() {
        for (raw, expected) in [
            ("true", true),
            ("1", true),
            ("yes", true),
            ("false", false),
            ("0", false),
        ] {
            let value = string_cell(raw);
            let sea_orm::Value::Bool(Some(actual)) = value else {
                panic!("expected bool value for {raw}");
            };
            assert_eq!(actual, expected, "raw: {raw}");
        }
    }

    #[test]
    fn string_timestamp_cells_convert_into_timestamp_values() {
        let value = cell_into_target_value(
            CellValue::String("2026-11-28T12:00:00+08:00".to_string()),
            BindingKind::Timestamp,
            &plan_with(BindingKind::Timestamp),
            &column(BindingKind::Timestamp),
        )
        .expect("conversion should succeed");
        assert!(matches!(value, sea_orm::Value::ChronoDateTimeUtc(Some(_))));
    }

    #[test]
    fn bytes_cannot_convert_into_string_values() {
        let error = cell_into_target_value(
            CellValue::Bytes(vec![1, 2, 3]),
            BindingKind::String,
            &plan_with(BindingKind::Bytes),
            &column(BindingKind::Bytes),
        )
        .expect_err("bytes to string must be rejected");
        assert!(error.to_string().contains("cannot losslessly convert"));
    }
}
