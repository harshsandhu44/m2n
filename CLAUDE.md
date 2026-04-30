# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                    # dev build
cargo build --release          # release build
cargo run -- <command> [args]  # run without installing
cargo test                     # run all tests
cargo clippy                   # lint
cargo fmt                      # format
```

Install locally for manual testing:
```bash
cargo install --path .
m2n --help
```

## Architecture

The codebase has two layers: `config.rs` (shared state) and `src/commands/` (one file per subcommand).

**`src/config.rs`** — the single source of truth for runtime state. `Config::load()` is called at the top of every command that needs it; commands that don't need config (e.g. future offline utilities) can skip it. Editor resolution order: `config.toml editor` field → `$EDITOR` → `nvim` → `vim` → `nano`.

**`src/commands/`** — each file exports one `pub fn run(...)` that matches what `main.rs` dispatches. Adding a new command means: create the file, add it to `commands/mod.rs`, add the variant to the `Command` enum in `main.rs`, and wire it in the `match`.

**Config location** — `dirs::config_dir()` resolves to `~/.config/m2n/config.toml` on Linux and `~/Library/Application Support/m2n/config.toml` on macOS. Tests or local overrides should not mutate this path.

**Note identity** — titles are slugified (lowercase, non-alphanumeric → `-`, consecutive dashes collapsed) to derive filenames. `write` accepts either a raw title or an exact file path; if neither exists it calls `new::run` to create the file before opening the editor.

**Notion token** — stored in `config.toml` under `[notion] token`. It must never be logged, printed, or included in error messages. `check::run` shows only `set` / `not set`.

**`push` command** is a stub — Notion API integration is not yet implemented.

## Frontmatter schema

Every note created by `new` gets:
```yaml
---
title: "..."
date: <ISO 8601 with offset>
status: draft
tags: []
---
```
Future frontmatter fields (e.g. `notion_id` after a push) should be added here and parsed via `serde`.
