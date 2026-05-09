use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use super::atomic::write_atomic;

/// Comment frontmatter per `storage-model.md#R4.4`.
#[derive(Debug, Clone, Serialize)]
pub struct CommentFrontmatter {
    pub seq: u32,
    pub issue: u64,
    pub author: String,
    pub kind: String,
    pub created: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
}

pub struct CommentFile {
    pub frontmatter: CommentFrontmatter,
    pub body: String,
}

/// Comment filename: `<seq:03>-<iso-filename>-<author>.md` per R4.4.2.
pub fn comment_filename(seq: u32, iso_filename: &str, author: &str) -> String {
    format!("{seq:03}-{iso_filename}-{author}.md")
}

pub fn comment_path(comments_dir: &Path, seq: u32, iso_filename: &str, author: &str) -> PathBuf {
    comments_dir.join(comment_filename(seq, iso_filename, author))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_pads_seq_to_three_digits() {
        assert_eq!(
            comment_filename(1, "2026-05-09T1030Z", "maintainer"),
            "001-2026-05-09T1030Z-maintainer.md"
        );
        assert_eq!(
            comment_filename(42, "2026-12-31T2359Z", "spec"),
            "042-2026-12-31T2359Z-spec.md"
        );
        assert_eq!(
            comment_filename(1234, "2026-05-09T1030Z", "x"),
            "1234-2026-05-09T1030Z-x.md"
        );
    }

    #[test]
    fn creation_comment_emits_from_to_only() {
        let fm = CommentFrontmatter {
            seq: 1,
            issue: 1,
            author: "maintainer".into(),
            kind: "state-change".into(),
            created: "2026-05-09T10:30:00Z".into(),
            from: Some("created".into()),
            to: Some("pm".into()),
            blocker: None,
        };
        let yaml = serde_yaml::to_string(&fm).unwrap();
        assert!(yaml.contains("from: created"));
        assert!(yaml.contains("to: pm"));
        assert!(!yaml.contains("blocker"));
    }
}

pub fn write_comment(path: &Path, file: &CommentFile) -> Result<()> {
    let yaml = serde_yaml::to_string(&file.frontmatter)
        .context("serializing comment frontmatter")?;
    let mut out = String::with_capacity(yaml.len() + file.body.len() + 16);
    out.push_str("---\n");
    out.push_str(&yaml);
    out.push_str("---\n");
    if !file.body.is_empty() {
        out.push_str(&file.body);
        if !file.body.ends_with('\n') {
            out.push('\n');
        }
    }
    write_atomic(path, out.as_bytes())
}
