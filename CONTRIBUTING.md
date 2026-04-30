# Contributing to m2n

Thanks for your interest in contributing.

## Development setup

Requires [Rust](https://rustup.rs/) stable.

```bash
git clone https://github.com/harshsandhu44/m2n
cd m2n
cargo build
```

Install locally for manual testing:

```bash
cargo install --path .
m2n --help
```

## Before you start

- For bug fixes and small improvements, just open a PR.
- For new features or larger changes, open an issue first to discuss the approach.

## Code style

Run these before committing:

```bash
cargo fmt        # format
cargo clippy     # lint (zero warnings expected)
cargo test       # all tests must pass
```

The CI pipeline enforces all three.

## Architecture

The codebase has two layers:

- **`src/config.rs`** — shared runtime state. `Config::load()` is called at the top of every command that needs it.
- **`src/commands/`** — one file per subcommand, each exporting a `pub fn run(...)`.

Adding a new command:

1. Create `src/commands/<name>.rs` with `pub fn run(...) -> anyhow::Result<()>`
2. Add it to `src/commands/mod.rs`
3. Add the variant to the `Command` enum in `src/main.rs`
4. Wire it in the `match` block

## Security

The Notion token must **never** appear in logs, error messages, or debug output. `check::run` shows only `set` / `not set`. Keep this invariant when touching any code that reads or passes the token.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add --tags flag to push command
fix: handle empty frontmatter gracefully
docs: update installation instructions
```

Types used in the changelog: `feat`, `fix`, `docs`, `perf`, `refactor`.  
Types ignored by the changelog: `chore`, `ci`, `test`.

## Pull requests

- Keep PRs focused — one logical change per PR.
- Include a short description of what changed and why.
- Link any related issues.

## Reporting issues

Use [GitHub Issues](https://github.com/harshsandhu44/m2n/issues). For bugs, include:

- m2n version (`m2n --version`)
- OS and architecture
- Steps to reproduce
- Expected vs. actual behavior
- Output of `m2n check` (token value will show as `set`, safe to include)
