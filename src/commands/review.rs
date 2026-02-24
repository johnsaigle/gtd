use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::gtd;
use crate::markdown;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewPhase {
    Intro,
    InTray,
    NextActions,
    WaitingFor,
    Projects,
    SomedayMaybe,
    Summary,
}

impl ReviewPhase {
    fn next(self) -> Option<Self> {
        match self {
            Self::Intro => Some(Self::InTray),
            Self::InTray => Some(Self::NextActions),
            Self::NextActions => Some(Self::WaitingFor),
            Self::WaitingFor => Some(Self::Projects),
            Self::Projects => Some(Self::SomedayMaybe),
            Self::SomedayMaybe => Some(Self::Summary),
            Self::Summary => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Intro => "Weekly Review",
            Self::InTray => "1. Clear the In-Tray",
            Self::NextActions => "2. Review Next Actions",
            Self::WaitingFor => "3. Review Waiting-For",
            Self::Projects => "4. Review Projects",
            Self::SomedayMaybe => "5. Review Someday/Maybe",
            Self::Summary => "Review Complete",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Intro => {
                "The weekly review keeps your GTD system trustworthy. Walk through each list, update what's changed, and capture anything new."
            }
            Self::InTray => {
                "Process everything in your in-tray to zero. Each item needs a decision: actionable or not?"
            }
            Self::NextActions => {
                "Review each next action. Is it still relevant? Still the right next step? Remove anything done or no longer needed."
            }
            Self::WaitingFor => {
                "Check on everything you're waiting for. Follow up if needed. Remove anything that's been received."
            }
            Self::Projects => {
                "Review each project. Does it have a clear next action? Is it still active? Any new tasks to add?"
            }
            Self::SomedayMaybe => {
                "Scan your someday/maybe list. Anything ready to activate? Anything to remove? Any new ideas to add?"
            }
            Self::Summary => "Weekly review complete! Your system is up to date.",
        }
    }

    fn all() -> &'static [ReviewPhase] {
        &[
            Self::Intro,
            Self::InTray,
            Self::NextActions,
            Self::WaitingFor,
            Self::Projects,
            Self::SomedayMaybe,
            Self::Summary,
        ]
    }
}

struct ReviewApp {
    phase: ReviewPhase,
    should_quit: bool,
    scroll: u16,
    // Cached data
    intray_count: usize,
    next_actions: Vec<String>,
    waiting_for: Vec<String>,
    projects: Vec<(String, usize)>, // (name, open_task_count)
    someday_count: usize,
}

