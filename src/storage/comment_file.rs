use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use super::atomic::write_atomic;

/// Comment frontmatter per `storage-model.md#R4.4`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub fn read_comment(path: &Path) -> Result<CommentFile> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let stripped = raw
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("{} is not a frontmatter document", path.display()))?;
    let end = stripped
        .find("\n---\n")
        .ok_or_else(|| anyhow!("{} missing frontmatter terminator", path.display()))?;
    let yaml = &stripped[..end + 1];
    let body = &stripped[end + "\n---\n".len()..];
    let frontmatter: CommentFrontmatter = serde_yaml::from_str(yaml)
        .with_context(|| format!("parsing comment frontmatter in {}", path.display()))?;
    Ok(CommentFile {
        frontmatter,
        body: body.to_string(),
    })
}

/// Compute the next available comment sequence number by scanning filenames
/// for the leading numeric prefix. Returns 1 for an empty (or absent)
/// directory. Caller must hold the repo lock to make seq allocation atomic
/// across concurrent writers.
pub fn next_comment_seq(comments_dir: &Path) -> Result<u32> {
    if !comments_dir.exists() {
        return Ok(1);
    }
    let mut max = 0_u32;
    for entry in fs::read_dir(comments_dir)
        .with_context(|| format!("reading {}", comments_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(prefix) = name.split('-').next() {
            if let Ok(seq) = prefix.parse::<u32>() {
                if seq > max {
                    max = seq;
                }
            }
        }
    }
    Ok(max + 1)
}

/// Read all comments under a `comments/` directory, sorted by `seq` ascending.
pub fn list_comments(comments_dir: &Path) -> Result<Vec<CommentFile>> {
    let mut comments = Vec::new();
    if !comments_dir.exists() {
        return Ok(comments);
    }
    for entry in fs::read_dir(comments_dir)
        .with_context(|| format!("reading {}", comments_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".md") || name.starts_with('.') {
            continue;
        }
        comments.push(read_comment(&entry.path())?);
    }
    comments.sort_by_key(|c| c.frontmatter.seq);
    Ok(comments)
}
