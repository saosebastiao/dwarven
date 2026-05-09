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
