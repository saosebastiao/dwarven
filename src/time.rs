use chrono::{DateTime, SecondsFormat, Utc};

/// Frontmatter timestamp form: `YYYY-MM-DDTHH:MM:SSZ` (ISO 8601, UTC, second
/// precision). Per `storage-model.md#R4.3` and `R4.4`.
pub fn iso_frontmatter(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Comment-filename timestamp form: `YYYY-MM-DDTHHMMZ` (no colons, minute
/// precision). Per `storage-model.md#R4.4.2`.
pub fn iso_filename(ts: &DateTime<Utc>) -> String {
    ts.format("%Y-%m-%dT%H%MZ").to_string()
}

pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}
