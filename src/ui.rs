use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Focus, Mode, PrTab};

// Gruvbox dark palette
const BG: Color = Color::Rgb(40, 40, 40);
const BG1: Color = Color::Rgb(60, 56, 54);
const BG2: Color = Color::Rgb(80, 73, 69);
const FG: Color = Color::Rgb(235, 219, 178);
const YELLOW: Color = Color::Rgb(250, 189, 47);
const GREEN: Color = Color::Rgb(184, 187, 38);
const BLUE: Color = Color::Rgb(131, 165, 152);
const ORANGE: Color = Color::Rgb(254, 128, 25);
const GRAY: Color = Color::Rgb(168, 153, 132);
const RED: Color = Color::Rgb(251, 73, 52);
const PURPLE: Color = Color::Rgb(211, 134, 155);
const AQUA: Color = Color::Rgb(142, 192, 124);

const LANG_COLORS: [Color; 6] = [BLUE, YELLOW, GREEN, ORANGE, PURPLE, AQUA];

pub fn draw(f: &mut Frame, app: &App) {
    if matches!(app.mode, Mode::Diff) {
        draw_diff(f, app);
        return;
    }
    if matches!(app.mode, Mode::PrBrowser) {
        draw_pr_browser(f, app);
        return;
    }

    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG).fg(FG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    draw_main(f, chunks[0]);
    draw_statusbar(f, app, chunks[1]);

    if matches!(app.mode, Mode::Picker) {
        draw_picker(f, app, area);
    }
}

fn draw_main(f: &mut Frame, area: Rect) {
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let art = [
        r" _   _  ___  ____  ____  _____",
        r"| \ | |/ _ \|  _ \/ ___|| ____|",
        r"|  \| | | | | |_) \___ \|  _|  ",
        r"| |\  | |_| ||  _ < ___) || |___",
        r"|_| \_|\___/ |_| \_\____/ |_____|",
    ];

    let keys: &[(&str, &str)] = &[
        ("r",      "search repos"),
        ("p",      "browse PRs"),
        ("q",      "quit"),
    ];

    let art_width = art.iter().map(|l| l.len()).max().unwrap_or(0) as u16;
    let content_height = (art.len() + 1 + 1 + 1 + keys.len()) as u16;  // art + blank + subtitle + blank + keys

    let mut lines: Vec<Line> = art
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(YELLOW).add_modifier(Modifier::BOLD))))
        .collect();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "source code tools",
        Style::default().fg(GRAY),
    )));
    lines.push(Line::from(""));
    for (key, desc) in keys {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<8}", key), Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)),
            Span::styled("  ", Style::default()),
            Span::styled(*desc, Style::default().fg(GRAY)),
        ]));
    }

    let rect = centered_rect_abs(art_width, content_height, area);
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), rect);
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let mode_label = match app.mode {
        Mode::Normal => Span::styled(" NORMAL ", Style::default().fg(BG).bg(GREEN).add_modifier(Modifier::BOLD)),
        Mode::Picker => Span::styled(" PICKER ", Style::default().fg(BG).bg(BLUE).add_modifier(Modifier::BOLD)),
        Mode::Diff => Span::styled(" DIFF ", Style::default().fg(BG).bg(ORANGE).add_modifier(Modifier::BOLD)),
        Mode::PrBrowser => Span::styled(" PRs ", Style::default().fg(BG).bg(YELLOW).add_modifier(Modifier::BOLD)),
    };
    let orgs = Span::styled(" maxsystems · acv-auctions ", Style::default().fg(GRAY).bg(BG1));
    f.render_widget(
        Paragraph::new(Line::from(vec![mode_label, orgs])).style(Style::default().bg(BG1)),
        area,
    );
}

