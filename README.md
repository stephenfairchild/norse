# Norse

A terminal UI for browsing GitHub repositories, commits, and pull requests across your organisations — with built-in Claude AI for diff summaries and codebase Q&A.

## Features

- **Repo picker** — fuzzy-search repos across multiple GitHub orgs, preview recent commits
- **Diff viewer** — read commit and PR diffs with automatic AI summaries
- **PR browser** — browse open PRs filtered by author, repo, review requests, or watched repos
- **AI Q&A** — ask questions about a diff or an entire codebase; conversation history is kept for follow-ups

See [docs/features.md](docs/features.md) for a full breakdown and [docs/keybindings.md](docs/keybindings.md) for all keybindings.

## Requirements

- macOS, Linux, or Windows
- A terminal with 256-colour support
- A [GitHub personal access token](https://github.com/settings/tokens) with `repo` and `read:org` scopes

**Optional:**
- [Claude Code](https://claude.ai/code) (`gh` CLI must also be on `PATH`) — enables AI diff summaries and codebase Q&A

## Installation

Download a pre-built binary from the [releases page](../../releases) and put it somewhere on your `PATH`:

```sh
# macOS (Apple Silicon)
curl -L https://github.com/acv-auctions/norse/releases/latest/download/norse-macos-arm64 -o /usr/local/bin/norse
chmod +x /usr/local/bin/norse

# macOS (Intel)
curl -L https://github.com/acv-auctions/norse/releases/latest/download/norse-macos-amd64 -o /usr/local/bin/norse
chmod +x /usr/local/bin/norse

# Linux (x86-64)
curl -L https://github.com/acv-auctions/norse/releases/latest/download/norse-linux-amd64 -o /usr/local/bin/norse
chmod +x /usr/local/bin/norse

# Linux (ARM64)
curl -L https://github.com/acv-auctions/norse/releases/latest/download/norse-linux-arm64 -o /usr/local/bin/norse
chmod +x /usr/local/bin/norse

# Windows (x86-64) — run in PowerShell
curl -L https://github.com/acv-auctions/norse/releases/latest/download/norse-windows-amd64.exe -o norse.exe
```

Or build from source (requires [Rust](https://rustup.rs)):

```sh
cargo build --release
cp target/release/terminal /usr/local/bin/norse
```

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
