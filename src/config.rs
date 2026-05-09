use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use toml_edit::{DocumentMut, Item, Value, value};

use crate::issue::create::{NotFoundError, UserError};
use crate::storage::config::{RepoPaths, require_initialized};
use crate::storage::write_atomic;

pub struct GetArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub key: String,
}

pub struct SetArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub key: String,
    pub value: String,
}

pub fn run_get(args: GetArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;

    let raw = fs::read_to_string(paths.config_path())?;
    let doc: DocumentMut = raw.parse().context("parsing config.toml")?;

    let parts = parse_key(&args.key)?;
    let item = navigate(doc.as_item(), &parts).ok_or_else(|| {
        NotFoundError(format!(
            "config key '{}' not found in {}",
            args.key,
            paths.config_path().display()
        ))
    })?;

    let rendered = render_item(item);
    if args.json {
        println!("{{\"key\":\"{}\",\"value\":{}}}", args.key, json_quote(&rendered));
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("{rendered}");
    }
    Ok(())
}

pub fn run_set(args: SetArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;

    let parts = parse_key(&args.key)?;
    let raw = fs::read_to_string(paths.config_path())?;
    let mut doc: DocumentMut = raw.parse().context("parsing config.toml")?;

    let new_value = parse_value(&args.value);
    set_at(doc.as_item_mut(), &parts, new_value)?;

    write_atomic(&paths.config_path(), doc.to_string().as_bytes())?;

    if args.json {
        println!("{{\"key\":\"{}\",\"ok\":true}}", args.key);
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Set {} = {}", args.key, args.value);
        // Daemon-restart notice (R6.15.2). Daemon not yet implemented in v1
        // so this is informational rather than load-bearing.
        eprintln!(
            "note: changes to {} take effect on next `dwarven serve`",
            args.key
        );
    }
    Ok(())
}

fn parse_key(key: &str) -> Result<Vec<String>> {
    if key.is_empty() {
        return Err(UserError("config key must not be empty".into()).into());
    }
    let parts: Vec<String> = key.split('.').map(|s| s.to_string()).collect();
    if parts.iter().any(|p| p.is_empty()) {
        return Err(UserError(format!(
            "invalid dotted key '{key}'; segments must not be empty"
        ))
        .into());
    }
    Ok(parts)
}

fn navigate<'a>(item: &'a Item, parts: &[String]) -> Option<&'a Item> {
    let mut cur = item;
    for p in parts {
        let table = cur.as_table()?;
        cur = table.get(p)?;
    }
    Some(cur)
}

fn set_at(item: &mut Item, parts: &[String], v: Value) -> Result<()> {
    let mut cur = item;
    for (i, p) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            let table = cur.as_table_mut().ok_or_else(|| {
                UserError(format!(
                    "cannot set '{}' at non-table parent",
                    parts.join(".")
                ))
            })?;
            table.insert(p, value(v));
            return Ok(());
        }
        let table = cur.as_table_mut().ok_or_else(|| {
            UserError(format!(
                "cannot descend into '{}' at non-table parent",
                parts[..=i].join(".")
            ))
        })?;
        if !table.contains_key(p) {
            table.insert(p, Item::Table(toml_edit::Table::new()));
        }
        cur = table.get_mut(p).expect("just inserted");
    }
    unreachable!("parts is non-empty after parse_key");
}

fn parse_value(s: &str) -> Value {
    if let Ok(i) = s.parse::<i64>() {
        return Value::from(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::from(f);
    }
    match s {
        "true" => return Value::from(true),
        "false" => return Value::from(false),
        _ => {}
    }
    Value::from(s)
}

fn render_item(item: &Item) -> String {
    match item {
        Item::Value(v) => render_value(v),
        Item::Table(_) | Item::ArrayOfTables(_) => {
            // For non-leaf get, dump the TOML subtree as-is.
            item.to_string()
        }
        Item::None => String::new(),
    }
}

fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.value().to_string(),
        Value::Integer(i) => i.value().to_string(),
        Value::Float(f) => f.value().to_string(),
        Value::Boolean(b) => b.value().to_string(),
        Value::Datetime(d) => d.value().to_string(),
        Value::Array(_) | Value::InlineTable(_) => v.to_string().trim().to_string(),
    }
}

fn json_quote(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".to_string(),
            '"' => "\\\"".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}