fn draw_picker(f: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(90, 80, area);
    f.render_widget(Clear, popup);

    let rows = if app.repo_prompt_active {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(3)])
            .split(popup)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(popup)
    };

    let (input_border, mode_indicator, cursor) = if app.picker_insert {
        (ORANGE, Span::styled(" -- INSERT -- ", Style::default().fg(ORANGE)), "█")
    } else {
        (BG2, Span::styled(" -- NORMAL -- ", Style::default().fg(GRAY)), "")
    };
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(YELLOW)),
        Span::styled(&app.search.query, Style::default().fg(FG)),
        Span::styled(cursor, Style::default().fg(ORANGE)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(input_border))
            .title(Span::styled(" Search repos ", Style::default().fg(YELLOW)))
            .title_bottom(mode_indicator)
            .style(Style::default().bg(BG)),
    );
    f.render_widget(input, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(rows[1]);

    draw_results(f, app, cols[0]);

    let showing_repo_answer = app.repo_answer_loading || !app.repo_answer.is_empty();
    if showing_repo_answer {
        draw_repo_answer(f, app, cols[1]);
    } else {
        draw_preview(f, app, cols[1]);
    }

    if app.repo_prompt_active {
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE))
            .title(Span::styled(" Ask about this repo ", Style::default().fg(YELLOW)))
            .style(Style::default().bg(BG));
        let prompt_input = Paragraph::new(Line::from(vec![
            Span::styled("/", Style::default().fg(YELLOW)),
            Span::styled(app.repo_prompt_input.as_str(), Style::default().fg(FG)),
            Span::styled("█", Style::default().fg(ORANGE)),
        ])).block(prompt_block);
        f.render_widget(prompt_input, rows[2]);
    }
}

