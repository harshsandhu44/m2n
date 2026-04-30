# m2n — Markdown to Notion

A local-first CLI for writing Markdown notes and publishing them directly to Notion.

Write in your editor. Sync to Notion. No browser required.

## Features

- **Write locally** — notes live as Markdown files with YAML frontmatter
- **Push to Notion** — full Markdown-to-Notion-blocks conversion (headings, lists, code, inline formatting)
- **One-step flow** — `m2n write "My Note"` opens your editor, then pushes on save
- **Any editor** — respects `$EDITOR`, or configure one explicitly
- **Database-aware** — auto-detects title, status, and tags properties in your Notion database

## Installation

### Pre-built binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/harshsandhu44/m2n/releases).

```bash
# macOS (Apple Silicon)
curl -L https://github.com/harshsandhu44/m2n/releases/latest/download/m2n-aarch64-apple-darwin.tar.gz | tar xz
sudo mv m2n /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/harshsandhu44/m2n/releases/latest/download/m2n-x86_64-apple-darwin.tar.gz | tar xz
sudo mv m2n /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/harshsandhu44/m2n/releases/latest/download/m2n-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv m2n /usr/local/bin/
```

### From source

Requires [Rust](https://rustup.rs/) (stable).

```bash
cargo install --git https://github.com/harshsandhu44/m2n
```

Or clone and build:

```bash
git clone https://github.com/harshsandhu44/m2n
cd m2n
cargo install --path .
```

## Prerequisites

m2n requires a **Notion integration** with access to a database.

1. Go to [notion.so/my-integrations](https://www.notion.so/my-integrations) and create a new integration
2. Copy the **Internal Integration Token**
3. Open your Notion database → **⋯ menu** → **Connections** → add your integration
4. Copy the database URL (e.g. `https://www.notion.so/myworkspace/abc123...`)

## Quick Start

```bash
m2n init
```

This walks you through connecting to your Notion database and saves config to `~/.config/m2n/config.toml`.

Then write your first note:

```bash
m2n write "My First Note"
```

Your editor opens with a pre-populated Markdown file. Save and close — m2n pushes it to Notion automatically.

## Commands

### `m2n init`

Interactive setup wizard. Prompts for your Notion token, database URL, and notes directory, tests the connection, and saves config.

```bash
m2n init
```

### `m2n write <title>`

Open a note in your editor, creating it if it doesn't exist, then sync to Notion on close.

```bash
m2n write "Meeting Notes 2026-05-01"
```

New notes are pre-populated with frontmatter:

```yaml
---
title: "Meeting Notes 2026-05-01"
date: 2026-05-01T10:30:00-07:00
status: draft
tags: []
---

# Meeting Notes 2026-05-01
```

Notes are saved to your configured `notes_dir` and persist locally across sessions.

### `m2n edit <title>`

Open an existing note in your editor and sync changes to Notion on close. Fails if the note doesn't exist — use `m2n write` to create it first.

```bash
m2n edit "Meeting Notes 2026-05-01"
```

### `m2n list`

List all notes in your notes directory with their sync status.

```bash
m2n list
```

Sample output:

```
Meeting Notes 2026-05-01    [synced]
Quick Idea                  [draft]
Old Post                    [unsynced]
```

### `m2n new <title>`

Create a blank Notion page directly (no editor, no local file).

```bash
m2n new "Quick Idea"
```

### `m2n push <path>`

Push an existing Markdown file to Notion.

```bash
m2n push ./notes/my-note.md
m2n push "My Note Title"     # resolved by slugified title

m2n push ./my-note.md --dry-run   # preview blocks without creating
m2n push ./my-note.md --open      # open in browser after pushing
```

Flags:

| Flag | Description |
|------|-------------|
| `--dry-run` | Print the Notion blocks that would be created, without making any API calls |
| `--open` | Open the resulting Notion page in your default browser |

### `m2n check`

Verify your config, editor resolution, and Notion connectivity.

```bash
m2n check
```

Sample output:

```
config path : /Users/you/.config/m2n/config.toml
editor      : nvim  ($EDITOR not set)
notion token: set
database id : set
auth        : ok (My Integration)
title prop  : Name
status prop : Status
tags prop   : Tags
```

## Configuration

Config lives at `~/.config/m2n/config.toml` (Linux/macOS) or `%APPDATA%\m2n\config.toml` (Windows).

```toml
[notion]
token = "secret_..."
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"

editor = "nvim"  # optional, overrides $EDITOR
```

**Editor resolution order:** `config.toml editor` → `$EDITOR` env var → `nvim` → `vim` → `nano`

## Markdown Support

m2n converts Markdown to native Notion blocks:

| Markdown | Notion block |
|----------|-------------|
| `# Heading` | Heading 1 |
| `## Heading` | Heading 2 |
| `### Heading` | Heading 3 |
| `- item` / `* item` | Bulleted list |
| `1. item` | Numbered list |
| `> quote` | Quote |
| ` ``` lang ` | Code block |
| `---` | Divider |
| `**bold**` | Bold text |
| `*italic*` | Italic text |
| `` `code` `` | Inline code |
| `~~text~~` | Strikethrough |
| `[label](url)` | Link |
| `- [ ] item` | To-do (unchecked) |
| `- [x] item` | To-do (checked) |
| Indented `- item` | Nested list |

Supported code block languages: Rust, JavaScript, TypeScript, Python, Bash/Shell, Go, Java, C, C++, C#, Ruby, SQL, JSON, YAML, HTML, CSS, Markdown, TOML, and more.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get started.

## License

MIT — see [LICENSE](LICENSE).