impl ReviewApp {
    fn new() -> Result<Self> {
        let intray_path = gtd::in_tray_dir().join("inbox.md");
        let intray_tasks = markdown::parse_tasks(&intray_path)?;

        let next = markdown::parse_tasks(&gtd::tasks_file())?;
        let waiting = markdown::parse_tasks(&gtd::waiting_for_file())?;
        let someday = markdown::parse_tasks(&gtd::someday_maybe_file())?;

        let project_names = gtd::list_projects()?;
        let mut projects = Vec::new();
        for name in &project_names {
            let tasks = markdown::parse_tasks(&gtd::project_tasks_file(name))?;
            let open = tasks.iter().filter(|t| !t.done).count();
            projects.push((name.clone(), open));
        }

        Ok(Self {
            phase: ReviewPhase::Intro,
            should_quit: false,
            scroll: 0,
            intray_count: intray_tasks.len(),
            next_actions: next
                .iter()
                .filter(|t| !t.done)
                .map(|t| t.to_string())
                .collect(),
            waiting_for: waiting
                .iter()
                .filter(|t| !t.done)
                .map(|t| t.to_string())
                .collect(),
            projects,
            someday_count: someday.iter().filter(|t| !t.done).count(),
        })
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                if let Some(next) = self.phase.next() {
                    self.phase = next;
                    self.scroll = 0;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = self.scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let layout = Layout::vertical([
            Constraint::Length(3), // header / progress
            Constraint::Length(5), // phase description
            Constraint::Min(10),   // content
            Constraint::Length(3), // footer
        ])
        .split(area);

        if let (Some(header), Some(desc), Some(content), Some(footer)) =
            (layout.first(), layout.get(1), layout.get(2), layout.get(3))
        {
            self.render_progress(frame, *header);
            self.render_description(frame, *desc);
            self.render_content(frame, *content);
            self.render_footer(frame, *footer);
        }
    }

    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        let phases = ReviewPhase::all();
        let current_idx = phases.iter().position(|&p| p == self.phase).unwrap_or(0);

        let mut spans = vec![Span::raw(" ")];
        for (i, _phase) in phases.iter().enumerate() {
            let style = if i < current_idx {
                Style::default().fg(Color::Green)
            } else if i == current_idx {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let marker = if i < current_idx {
                "●"
            } else if i == current_idx {
                "◉"
            } else {
                "○"
            };
            spans.push(Span::styled(format!(" {} ", marker), style));
        }

        let block = Block::default()
            .title(format!(" {} ", self.phase.title()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let paragraph = Paragraph::new(Line::from(spans)).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_description(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let paragraph = Paragraph::new(format!("  {}", self.phase.description()))
            .block(block)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(Color::White));
        frame.render_widget(paragraph, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Items ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        match self.phase {
            ReviewPhase::Intro => {
                let lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  In-Tray:       {} items", self.intray_count),
                        if self.intray_count > 0 {
                            Style::default().fg(Color::Yellow).bold()
                        } else {
                            Style::default().fg(Color::Green)
                        },
                    )),
                    Line::from(Span::styled(
                        format!("  Next Actions:  {} tasks", self.next_actions.len()),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("  Waiting For:   {} items", self.waiting_for.len()),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("  Projects:      {}", self.projects.len()),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("  Someday/Maybe: {} items", self.someday_count),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press Enter to begin the review...",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let paragraph = Paragraph::new(lines).block(block);
                frame.render_widget(paragraph, area);
            }
            ReviewPhase::InTray => {
                let text = if self.intray_count == 0 {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  In-tray is empty! Inbox zero.",
                            Style::default().fg(Color::Green).bold(),
                        )),
                    ]
                } else {
                    vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            format!(
                                "  {} items in in-tray. Run `gtd process` after review to clear them.",
                                self.intray_count
                            ),
                            Style::default().fg(Color::Yellow).bold(),
                        )),
                    ]
                };
                let paragraph = Paragraph::new(text).block(block).scroll((self.scroll, 0));
                frame.render_widget(paragraph, area);
            }
            ReviewPhase::NextActions => {
                let items: Vec<ListItem> = self
                    .next_actions
                    .iter()
                    .map(|t| ListItem::new(format!("  {}", t)))
                    .collect();
                if items.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No next actions. Consider adding some!",
                            Style::default().fg(Color::Yellow),
                        )),
                    ])
                    .block(block);
                    frame.render_widget(paragraph, area);
                } else {
                    let list = List::new(items).block(block);
                    frame.render_widget(list, area);
                }
            }
            ReviewPhase::WaitingFor => {
                let items: Vec<ListItem> = self
                    .waiting_for
                    .iter()
                    .map(|t| ListItem::new(format!("  {}", t)))
                    .collect();
                if items.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Nothing in waiting-for.",
                            Style::default().fg(Color::Green),
                        )),
                    ])
                    .block(block);
                    frame.render_widget(paragraph, area);
                } else {
                    let list = List::new(items).block(block);
                    frame.render_widget(list, area);
                }
            }
            ReviewPhase::Projects => {
                if self.projects.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No projects. Create one with: gtd project new <name>",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(block);
                    frame.render_widget(paragraph, area);
                } else {
                    let items: Vec<ListItem> = self
                        .projects
                        .iter()
                        .map(|(name, count)| {
                            ListItem::new(format!("  {} ({} open tasks)", name, count))
                        })
                        .collect();
                    let list = List::new(items).block(block);
                    frame.render_widget(list, area);
                }
            }
            ReviewPhase::SomedayMaybe => {
                let text = format!("  {} items on the someday/maybe list.", self.someday_count);
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(text, Style::default().fg(Color::White))),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Use `gtd edit someday` to review and update the list.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(block);
                frame.render_widget(paragraph, area);
            }
            ReviewPhase::Summary => {
                let lines = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Weekly review complete!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("  Recommended next steps:"),
                    Line::from(Span::styled(
                        "    1. Process any remaining in-tray items: gtd process",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        "    2. Follow up on waiting-for items",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        "    3. Ensure every project has a next action",
                        Style::default().fg(Color::White),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press Enter to finish.",
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                let paragraph = Paragraph::new(lines).block(block);
                frame.render_widget(paragraph, area);
            }
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help = Line::from(vec![
            Span::styled(" Enter/Space", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" next phase  "),
            Span::styled("j/k", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" scroll  "),
            Span::styled("q/Esc", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" quit"),
        ]);
        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(help).block(block);
        frame.render_widget(paragraph, area);
    }
}

pub fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let mut app = ReviewApp::new()?;

    let result = run_tui(&mut terminal, &mut app);

    ratatui::restore();

    result?;

    println!(
        "\n{}",
        colored::Colorize::bold(colored::Colorize::green("Weekly review complete!"))
    );

    Ok(())
}

fn run_tui(terminal: &mut DefaultTerminal, app: &mut ReviewApp) -> Result<()> {
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if app.should_quit {
            break;
        }

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code);
        }
    }
    Ok(())
}