fn draw_repo_answer(f: &mut Frame, app: &App, area: Rect) {
    let selected_repo = app.search.results.get(app.search.selected).map(|r| r.repo.as_str()).unwrap_or("");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(AQUA))
        .title(Span::styled(format!(" {} ", selected_repo), Style::default().fg(AQUA).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG));

    if app.repo_answer_loading {
        let mut lines: Vec<Line> = app.repo_progress.iter().map(|msg| {
            if msg.starts_with("$ ") {
                Line::from(Span::styled(msg.clone(), Style::default().fg(YELLOW)))
            } else {
                Line::from(Span::styled(msg.clone(), Style::default().fg(GRAY)))
            }
        }).collect();
        if lines.is_empty() {
            lines.push(Line::from(Span::styled(" Thinking…", Style::default().fg(GRAY))));
        }
        f.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    } else {
        f.render_widget(
            Paragraph::new(app.repo_answer.as_str())
                .block(block)
                .style(Style::default().fg(FG).bg(BG))
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

fn draw_results(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focus, Focus::Results);
    let border_color = if focused { ORANGE } else { BG2 };

    let title = if app.search.loading {
        " searching… ".to_string()
    } else if let Some(ref err) = app.search.error {
        format!(" {} ", err)
    } else {
        format!(" {} repos ", app.search.results.len())
    };
    let title_style = if app.search.error.is_some() {
        Style::default().fg(RED)
    } else {
        Style::default().fg(GRAY)
    };

    let items: Vec<ListItem> = app.search.results.iter().enumerate().map(|(i, r)| {
        let is_selected = i == app.search.selected;
        let (bg, dimmed) = if is_selected { (BLUE, BLUE) } else { (BG, BG) };
        let (org, name) = r.repo.split_once('/').unwrap_or(("", &r.repo));
        let w_mark = if app.watched_repos.contains(&r.repo) {
            Span::styled("W ", Style::default().fg(YELLOW).bg(dimmed))
        } else {
            Span::styled("  ", Style::default().bg(dimmed))
        };
        ListItem::new(Line::from(vec![
            w_mark,
            Span::styled(format!(" {}/", org), Style::default().fg(GRAY).bg(dimmed)),
            Span::styled(format!("{} ", name), Style::default().fg(if is_selected { BG } else { FG }).bg(bg).add_modifier(Modifier::BOLD)),
        ]))
    }).collect();

    let mut list_state = ListState::default();
    if !app.search.results.is_empty() {
        list_state.select(Some(app.search.selected));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(title, title_style))
                .style(Style::default().bg(BG)),
        )
        .highlight_style(Style::default().bg(BLUE));

    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let focused = matches!(app.focus, Focus::Preview);
    let border_color = if focused { ORANGE } else { BG2 };

    let selected_repo = app.search.results.get(app.search.selected).map(|r| r.repo.as_str()).unwrap_or("");

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {} ", selected_repo), Style::default().fg(YELLOW)))
        .style(Style::default().bg(BG));

    if app.search.preview_loading {
        f.render_widget(
            Paragraph::new(Span::styled(" loading…", Style::default().fg(GRAY))).block(block),
            area,
        );
        return;
    }

    let Some(ref preview) = app.search.preview else {
        f.render_widget(block, area);
        return;
    };

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lang_count = preview.languages.len().min(6) as u16;
    // header + bars + blank line
    let langs_height = 1 + lang_count + 1;
    // "Recent Commits" label
    let commits_label_height = 1;

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(langs_height),
            Constraint::Length(commits_label_height),
            Constraint::Min(1),
        ])
        .split(inner);

    // Languages
    let bar_width = (sections[0].width as usize).saturating_sub(20).max(8);
    let mut lang_lines = vec![Line::from(Span::styled("Languages", Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)))];
    for (i, (lang, pct)) in preview.languages.iter().take(6).enumerate() {
        let color = LANG_COLORS[i % LANG_COLORS.len()];
        let filled = ((pct / 100.0) * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        lang_lines.push(Line::from(vec![
            Span::styled(format!("{:<12}", lang), Style::default().fg(FG)),
            Span::styled(format!("{:>5.1}%  ", pct), Style::default().fg(GRAY)),
            Span::styled("█".repeat(filled), Style::default().fg(color)),
            Span::styled("░".repeat(empty), Style::default().fg(BG2)),
        ]));
    }
    f.render_widget(Paragraph::new(lang_lines).style(Style::default().bg(BG)), sections[0]);

    // Commits label
    f.render_widget(
        Paragraph::new(Span::styled(
            if focused { "Recent Commits  (j/k navigate · enter open diff · ctrl-h back)" } else { "Recent Commits  (ctrl-l to focus)" },
            Style::default().fg(ORANGE).add_modifier(if focused { Modifier::BOLD } else { Modifier::empty() }),
        )).style(Style::default().bg(BG)),
        sections[1],
    );

    // Commits list
    let avail = sections[2].width as usize;
    let items: Vec<ListItem> = preview.commits.iter().enumerate().map(|(i, commit)| {
        let is_selected = focused && i == app.preview_commit_selected;
        let bg = if is_selected { BLUE } else { BG };
        let msg_width = avail.saturating_sub(7 + 1 + 14 + 1 + 10 + 1 + 2);
        let msg = if commit.message.len() > msg_width {
            format!("{}…", &commit.message[..msg_width.saturating_sub(1)])
        } else {
            commit.message.clone()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{} ", commit.sha), Style::default().fg(YELLOW).bg(bg)),
            Span::styled(format!("{:<14} ", trunc(&commit.author, 14)), Style::default().fg(AQUA).bg(bg)),
            Span::styled(format!("{:<width$} ", msg, width = msg_width), Style::default().fg(if is_selected { BG } else { FG }).bg(bg)),
            Span::styled(commit.date.as_str(), Style::default().fg(GRAY).bg(bg)),
        ]))
    }).collect();

    let mut list_state = ListState::default();
    if focused && !preview.commits.is_empty() {
        list_state.select(Some(app.preview_commit_selected));
    }

    let list = List::new(items)
        .style(Style::default().bg(BG))
        .highlight_style(Style::default().bg(BLUE));

    f.render_stateful_widget(list, sections[2], &mut list_state);
}

