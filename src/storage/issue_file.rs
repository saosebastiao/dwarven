use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use super::atomic::write_atomic;

/// Issue frontmatter per `storage-model.md#R4.3`.
///
/// Field order in the struct determines emit order in the YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueFrontmatter {
    pub id: u64,
    pub title: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    pub created: String,
    pub created_by: String,
    pub updated: String,
}

pub struct IssueFile {
    pub frontmatter: IssueFrontmatter,
    pub body: String,
}

pub fn write_issue(path: &Path, file: &IssueFile) -> Result<()> {
    let yaml = serde_yaml::to_string(&file.frontmatter)
        .context("serializing issue frontmatter")?;
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

pub fn read_issue(path: &Path) -> Result<IssueFile> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let (yaml, body) = split_frontmatter(&raw)
        .ok_or_else(|| anyhow!("{} is not a frontmatter document", path.display()))?;
    let frontmatter: IssueFrontmatter = serde_yaml::from_str(yaml)
        .with_context(|| format!("parsing frontmatter of {}", path.display()))?;
    Ok(IssueFile {
        frontmatter,
        body: body.to_string(),
    })
}

fn split_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let stripped = raw.strip_prefix("---\n")?;
    let end = stripped.find("\n---\n")?;
    let yaml = &stripped[..end + 1];
    let rest = &stripped[end + "\n---\n".len()..];
    Some((yaml, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fm() -> IssueFrontmatter {
        IssueFrontmatter {
            id: 7,
            title: "Hello".into(),
            issue_type: "feature".into(),
            state: "pm".into(),
            priority: None,
            blocked_by: vec![],
            blocks: vec![],
            epic: None,
            created: "2026-05-09T10:30:00Z".into(),
            created_by: "maintainer".into(),
            updated: "2026-05-09T10:30:00Z".into(),
        }
    }

    #[test]
    fn empty_optionals_are_omitted_from_yaml() {
        let yaml = serde_yaml::to_string(&fm()).unwrap();
        assert!(!yaml.contains("priority"));
        assert!(!yaml.contains("blocked_by"));
        assert!(!yaml.contains("blocks"));
        assert!(!yaml.contains("epic"));
        // Required fields are present.
        assert!(yaml.contains("id: 7"));
        assert!(yaml.contains("type: feature"));
    }

    #[test]
    fn populated_optionals_serialize() {
        let mut f = fm();
        f.priority = Some("p1".into());
        f.blocked_by = vec![3, 4];
        f.blocks = vec![9];
        f.epic = Some("cli-foundation".into());
        let yaml = serde_yaml::to_string(&f).unwrap();
        assert!(yaml.contains("priority: p1"));
        assert!(yaml.contains("blocked_by:"));
        assert!(yaml.contains("- 3"));
        assert!(yaml.contains("- 4"));
        assert!(yaml.contains("blocks:"));
        assert!(yaml.contains("- 9"));
        assert!(yaml.contains("epic: cli-foundation"));
    }

    #[test]
    fn write_then_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("issue.md");
        let original = IssueFile {
            frontmatter: {
                let mut f = fm();
                f.blocks = vec![1, 2];
                f.priority = Some("p0".into());
                f
            },
            body: "Multi\nline\nbody.\n".into(),
        };
        write_issue(&path, &original).unwrap();
        let read = read_issue(&path).unwrap();
        assert_eq!(read.frontmatter.id, original.frontmatter.id);
        assert_eq!(read.frontmatter.title, original.frontmatter.title);
        assert_eq!(read.frontmatter.priority, original.frontmatter.priority);
        assert_eq!(read.frontmatter.blocks, original.frontmatter.blocks);
        assert_eq!(read.body, original.body);
    }

    #[test]
    fn read_rejects_non_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("issue.md");
        std::fs::write(&path, "not a frontmatter doc").unwrap();
        assert!(read_issue(&path).is_err());
    }
}
