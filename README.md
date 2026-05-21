# Norse

A terminal UI for browsing GitHub repositories, commits, and pull requests across your organisations — with built-in Claude AI for diff summaries and codebase Q&A.

## Features

- **Repo picker** — fuzzy-search repos across multiple GitHub orgs, preview recent commits
- **Diff viewer** — read commit and PR diffs with automatic AI summaries
- **PR browser** — browse open PRs filtered by author, repo, review requests, or watched repos
- **AI Q&A** — ask questions about a diff or an entire codebase; conversation history is kept for follow-ups

See [docs/features.md](docs/features.md) for a full breakdown and [docs/keybindings.md](docs/keybindings.md) for all keybindings.

## Installation

Download a pre-built binary from the [releases page](../../releases), or build from source:

```sh
cargo build --release
```

The binary is output to `target/release/terminal`.

## Configuration

Create `~/.norse`:

```toml
[github]
token = "ghp_..."
orgs  = ["your-org", "another-org"]
```

Claude AI features are enabled automatically if [Claude Code](https://claude.ai/code) is installed — no extra configuration needed.

## Usage

```sh
./target/release/terminal
```

| Key | Action |
|-----|--------|
| `r` | Open repo picker |
| `p` | Open PR browser |
| `q` | Quit |
