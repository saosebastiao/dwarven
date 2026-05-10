use serde::Serialize;

use crate::storage::comment_file::CommentFile;
use crate::storage::issue_file::{IssueFile, IssueFrontmatter};

#[derive(Debug, Serialize)]
pub struct Issue {
    pub id: u64,
    pub title: String,
    pub r#type: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_priority_override: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
    pub blocked_by: Vec<u64>,
    pub blocks: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    pub created: String,
    pub created_by: String,
    pub updated: String,
    pub body: String,
}

impl Issue {
    pub fn from_file(f: IssueFile) -> Self {
        let IssueFrontmatter {
            id,
            title,
            issue_type,
            state,
            priority,
            effective_priority_override,
            blocker,
            blocked_by,
            blocks,
            epic,
            created,
            created_by,
            updated,
        } = f.frontmatter;
        Self {
            id,
            title,
            r#type: issue_type,
            state,
            priority,
            effective_priority_override,
            blocker,
            blocked_by,
            blocks,
            epic,
            created,
            created_by,
            updated,
            body: f.body,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Comment {
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
    pub body: String,
}

impl Comment {
    pub fn from_file(c: CommentFile) -> Self {
        Self {
            seq: c.frontmatter.seq,
            issue: c.frontmatter.issue,
            author: c.frontmatter.author,
            kind: c.frontmatter.kind,
            created: c.frontmatter.created,
            from: c.frontmatter.from,
            to: c.frontmatter.to,
            blocker: c.frontmatter.blocker,
            body: c.body,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DaemonStatus {
    pub state: String,
    pub pid: u32,
    pub port: u16,
    pub uptime_seconds: u64,
    pub version: String,
}