fn draw_diff(f: &mut Frame, app: &App) {
    let area = f.area();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: diff — render block and paragraph separately so the paragraph is
    // given the exact inner rect and long lines are clipped at the border.
    let diff_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(format!(" {} ", app.diff_header), Style::default().fg(YELLOW)))
        .title_bottom(Span::styled(" j/k line  ctrl-d/u page  ctrl-x open PR  q back ", Style::default().fg(GRAY)))
        .style(Style::default().bg(BG));

    let diff_inner = diff_block.inner(cols[0]);
    f.render_widget(diff_block, cols[0]);

    let diff_lines: Vec<Line> = if app.diff_loading {
        vec![Line::from(Span::styled(" loading diff…", Style::default().fg(GRAY)))]
    } else {
        app.diff_lines.iter().flat_map(|line| wrap_diff_line(line, 100)).collect()
    };

    f.render_widget(
        Paragraph::new(diff_lines).scroll((app.diff_scroll as u16, 0)),
        diff_inner,
    );

    // Right: AI summary / answer + optional prompt input
    let right_chunks = if app.diff_prompt_active {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(cols[1])
            .to_vec()
    } else {
        vec![cols[1]]
    };

    let showing_answer = app.diff_answer_loading || !app.diff_answer.is_empty();
    let (panel_title, panel_color) = if showing_answer {
        (" Answer ", AQUA)
    } else {
        (" AI Summary ", PURPLE)
    };

    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(panel_color))
        .title(Span::styled(panel_title, Style::default().fg(panel_color).add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(" / ask a question ", Style::default().fg(GRAY)))
        .style(Style::default().bg(BG));

    if app.diff_answer_loading {
        f.render_widget(
            Paragraph::new(Span::styled(" Thinking…", Style::default().fg(GRAY)))
                .block(summary_block),
            right_chunks[0],
        );
    } else if showing_answer {
        f.render_widget(
            Paragraph::new(app.diff_answer.as_str())
                .block(summary_block)
                .style(Style::default().fg(FG).bg(BG))
                .wrap(Wrap { trim: false }),
            right_chunks[0],
        );
    } else if app.summary_loading {
        f.render_widget(
            Paragraph::new(Span::styled(" Summarizing…", Style::default().fg(GRAY)))
                .block(summary_block),
            right_chunks[0],
        );
    } else {
        f.render_widget(
            Paragraph::new(app.summary.as_str())
                .block(summary_block)
                .style(Style::default().fg(FG).bg(BG))
                .wrap(Wrap { trim: false }),
            right_chunks[0],
        );
    }

    if app.diff_prompt_active {
        let prompt_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE))
            .style(Style::default().bg(BG));
        let input = Paragraph::new(Line::from(vec![
            Span::styled("/", Style::default().fg(YELLOW)),
            Span::styled(app.diff_prompt_input.as_str(), Style::default().fg(FG)),
            Span::styled("█", Style::default().fg(ORANGE)),
        ])).block(prompt_block);
        f.render_widget(input, right_chunks[1]);
    }
}

