# Keybindings

## Normal mode

| Key | Action |
|-----|--------|
| `r` | Open repo picker |
| `p` | Open PR browser |
| `q` | Quit |

## Repo picker — insert mode (default)

| Key | Action |
|-----|--------|
| Type | Filter repos |
| `Backspace` | Delete character |
| `Ctrl-l` | Move focus to preview / answer panel |
| `Esc` | Switch to normal mode (cursor stays in results) |

## Repo picker — normal mode

| Key | Action |
|-----|--------|
| `i` | Return to insert mode |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `c` | Collect / uncollect selected repo |
| `w` | Toggle GitHub watch on selected repo |
| `/` | Ask a question about the repo (or collected set) |
| `Ctrl-l` | Move focus to preview / answer panel |
| `Esc` | Return to normal mode |

## Repo picker — preview panel

| Key | Action |
|-----|--------|
| `j` / `↓` | Next commit |
| `k` / `↑` | Previous commit |
| `Enter` | Open commit diff |
| `Ctrl-h` | Return focus to results |

## Repo picker — answer panel

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl-d` | Scroll down half-page |
| `Ctrl-u` | Scroll up half-page |
| `Ctrl-h` | Return focus to results |

## Repo Q&A prompt

| Key | Action |
|-----|--------|
| Type | Enter question |
| `Enter` | Submit |
| `Backspace` | Delete character |
| `Ctrl-l` | Move focus to answer panel |
| `Ctrl-h` | Return focus to results |
| `Esc` | Dismiss prompt |

## Diff viewer

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down one line |
| `k` / `↑` | Scroll up one line |
| `Ctrl-d` | Scroll down half-page |
| `Ctrl-u` | Scroll up half-page |
| `/` | Ask a question about this diff |
| `Ctrl-x` | Open commit or PR in browser |
| `q` / `Esc` | Return to picker or PR browser |

## Diff Q&A prompt

| Key | Action |
|-----|--------|
| Type | Enter question |
| `Enter` | Submit |
| `Backspace` | Delete character |
| `Esc` | Dismiss prompt |

## PR browser

| Key | Action |
|-----|--------|
| `Ctrl-l` | Next tab |
| `Ctrl-h` | Previous tab |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Open PR diff |
| `Ctrl-x` | Open PR in browser |
| `r` | Refresh PR list |
| `q` / `Esc` | Return to normal mode |

### By People / By Repo tabs

Type to filter. `Backspace` removes a character. `Esc` clears the filter (second `Esc` returns to normal mode).
