# Features

## Repo Picker

Search repositories across your configured GitHub organisations. Results appear as you type with a 300ms debounce. Select a repo to see its recent commits in the preview panel.

**Collect repos** — mark multiple repos with `c` to group them together. Collected repos are pinned to the bottom of the list and treated as a set when asking AI questions.

**Watch repos** — toggle GitHub watch status on a repo with `w`. Watched repos appear in the PR browser's Watching tab.

## Diff Viewer

Opens the full unified diff for a commit or pull request. A Claude-powered summary is generated automatically when the diff loads (requires Claude API configuration).

Ask follow-up questions about the diff with `/`. The answer panel appears inline below the summary.

Open the commit or PR in GitHub with `Ctrl-x`.

## PR Browser

Fetches open pull requests across your organisations. Five tabs filter the list in different ways:

| Tab | Description |
|-----|-------------|
| All | Every open PR, sorted by creation date |
| By People | Filter by author name |
| By Repo | Filter by repository name |
| Reviews Requested | PRs where your review has been requested |
| Watching | PRs in repos you watch |

Select a PR and press `Enter` to open its diff. Press `r` to refresh.

## AI — Repo Q&A

From the repo picker, press `/` to ask a question about the selected repo (or all collected repos). Norse fetches the codebase context and answers using Claude. Conversation history is kept for follow-up questions.

## AI — Diff Q&A

From the diff viewer, press `/` to ask a question about the current diff. Useful for understanding the intent behind a change or getting a second opinion on an approach.

## Configuration

Create `~/.norse`:

```toml
[github]
token = "ghp_..."
orgs  = ["my-org", "another-org"]
```

Claude API access is read automatically from `~/.claude/settings.json` if Claude Code is installed.
