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
