//! `GET` / `PATCH /api/v1/config` handlers.
//!
//! `GET` returns the validated config as JSON. `PATCH` accepts a
//! partial update; missing keys are unchanged. Restart-required keys
//! (`daemon.port`) are accepted into the file but the running daemon
//! continues on the old value until restart — see
//! [`docs/configuration.md`](../../../docs/configuration.md) for the
//! flag table.

use std::fs;

use axum::Json;
use axum::extract::State;
use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item};

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::storage::write_atomic;

/// Keys whose mutation requires a daemon restart per `coordination-hub.md#R10.4`.
const RESTART_KEYS: &[&str] = &["daemon.port", "daemon.bind"];

pub async fn get_config(
    State(app): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let paths = app.paths.clone();
    let value = tokio::task::spawn_blocking(move || -> Result<Value, ApiError> {
        let raw = fs::read_to_string(paths.config_path())
            .map_err(|e| ApiError::hub_error(format!("reading config.toml: {e}")))?;
        let parsed: toml::Value = raw
            .parse()
            .map_err(|e| ApiError::hub_error(format!("parsing config.toml: {e}")))?;
        Ok(toml_to_json(parsed))
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;

    Ok(Json(value))
}

pub async fn patch_config(
    State(app): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let paths = app.paths.clone();
    let updates = flatten_dotted(&body);
    if updates.is_empty() {
        return Err(ApiError::bad_request(
            "PATCH body must be an object with at least one key",
        ));
    }

    let updates_clone = updates.clone();
    tokio::task::spawn_blocking(move || -> Result<(), ApiError> {
        let raw = fs::read_to_string(paths.config_path())
            .map_err(|e| ApiError::hub_error(format!("reading config.toml: {e}")))?;
        let mut doc: DocumentMut = raw
            .parse()
            .map_err(|e| ApiError::hub_error(format!("parsing config.toml: {e}")))?;
        for (key, val) in &updates_clone {
            apply_update(&mut doc, key, val).map_err(ApiError::bad_request)?;
        }
        write_atomic(&paths.config_path(), doc.to_string().as_bytes())
            .map_err(|e| ApiError::hub_error(format!("writing config.toml: {e}")))?;
        Ok(())
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;

    let requires_restart = updates
        .iter()
        .any(|(k, _)| RESTART_KEYS.iter().any(|r| r == k));

    let paths = app.paths.clone();
    let updated = tokio::task::spawn_blocking(move || -> Result<Value, ApiError> {
        let raw = fs::read_to_string(paths.config_path())
            .map_err(|e| ApiError::hub_error(format!("re-reading config.toml: {e}")))?;
        let parsed: toml::Value = raw
            .parse()
            .map_err(|e| ApiError::hub_error(format!("re-parsing config.toml: {e}")))?;
        Ok(toml_to_json(parsed))
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;

    Ok(Json(serde_json::json!({
        "config": updated,
        "requires_restart": requires_restart,
    })))
}

/// Walk the JSON object, returning a list of (dotted_key, leaf_value).
/// E.g., `{"daemon": {"port": 7777}}` → `[("daemon.port", 7777)]`.
fn flatten_dotted(value: &Value) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    if let Value::Object(map) = value {
        for (k, v) in map {
            walk(k, v, &mut out);
        }
    }
    out
}

fn walk(prefix: &str, value: &Value, out: &mut Vec<(String, Value)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = format!("{prefix}.{k}");
                walk(&key, v, out);
            }
        }
        _ => out.push((prefix.to_string(), value.clone())),
    }
}

fn apply_update(doc: &mut DocumentMut, dotted: &str, val: &Value) -> Result<(), String> {
    let parts: Vec<&str> = dotted.split('.').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return Err(format!("invalid key '{dotted}': segments must not be empty"));
    }
    let toml_value = json_to_toml_value(val).ok_or_else(|| {
        format!("unsupported value type for key '{dotted}' (arrays of mixed types not supported)")
    })?;

    let mut cur: &mut Item = doc.as_item_mut();
    for (i, p) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            let table = cur.as_table_mut().ok_or_else(|| {
                format!("cannot set '{dotted}': parent is not a table")
            })?;
            table.insert(p, toml_edit::value(toml_value));
            return Ok(());
        }
        let table = cur.as_table_mut().ok_or_else(|| {
            format!("cannot descend into '{}': not a table", parts[..=i].join("."))
        })?;
        if !table.contains_key(p) {
            table.insert(p, Item::Table(toml_edit::Table::new()));
        }
        cur = table.get_mut(p).expect("just inserted");
    }
    unreachable!("parts is non-empty");
}

fn json_to_toml_value(v: &Value) -> Option<toml_edit::Value> {
    match v {
        Value::Null => None,
        Value::Bool(b) => Some(toml_edit::Value::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(toml_edit::Value::from(i))
            } else if let Some(f) = n.as_f64() {
                Some(toml_edit::Value::from(f))
            } else {
                None
            }
        }
        Value::String(s) => Some(toml_edit::Value::from(s.clone())),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut m = Map::new();
            for (k, v) in t {
                m.insert(k, toml_to_json(v));
            }
            Value::Object(m)
        }
    }
}
