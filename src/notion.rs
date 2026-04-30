use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::{json, Value};

const NOTION_VERSION: &str = "2022-06-28";
const BASE_URL: &str = "https://api.notion.com/v1";
const MAX_BLOCKS_PER_REQ: usize = 100;
const MAX_TEXT_LEN: usize = 2000;

// ── Frontmatter ─────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub date: Option<String>,
    pub status: Option<String>,
    pub tags: Vec<String>,
    pub notion_id: Option<String>,
}

/// Split content into (frontmatter, body). Returns defaults if no frontmatter block.
pub fn parse_note(content: &str) -> (Frontmatter, String) {
    let mut fm = Frontmatter::default();

    let after_marker = if let Some(s) = content.strip_prefix("---\r\n") {
        s
    } else if let Some(s) = content.strip_prefix("---\n") {
        s
    } else {
        return (fm, content.to_string());
    };

    let (yaml_len, skip) = if let Some(p) = after_marker.find("\n---\r\n") {
        (p, 6)
    } else if let Some(p) = after_marker.find("\n---\n") {
        (p, 5)
    } else {
        return (fm, content.to_string());
    };

    let yaml = &after_marker[..yaml_len];
    let body = after_marker[yaml_len + skip..].to_string();

    for line in yaml.lines() {
        if let Some(v) = line.strip_prefix("title:") {
            fm.title = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(v) = line.strip_prefix("date:") {
            fm.date = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("status:") {
            fm.status = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("tags:") {
            let v = v.trim();
            if v.starts_with('[') && v.ends_with(']') {
                fm.tags = v[1..v.len() - 1]
                    .split(',')
                    .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
            }
        } else if let Some(v) = line.strip_prefix("notion_id:") {
            fm.notion_id = Some(v.trim().to_string());
        }
    }

    (fm, body)
}

pub fn serialize_frontmatter(fm: &Frontmatter) -> String {
    let mut s = String::from("---\n");
    if let Some(t) = &fm.title {
        s.push_str(&format!("title: \"{}\"\n", t.replace('"', "\\\"")));
    }
    if let Some(d) = &fm.date {
        s.push_str(&format!("date: {}\n", d));
    }
    if let Some(st) = &fm.status {
        s.push_str(&format!("status: {}\n", st));
    }
    let tags = if fm.tags.is_empty() {
        "[]".to_string()
    } else {
        let inner = fm
            .tags
            .iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", inner)
    };
    s.push_str(&format!("tags: {}\n", tags));
    if let Some(id) = &fm.notion_id {
        s.push_str(&format!("notion_id: {}\n", id));
    }
    s.push_str("---\n");
    s
}

// ── Notion HTTP client ───────────────────────────────────────────────────────

pub struct NotionClient {
    client: Client,
    token: String,
}

pub struct DatabaseInfo {
    pub title_prop: String,
    pub status_prop: Option<String>,
    pub tags_prop: Option<String>,
}

impl NotionClient {
    pub fn new(token: &str) -> Self {
        Self {
            client: Client::new(),
            token: token.to_string(),
        }
    }

    fn get(&self, path: &str) -> Result<Value> {
        let resp = self
            .client
            .get(format!("{}{}", BASE_URL, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", NOTION_VERSION)
            .send()
            .context("Failed to connect to Notion API")?;

        let status = resp.status();
        let body: Value = resp.json().unwrap_or_default();
        check_status(status, &body)
    }

    fn post(&self, path: &str, payload: &Value) -> Result<Value> {
        let resp = self
            .client
            .post(format!("{}{}", BASE_URL, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", NOTION_VERSION)
            .json(payload)
            .send()
            .context("Failed to connect to Notion API")?;

        let status = resp.status();
        let body: Value = resp.json().unwrap_or_default();
        check_status(status, &body)
    }

    fn patch(&self, path: &str, payload: &Value) -> Result<Value> {
        let resp = self
            .client
            .patch(format!("{}{}", BASE_URL, path))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Notion-Version", NOTION_VERSION)
            .json(payload)
            .send()
            .context("Failed to connect to Notion API")?;

        let status = resp.status();
        let body: Value = resp.json().unwrap_or_default();
        check_status(status, &body)
    }

    pub fn check_auth(&self) -> Result<String> {
        let body = self.get("/users/me")?;
        let name = body["name"]
            .as_str()
            .unwrap_or("integration")
            .to_string();
        Ok(name)
    }

    pub fn inspect_database(&self, db_id: &str) -> Result<DatabaseInfo> {
        let body = self
            .get(&format!("/databases/{}", db_id))
            .with_context(|| {
                format!(
                    "Cannot access database '{}'. Make sure it's shared with your integration.",
                    db_id
                )
            })?;

        let properties = body["properties"]
            .as_object()
            .context("Database has no properties")?;

        let mut title_prop = "Name".to_string();
        let mut status_prop = None;
        let mut tags_prop = None;

        for (name, prop) in properties {
            match prop["type"].as_str().unwrap_or("") {
                "title" => title_prop = name.clone(),
                "select" if name.to_lowercase() == "status" => {
                    status_prop = Some(name.clone());
                }
                "multi_select" if name.to_lowercase() == "tags" => {
                    tags_prop = Some(name.clone());
                }
                _ => {}
            }
        }

        Ok(DatabaseInfo {
            title_prop,
            status_prop,
            tags_prop,
        })
    }

    pub fn create_page(
        &self,
        db_id: &str,
        db_info: &DatabaseInfo,
        title: &str,
        status: Option<&str>,
        tags: &[String],
        blocks: Vec<Value>,
    ) -> Result<String> {
        let mut props = serde_json::Map::new();
        props.insert(
            db_info.title_prop.clone(),
            json!({ "title": [{ "text": { "content": truncate(title, MAX_TEXT_LEN) } }] }),
        );
        if let (Some(prop), Some(s)) = (&db_info.status_prop, status)
            && !s.is_empty()
        {
            props.insert(prop.clone(), json!({ "select": { "name": s } }));
        }
        if let Some(prop) = &db_info.tags_prop
            && !tags.is_empty()
        {
            let tag_vals: Vec<Value> = tags.iter().map(|t| json!({ "name": t })).collect();
            props.insert(prop.clone(), json!({ "multi_select": tag_vals }));
        }

        let first: Vec<Value> = blocks.iter().take(MAX_BLOCKS_PER_REQ).cloned().collect();
        let rest: Vec<Value> = blocks.into_iter().skip(MAX_BLOCKS_PER_REQ).collect();

        let result = self.post(
            "/pages",
            &json!({
                "parent": { "database_id": db_id },
                "properties": props,
                "children": first,
            }),
        )?;

        let page_id = result["id"]
            .as_str()
            .context("Missing page ID in Notion response")?
            .to_string();

        // Append overflow blocks in batches
        let mut offset = 0;
        while offset < rest.len() {
            let end = (offset + MAX_BLOCKS_PER_REQ).min(rest.len());
            self.patch(
                &format!("/blocks/{}/children", page_id),
                &json!({ "children": &rest[offset..end] }),
            )?;
            offset = end;
        }

        Ok(page_id)
    }
}

fn check_status(status: reqwest::StatusCode, body: &Value) -> Result<Value> {
    if status.is_success() {
        return Ok(body.clone());
    }
    let msg = body["message"].as_str().unwrap_or("unknown error");
    if status == 401 {
        bail!("Notion token is invalid. Check notion.token in your config.");
    }
    if status == 403 {
        bail!(
            "No access: {}. Share the database with your integration in Notion.",
            msg
        );
    }
    if status == 404 {
        bail!("Not found (404): {}.", msg);
    }
    bail!("Notion API error ({}): {}", status, msg);
}

// ── Markdown → Notion blocks ─────────────────────────────────────────────────

pub fn markdown_to_blocks(body: &str) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(fence_lang) = line.strip_prefix("```") {
            if in_code {
                let content = code_lines.join("\n");
                for chunk in chunk_str(&content, MAX_TEXT_LEN) {
                    blocks.push(json!({
                        "object": "block",
                        "type": "code",
                        "code": {
                            "rich_text": [{ "type": "text", "text": { "content": chunk } }],
                            "language": lang_name(&code_lang),
                        }
                    }));
                }
                in_code = false;
                code_lang.clear();
                code_lines.clear();
            } else {
                in_code = true;
                code_lang = fence_lang.trim().to_string();
            }
            continue;
        }

        if in_code {
            code_lines.push(line);
            continue;
        }

        if line.trim().is_empty() {
            continue;
        }

        let block = if let Some(t) = line.strip_prefix("#### ").or_else(|| line.strip_prefix("### ")) {
            block_with_rt("heading_3", t)
        } else if let Some(t) = line.strip_prefix("## ") {
            block_with_rt("heading_2", t)
        } else if let Some(t) = line.strip_prefix("# ") {
            block_with_rt("heading_1", t)
        } else if let Some(t) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            block_with_rt("bulleted_list_item", t)
        } else if is_numbered(line) {
            let t = line.split_once(". ").map(|x| x.1).unwrap_or(line);
            block_with_rt("numbered_list_item", t)
        } else if let Some(t) = line.strip_prefix("> ") {
            block_with_rt("quote", t)
        } else if line == "---" || line == "***" || line == "___" {
            json!({ "object": "block", "type": "divider", "divider": {} })
        } else {
            block_with_rt("paragraph", line)
        };

        blocks.push(block);
    }

    // flush an unclosed code block
    if in_code && !code_lines.is_empty() {
        let content = code_lines.join("\n");
        blocks.push(json!({
            "object": "block",
            "type": "code",
            "code": {
                "rich_text": [{ "type": "text", "text": { "content": truncate(&content, MAX_TEXT_LEN) } }],
                "language": "plain text",
            }
        }));
    }

    blocks
}

fn block_with_rt(kind: &str, text: &str) -> Value {
    json!({
        "object": "block",
        "type": kind,
        kind: { "rich_text": rich_text(text) }
    })
}

fn is_numbered(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    if !chars.next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return false;
    }
    let rest: String = chars.collect();
    rest.starts_with(". ")
        || rest
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .starts_with(". ")
}

fn lang_name(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "rust" => "rust",
        "js" | "javascript" => "javascript",
        "ts" | "typescript" => "typescript",
        "py" | "python" => "python",
        "sh" | "bash" | "shell" => "shell",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" | "c++" => "c++",
        "cs" | "csharp" => "c#",
        "rb" | "ruby" => "ruby",
        "sql" => "sql",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        _ => "plain text",
    }
}

// ── Inline rich-text parsing ─────────────────────────────────────────────────

fn rich_text(text: &str) -> Value {
    let mut segs: Vec<Value> = Vec::new();
    parse_inline(text, &mut segs);
    if segs.is_empty() {
        segs.push(json!({ "type": "text", "text": { "content": "" } }));
    }
    json!(segs)
}

fn parse_inline(text: &str, out: &mut Vec<Value>) {
    if text.is_empty() {
        return;
    }

    let bold_pos = text.find("**");
    let italic_pos = first_single_star(text);
    let code_pos = text.find('`');

    let earliest = [
        bold_pos.map(|p| (p, "**", "bold")),
        italic_pos.map(|p| (p, "*", "italic")),
        code_pos.map(|p| (p, "`", "code")),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|&(p, _, _)| p);

    match earliest {
        None => push_plain(text, out),
        Some((pos, marker, anno)) => {
            if pos > 0 {
                push_plain(&text[..pos], out);
            }
            let after = &text[pos + marker.len()..];
            if let Some(end) = closing(after, marker) {
                for chunk in chunk_str(&after[..end], MAX_TEXT_LEN) {
                    out.push(json!({
                        "type": "text",
                        "text": { "content": chunk },
                        "annotations": { anno: true }
                    }));
                }
                parse_inline(&after[end + marker.len()..], out);
            } else {
                push_plain(&text[pos..], out);
            }
        }
    }
}

/// First `*` that is not part of `**`.
fn first_single_star(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        if b[i] == b'*' {
            if b.get(i + 1) == Some(&b'*') {
                i += 2;
            } else {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Find the closing occurrence of `marker` in `text`.
/// For `*`, skips `**` sequences.
fn closing(text: &str, marker: &str) -> Option<usize> {
    let b = text.as_bytes();
    let m = marker.as_bytes();
    let mlen = m.len();
    let mut i = 0;
    while i + mlen <= text.len() {
        if &b[i..i + mlen] == m {
            if marker == "*" && b.get(i + 1) == Some(&b'*') {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += 1;
        while i < text.len() && !text.is_char_boundary(i) {
            i += 1;
        }
    }
    None
}

fn push_plain(text: &str, out: &mut Vec<Value>) {
    for chunk in chunk_str(text, MAX_TEXT_LEN) {
        out.push(json!({ "type": "text", "text": { "content": chunk } }));
    }
}

fn chunk_str(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + max).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        chunks.push(text[start..end].to_string());
        start = end;
    }
    chunks
}

fn truncate(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