fn draw_pr_browser(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(BG).fg(FG)), area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title_bottom(Span::styled(
            " ctrl-h/l tabs · ↑↓ navigate · enter open · q back ",
            Style::default().fg(GRAY),
        ))
        .style(Style::default().bg(BG));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    // Split inner into tab bar + optional search input + list
    let has_search = matches!(app.pr_tab, PrTab::ByPeople | PrTab::ByRepo);
    let chunks = if has_search {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Min(1)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner)
    };

    // Tab bar
    let tab_spans = vec![
        tab_span("  All PRs  ", matches!(app.pr_tab, PrTab::All)),
        Span::raw("  "),
        tab_span("  By People  ", matches!(app.pr_tab, PrTab::ByPeople)),
        Span::raw("  "),
        tab_span("  By Repo  ", matches!(app.pr_tab, PrTab::ByRepo)),
        Span::raw("  "),
        tab_span("  Review Requests  ", matches!(app.pr_tab, PrTab::ReviewsRequested)),
        Span::raw("  "),
        tab_span("  Watching  ", matches!(app.pr_tab, PrTab::Watching)),
    ];
    f.render_widget(Paragraph::new(Line::from(tab_spans)).style(Style::default().bg(BG)), chunks[0]);

    // Search input (ByPeople / ByRepo only)
    let list_chunk = if has_search {
        let (query, placeholder) = match app.pr_tab {
            PrTab::ByPeople => (app.pr_people_query.as_str(), "filter by username"),
            PrTab::ByRepo   => (app.pr_repo_query.as_str(),   "filter by repo"),
            _ => unreachable!(),
        };
        let hint = if query.is_empty() {
            Span::styled(placeholder, Style::default().fg(BG2))
        } else {
            Span::styled(query, Style::default().fg(FG))
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(YELLOW)),
                hint,
                Span::styled("█", Style::default().fg(ORANGE)),
            ])).style(Style::default().bg(BG)),
            chunks[1],
        );
        chunks[2]
    } else {
        chunks[1]
    };

    // Build the list for the active tab
    let (prs, selected_idx, loading, error): (Vec<&crate::github::PrItem>, usize, bool, Option<&str>) =
        match app.pr_tab {
            PrTab::All => (
                app.pr_items.iter().collect(),
                app.pr_all_selected,
                app.pr_loading,
                app.pr_error.as_deref(),
            ),
            PrTab::ByPeople => (
                app.pr_people_filtered(),
                app.pr_people_selected,
                app.pr_loading,
                app.pr_error.as_deref(),
            ),
            PrTab::ByRepo => (
                app.pr_repo_filtered(),
                app.pr_repo_selected,
                app.pr_loading,
                app.pr_error.as_deref(),
            ),
            PrTab::ReviewsRequested => (
                app.pr_reviews_items.iter().collect(),
                app.pr_reviews_selected,
                app.pr_reviews_loading,
                app.pr_reviews_error.as_deref(),
            ),
            PrTab::Watching => (
                app.pr_watching_filtered(),
                app.pr_watching_selected,
                app.pr_loading,
                app.pr_error.as_deref(),
            ),
        };

    if loading {
        f.render_widget(
            Paragraph::new(Span::styled(" loading…", Style::default().fg(GRAY)))
                .style(Style::default().bg(BG)),
            list_chunk,
        );
        return;
    }
    if let Some(e) = error {
        f.render_widget(
            Paragraph::new(Span::styled(format!(" error: {}", e), Style::default().fg(RED)))
                .style(Style::default().bg(BG)),
            list_chunk,
        );
        return;
    }

    let avail = list_chunk.width as usize;
    let repo_w = 32usize;
    let num_w = 6usize;
    let author_w = 14usize;
    let ago_w = 12usize;
    let title_w = avail.saturating_sub(repo_w + num_w + author_w + ago_w + 4 + 2);

    let items: Vec<ListItem> = prs.iter().enumerate().map(|(i, pr)| {
        let sel = i == selected_idx;
        let bg = if sel { BLUE } else { BG };
        let fg = if sel { BG } else { FG };
        let title = if pr.title.len() > title_w {
            format!("{}…", &pr.title[..title_w.saturating_sub(1)])
        } else {
            pr.title.clone()
        };
        let ago = time_ago(&pr.created_at);
        let draft = if pr.draft { Span::styled(" draft", Style::default().fg(ORANGE).bg(bg)) }
                    else { Span::raw("") };
        let w_mark = if app.watched_repos.contains(&pr.repo) {
            Span::styled("W ", Style::default().fg(YELLOW).bg(bg))
        } else {
            Span::styled("  ", Style::default().bg(bg))
        };
        ListItem::new(Line::from(vec![
            w_mark,
            Span::styled(format!("{:<repo_w$}", trunc(&pr.repo, repo_w)), Style::default().fg(GRAY).bg(bg)),
            Span::styled(format!(" #{:<num_w$}", pr.number), Style::default().fg(YELLOW).bg(bg)),
            Span::styled(format!("{:<title_w$} ", title), Style::default().fg(fg).bg(bg)),
            Span::styled(format!("{:<author_w$} ", trunc(&pr.author, author_w)), Style::default().fg(AQUA).bg(bg)),
            Span::styled(format!("{:<ago_w$}", ago), Style::default().fg(GRAY).bg(bg)),
            draft,
        ]))
    }).collect();

    let mut list_state = ListState::default();
    if !prs.is_empty() {
        list_state.select(Some(selected_idx));
    }

    f.render_stateful_widget(
        List::new(items)
            .style(Style::default().bg(BG))
            .highlight_style(Style::default().bg(BLUE)),
        list_chunk,
        &mut list_state,
    );
}

