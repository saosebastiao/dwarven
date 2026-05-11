//! UTC ISO-8601 timestamp helpers.
//!
//! Two forms, both UTC-anchored:
//! - [`iso_frontmatter`]: `YYYY-MM-DDTHH:MM:SSZ` for frontmatter fields.
//! - [`iso_filename`]: `YYYY-MM-DDTHHMMZ` for comment filenames (no
//!   colons; filesystem-safe across platforms).
//!
//! Per `storage-model.md#R4.3` and `R4.4`.

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 9, 10, 30, 7).unwrap()
    }

    #[test]
    fn frontmatter_has_seconds_and_z() {
        assert_eq!(iso_frontmatter(&fixed()), "2026-05-09T10:30:07Z");
    }

    #[test]
    fn filename_has_no_colons_minute_precision() {
        assert_eq!(iso_filename(&fixed()), "2026-05-09T1030Z");
    }
}
