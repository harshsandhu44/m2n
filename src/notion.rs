use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::{Value, json};

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
        let name = body["name"].as_str().unwrap_or("integration").to_string();
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

    pub fn update_page(
        &self,
        page_id: &str,
        db_info: &DatabaseInfo,
        title: &str,
        status: Option<&str>,
        tags: &[String],
        blocks: Vec<Value>,
    ) -> Result<()> {
        // Update properties
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
        self.patch(
            &format!("/pages/{}", page_id),
            &json!({ "properties": props }),
        )?;

        // Clear existing content
        let child_ids = self.get_block_children(page_id)?;
        for id in child_ids {
            self.patch(&format!("/blocks/{}", id), &json!({ "archived": true }))?;
        }

        // Append new blocks in batches of 100
        let mut offset = 0;
        while offset < blocks.len() {
            let end = (offset + MAX_BLOCKS_PER_REQ).min(blocks.len());
            self.patch(
                &format!("/blocks/{}/children", page_id),
                &json!({ "children": &blocks[offset..end] }),
            )?;
            offset = end;
        }

        Ok(())
    }

    fn get_block_children(&self, block_id: &str) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let path = if let Some(ref c) = cursor {
                format!(
                    "/blocks/{}/children?page_size=100&start_cursor={}",
                    block_id, c
                )
            } else {
                format!("/blocks/{}/children?page_size=100", block_id)
            };
            let resp = self.get(&path)?;

            if let Some(results) = resp["results"].as_array() {
                for block in results {
                    if let Some(id) = block["id"].as_str() {
                        ids.push(id.to_string());
                    }
                }
            }

            if resp["has_more"].as_bool().unwrap_or(false) {
                cursor = resp["next_cursor"].as_str().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(ids)
    }
}

fn check_status(status: reqwest::StatusCode, body: &Value) -> Result<Value> {
    if status.is_success() {
        return Ok(body.clone());
    }
    let msg = body["message"].as_str().unwrap_or("unknown error");
    if status == 401 {
        bail!(
            "Authentication failed (401): token is invalid or expired.\n\
             → Get a new token at https://www.notion.so/my-integrations and update notion.token in your config."
        );
    }
    if status == 403 {
        bail!(
            "Access denied (403): {}.\n\
             → Open the database in Notion → click '...' → Connections → add your integration.",
            msg
        );
    }
    if status == 404 {
        bail!(
            "Not found (404): {}.\n\
             → Check that the database ID in your config is correct and the page hasn't been deleted.",
            msg
        );
    }
    bail!("Notion API error ({}): {}", status, msg);
}

/// Parse a Notion database ID from a full URL or raw ID string.
/// Accepts: full notion.so URLs, 32-char hex strings, UUID format.
/// Returns: UUID-formatted string or None if unparseable.
pub fn normalize_db_id(input: &str) -> Option<String> {
    let s = input.trim();

    // Strip URL down to the last path segment before any query/fragment
    let segment = if s.contains("notion.so") {
        s.split('?')
            .next()
            .and_then(|u| u.split('/').next_back())
            .unwrap_or(s)
    } else {
        s
    };

    // Collect hex characters (strip dashes and non-hex chars from slugs)
    let hex: String = segment.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    if hex.len() < 32 {
        return None;
    }

    // IDs are always the trailing 32 hex chars (after any slug prefix)
    let id = &hex[hex.len() - 32..];

    Some(format!(
        "{}-{}-{}-{}-{}",
        &id[0..8],
        &id[8..12],
        &id[12..16],
        &id[16..20],
        &id[20..32]
    ))
}

// ── Markdown → Notion blocks ─────────────────────────────────────────────────

pub fn markdown_to_blocks(body: &str) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<&str> = Vec::new();
    // Stack for nested list items: (indent_level, block_value)
    let mut list_stack: Vec<(usize, Value)> = Vec::new();

    for line in body.lines() {
        if let Some(fence_lang) = line.strip_prefix("```") {
            flush_list_stack(&mut list_stack, &mut blocks);
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
            flush_list_stack(&mut list_stack, &mut blocks);
            continue;
        }

        // Detect indent level for list nesting
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        // Check if this line is a list item
        let list_item = if let Some(t) = trimmed
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed.strip_prefix("* [ ] "))
        {
            Some(("to_do_unchecked", t))
        } else if let Some(t) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
            .or_else(|| trimmed.strip_prefix("* [x] "))
        {
            Some(("to_do_checked", t))
        } else if let Some(t) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            Some(("bulleted_list_item", t))
        } else if is_numbered(trimmed) {
            let t = trimmed.split_once(". ").map(|x| x.1).unwrap_or(trimmed);
            Some(("numbered_list_item", t))
        } else {
            None
        };

        if let Some((kind, text)) = list_item {
            let block = match kind {
                "to_do_unchecked" => json!({
                    "object": "block", "type": "to_do",
                    "to_do": { "rich_text": rich_text(text), "checked": false, "children": [] }
                }),
                "to_do_checked" => json!({
                    "object": "block", "type": "to_do",
                    "to_do": { "rich_text": rich_text(text), "checked": true, "children": [] }
                }),
                "numbered_list_item" => json!({
                    "object": "block", "type": "numbered_list_item",
                    "numbered_list_item": { "rich_text": rich_text(text), "children": [] }
                }),
                _ => json!({
                    "object": "block", "type": "bulleted_list_item",
                    "bulleted_list_item": { "rich_text": rich_text(text), "children": [] }
                }),
            };
            push_list_item(&mut list_stack, indent, block);
            continue;
        }

        // Non-list line: flush any pending nested list
        flush_list_stack(&mut list_stack, &mut blocks);

        let block = if let Some(t) = line
            .strip_prefix("#### ")
            .or_else(|| line.strip_prefix("### "))
        {
            block_with_rt("heading_3", t)
        } else if let Some(t) = line.strip_prefix("## ") {
            block_with_rt("heading_2", t)
        } else if let Some(t) = line.strip_prefix("# ") {
            block_with_rt("heading_1", t)
        } else if let Some(t) = line.strip_prefix("> ") {
            block_with_rt("quote", t)
        } else if line == "---" || line == "***" || line == "___" {
            json!({ "object": "block", "type": "divider", "divider": {} })
        } else {
            block_with_rt("paragraph", line)
        };

        blocks.push(block);
    }

    flush_list_stack(&mut list_stack, &mut blocks);

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