fn time_ago(iso: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let then = parse_iso8601(iso).unwrap_or(now);
    let secs = now.saturating_sub(then);
    if secs < 60 {
        "just now".into()
    } else if secs < 3600 {
        let m = secs / 60;
        format!("{} min ago", m)
    } else if secs < 86400 {
        let h = secs / 3600;
        format!("{} hr{} ago", h, if h == 1 { "" } else { "s" })
    } else if secs < 86400 * 7 {
        let d = secs / 86400;
        format!("{} day{} ago", d, if d == 1 { "" } else { "s" })
    } else if secs < 86400 * 30 {
        let w = secs / (86400 * 7);
        format!("{} wk{} ago", w, if w == 1 { "" } else { "s" })
    } else if secs < 86400 * 365 {
        let mo = secs / (86400 * 30);
        format!("{} mo ago", mo)
    } else {
        let y = secs / (86400 * 365);
        format!("{} yr{} ago", y, if y == 1 { "" } else { "s" })
    }
}

fn parse_iso8601(s: &str) -> Option<u64> {
    let s = s.trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let mut dp = date.split('-');
    let mut tp = time.split(':');
    let year: u64  = dp.next()?.parse().ok()?;
    let month: u64 = dp.next()?.parse().ok()?;
    let day: u64   = dp.next()?.parse().ok()?;
    let hour: u64  = tp.next()?.parse().ok()?;
    let min: u64   = tp.next()?.parse().ok()?;
    let sec: u64   = tp.next()?.parse().ok()?;

    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let months = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 0..(month as usize).saturating_sub(1) {
        days += months[m];
    }
    days += day.saturating_sub(1);
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn tab_span(label: &'static str, active: bool) -> Span<'static> {
    if active {
        Span::styled(label, Style::default().fg(BG).bg(ORANGE).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(label, Style::default().fg(GRAY).bg(BG1))
    }
}

fn wrap_diff_line(line: &str, max: usize) -> Vec<Line<'static>> {
    let style = diff_line_style(line);
    let mut result = Vec::new();
    let mut remaining = line;
    loop {
        if remaining.len() <= max {
            result.push(Line::from(Span::styled(remaining.to_string(), style)));
            break;
        }
        let mut split = max;
        while !remaining.is_char_boundary(split) {
            split -= 1;
        }
        result.push(Line::from(Span::styled(remaining[..split].to_string(), style)));
        remaining = &remaining[split..];
    }
    result
}

fn diff_line_style(line: &str) -> Style {
    if line.starts_with("+++") || line.starts_with("---") {
        Style::default().fg(FG).add_modifier(Modifier::BOLD)
    } else if line.starts_with('+') {
        Style::default().fg(GREEN)
    } else if line.starts_with('-') {
        Style::default().fg(RED)
    } else if line.starts_with("@@") {
        Style::default().fg(AQUA)
    } else if line.starts_with("diff ") || line.starts_with("index ") || line.starts_with("new file") || line.starts_with("deleted file") {
        Style::default().fg(GRAY)
    } else {
        Style::default().fg(FG)
    }
}

fn trunc(s: &str, max: usize) -> &str {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn centered_rect_abs(width: u16, height: u16, r: Rect) -> Rect {
    Rect::new(
        r.x + r.width.saturating_sub(width) / 2,
        r.y + r.height.saturating_sub(height) / 2,
        width.min(r.width),
        height.min(r.height),
    )
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