fn push_list_item(stack: &mut Vec<(usize, Value)>, indent: usize, block: Value) {
    const MAX_DEPTH: usize = 5;

    // Pop items at same or deeper indent to get the right parent level
    while stack.len() > 1 && stack.last().map(|(i, _)| *i >= indent).unwrap_or(false) {
        let (_, child) = stack.pop().unwrap();
        let parent = stack.last_mut().unwrap();
        let kind = parent.1["type"]
            .as_str()
            .unwrap_or("bulleted_list_item")
            .to_string();
        if let Some(children) = parent.1[&kind]["children"].as_array_mut() {
            children.push(child);
        }
    }

    if stack.is_empty() || indent == 0 || stack.len() >= MAX_DEPTH {
        // If deeper than stack top but already at max depth, just append flat
        if !stack.is_empty()
            && indent > stack.last().map(|(i, _)| *i).unwrap_or(0)
            && stack.len() >= MAX_DEPTH
        {
            let (_, child) = (indent, block);
            let parent = stack.last_mut().unwrap();
            let kind = parent.1["type"]
                .as_str()
                .unwrap_or("bulleted_list_item")
                .to_string();
            if let Some(children) = parent.1[&kind]["children"].as_array_mut() {
                children.push(child);
            }
        } else {
            stack.push((indent, block));
        }
    } else {
        stack.push((indent, block));
    }
}

fn flush_list_stack(stack: &mut Vec<(usize, Value)>, blocks: &mut Vec<Value>) {
    // Collapse the stack bottom-up: each item becomes a child of its parent
    while stack.len() > 1 {
        let (_, child) = stack.pop().unwrap();
        let parent = stack.last_mut().unwrap();
        let kind = parent.1["type"]
            .as_str()
            .unwrap_or("bulleted_list_item")
            .to_string();
        if let Some(children) = parent.1[&kind]["children"].as_array_mut() {
            children.push(child);
        }
    }
    if let Some((_, root)) = stack.pop() {
        blocks.push(root);
    }
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

    // Check for a Markdown link [label](url) at the earliest occurrence of '['
    let link_pos = try_parse_link(text);
    let bold_pos = text.find("**");
    let strike_pos = first_double_tilde(text);
    let italic_pos = first_single_star(text);
    let code_pos = text.find('`');

    // Find the earliest marker position
    let link_candidate = link_pos.as_ref().map(|(p, _, _, _)| *p);

    let anno_earliest = [
        bold_pos.map(|p| (p, "**", "bold")),
        strike_pos.map(|p| (p, "~~", "strikethrough")),
        italic_pos.map(|p| (p, "*", "italic")),
        code_pos.map(|p| (p, "`", "code")),
    ]
    .into_iter()
    .flatten()
    .min_by_key(|&(p, _, _)| p);

    // Pick whichever comes first: link or annotation marker
    let use_link = match (link_candidate, anno_earliest.as_ref()) {
        (Some(lp), Some(&(ap, _, _))) => lp <= ap,
        (Some(_), None) => true,
        _ => false,
    };

    if use_link {
        let (pos, label, url, end) = link_pos.unwrap();
        if pos > 0 {
            push_plain(&text[..pos], out);
        }
        for chunk in chunk_str(&label, MAX_TEXT_LEN) {
            out.push(json!({
                "type": "text",
                "text": { "content": chunk, "link": { "url": url } }
            }));
        }
        parse_inline(&text[end..], out);
        return;
    }

    match anno_earliest {
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

/// Parse a `[label](url)` link at the earliest `[`. Returns (pos, label, url, end_offset).
fn try_parse_link(text: &str) -> Option<(usize, String, String, usize)> {
    let pos = text.find('[')?;
    let rest = &text[pos + 1..];
    let close_bracket = rest.find(']')?;
    let after_bracket = &rest[close_bracket + 1..];
    if !after_bracket.starts_with('(') {
        return None;
    }
    let after_paren = &after_bracket[1..];
    let close_paren = after_paren.find(')')?;
    let label = rest[..close_bracket].to_string();
    let url = after_paren[..close_paren].to_string();
    let end = pos + 1 + close_bracket + 1 + 1 + close_paren + 1;
    Some((pos, label, url, end))
}

/// First `~~` that is not part of `~~~`.
fn first_double_tilde(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let mut i = 0;
    while i + 1 < text.len() {
        if b[i] == b'~' && b[i + 1] == b'~' {
            // Guard against triple tilde
            if b.get(i + 2) == Some(&b'~') {
                i += 3;
            } else {
                return Some(i);
            }
        } else {
            i += 1;
        }
    }
    None
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
/// For `*`, skips `**` sequences. For `~~`, skips `~~~`.
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
            if marker == "~~" && b.get(i + 2) == Some(&b'~') {
                i += 3;
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
