use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::gtd;
use crate::markdown::{self, Task};

/// GTD decision tree states
#[derive(Debug, Clone)]
enum ProcessState {
    /// Show the item, ask "Is it actionable?"
    Actionable,
    /// Not actionable: trash, reference, or someday/maybe?
    NotActionable,
    /// Actionable: can you do it in under 2 minutes?
    TwoMinRule,
    /// Takes longer: are you the right person?
    Delegate,
    /// You'll do it: does it belong to a project?
    Defer,
    /// Choose destination list
    ChooseList,
    /// Done processing this item
    Done(String),
}

struct ProcessApp {
    items: Vec<String>,
    current: usize,
    state: ProcessState,
    should_quit: bool,
    project_names: Vec<String>,
    selected_option: usize,
}

impl ProcessApp {
    fn new(items: Vec<String>, project_names: Vec<String>) -> Self {
        Self {
            items,
            current: 0,
            state: ProcessState::Actionable,
            should_quit: false,
            project_names,
            selected_option: 0,
        }
    }

    fn current_item(&self) -> Option<&str> {
        self.items.get(self.current).map(|s| s.as_str())
    }

    fn options(&self) -> Vec<(&str, &str)> {
        match &self.state {
            ProcessState::Actionable => vec![
                ("y", "Yes, it's actionable"),
                ("n", "No, not actionable"),
                ("s", "Skip for now"),
            ],
            ProcessState::NotActionable => vec![
                ("t", "Trash it"),
                ("r", "Reference material (keep in project)"),
                ("s", "Someday/Maybe"),
            ],
            ProcessState::TwoMinRule => vec![
                ("y", "Yes, under 2 minutes - do it now!"),
                ("n", "No, it'll take longer"),
            ],
            ProcessState::Delegate => vec![
                ("y", "Yes, delegate it (waiting-for)"),
                ("n", "No, I'll do it myself"),
            ],
            ProcessState::Defer => vec![
                ("n", "Next Actions (no project)"),
                ("p", "Add to a project"),
            ],
            ProcessState::ChooseList => {
                // Dynamic: list projects
                vec![] // handled specially in render
            }
            ProcessState::Done(_) => vec![("Enter", "Continue")],
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return Ok(());
            }
            _ => {}
        }

        // Handle option selection with arrow keys
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_option = self.selected_option.saturating_sub(1);
                return Ok(());
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = match &self.state {
                    ProcessState::ChooseList => self.project_names.len().saturating_sub(1),
                    _ => self.options().len().saturating_sub(1),
                };
                if self.selected_option < max {
                    self.selected_option = self.selected_option.saturating_add(1);
                }
                return Ok(());
            }
            _ => {}
        }

        let item = match self.current_item() {
            Some(i) => i.to_string(),
            None => {
                self.should_quit = true;
                return Ok(());
            }
        };

        match &self.state {
            ProcessState::Actionable => match key {
                KeyCode::Char('y') | KeyCode::Enter if self.selected_option == 0 => {
                    self.state = ProcessState::TwoMinRule;
                    self.selected_option = 0;
                }
                KeyCode::Char('n') | KeyCode::Enter if self.selected_option == 1 => {
                    self.state = ProcessState::NotActionable;
                    self.selected_option = 0;
                }
                KeyCode::Char('s') | KeyCode::Enter if self.selected_option == 2 => {
                    self.advance();
                }
                KeyCode::Enter => {
                    // Handle enter on selected option
                    match self.selected_option {
                        0 => {
                            self.state = ProcessState::TwoMinRule;
                            self.selected_option = 0;
                        }
                        1 => {
                            self.state = ProcessState::NotActionable;
                            self.selected_option = 0;
                        }
                        2 => self.advance(),
                        _ => {}
                    }
                }
                _ => {}
            },
            ProcessState::NotActionable => {
                let action = match key {
                    KeyCode::Char('t') => Some(0),
                    KeyCode::Char('r') => Some(1),
                    KeyCode::Char('s') => Some(2),
                    KeyCode::Enter => Some(self.selected_option),
                    _ => None,
                };
                if let Some(idx) = action {
                    match idx {
                        0 => {
                            // Trash - just remove from in-tray
                            self.state = ProcessState::Done(format!("Trashed: {}", item));
                            self.selected_option = 0;
                        }
                        1 => {
                            // Reference - keep note but don't make it a task
                            self.state = ProcessState::Done(format!("Kept as reference: {}", item));
                            self.selected_option = 0;
                        }
                        2 => {
                            // Someday/Maybe
                            let task = Task::new(item.clone());
                            markdown::append_task(&gtd::someday_maybe_file(), &task)?;
                            self.state =
                                ProcessState::Done(format!("Moved to Someday/Maybe: {}", item));
                            self.selected_option = 0;
                        }
                        _ => {}
                    }
                }
            }
            ProcessState::TwoMinRule => {
                let action = match key {
                    KeyCode::Char('y') => Some(0),
                    KeyCode::Char('n') => Some(1),
                    KeyCode::Enter => Some(self.selected_option),
                    _ => None,
                };
                if let Some(idx) = action {
                    match idx {
                        0 => {
                            // Do it now! Mark as done immediately
                            let mut task = Task::new(item.clone());
                            task.done = true;
                            markdown::append_task(&gtd::archive_file(), &task)?;
                            self.state =
                                ProcessState::Done(format!("Done! (2-min rule): {}", item));
                            self.selected_option = 0;
                        }
                        1 => {
                            self.state = ProcessState::Delegate;
                            self.selected_option = 0;
                        }
                        _ => {}
                    }
                }
            }
            ProcessState::Delegate => {
                let action = match key {
                    KeyCode::Char('y') => Some(0),
                    KeyCode::Char('n') => Some(1),
                    KeyCode::Enter => Some(self.selected_option),
                    _ => None,
                };
                if let Some(idx) = action {
                    match idx {
                        0 => {
                            // Delegate - add to waiting-for
                            let mut task = Task::new(item.clone());
                            task.meta.delegated_to = Some("(someone)".to_string());
                            markdown::append_task(&gtd::waiting_for_file(), &task)?;
                            self.state =
                                ProcessState::Done(format!("Delegated (waiting-for): {}", item));
                            self.selected_option = 0;
                        }
                        1 => {
                            self.state = ProcessState::Defer;
                            self.selected_option = 0;
                        }
                        _ => {}
                    }
                }
            }
            ProcessState::Defer => {
                let action = match key {
                    KeyCode::Char('n') => Some(0),
                    KeyCode::Char('p') => Some(1),
                    KeyCode::Enter => Some(self.selected_option),
                    _ => None,
                };
                if let Some(idx) = action {
                    match idx {
                        0 => {
                            // Next Actions
                            let task = Task::new(item.clone());
                            markdown::append_task(&gtd::tasks_file(), &task)?;
                            self.state =
                                ProcessState::Done(format!("Added to Next Actions: {}", item));
                            self.selected_option = 0;
                        }
                        1 => {
                            if self.project_names.is_empty() {
                                // No projects, just add to next actions
                                let task = Task::new(item.clone());
                                markdown::append_task(&gtd::tasks_file(), &task)?;
                                self.state = ProcessState::Done(format!(
                                    "Added to Next Actions (no projects exist): {}",
                                    item
                                ));
                                self.selected_option = 0;
                            } else {
                                self.state = ProcessState::ChooseList;
                                self.selected_option = 0;
                            }
                        }
                        _ => {}
                    }
                }
            }
            ProcessState::ChooseList => {
                if let Some(proj) = (key == KeyCode::Enter)
                    .then(|| self.project_names.get(self.selected_option))
                    .flatten()
                {
                    let proj = proj.clone();
                    let task = Task::new(item.clone());
                    markdown::append_task(&gtd::project_tasks_file(&proj), &task)?;
                    self.state =
                        ProcessState::Done(format!("Added to project '{}': {}", proj, item));
                    self.selected_option = 0;
                }
            }
            ProcessState::Done(_) => {
                if key == KeyCode::Enter {
                    self.advance();
                }
            }
        }

        Ok(())
    }

    fn advance(&mut self) {
        self.current = self.current.saturating_add(1);
        self.state = ProcessState::Actionable;
        self.selected_option = 0;
        if self.current >= self.items.len() {
            self.should_quit = true;
        }
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let layout = Layout::vertical([
            Constraint::Length(3), // header
            Constraint::Length(5), // current item
            Constraint::Min(10),   // decision area
            Constraint::Length(3), // footer
        ])
        .split(area);

        if let (Some(header), Some(item), Some(decision), Some(footer)) =
            (layout.first(), layout.get(1), layout.get(2), layout.get(3))
        {
            self.render_header(frame, *header);
            self.render_item(frame, *item);
            self.render_decision(frame, *decision);
            self.render_footer(frame, *footer);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let progress = format!(
            " Processing In-Tray [{}/{}] ",
            self.current.saturating_add(1),
            self.items.len()
        );
        let block = Block::default()
            .title(progress)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(block, area);
    }

    fn render_item(&self, frame: &mut Frame, area: Rect) {
        let item_text = self.current_item().unwrap_or("(no more items)");
        let block = Block::default()
            .title(" Current Item ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        let paragraph = Paragraph::new(format!("  {}", item_text))
            .block(block)
            .wrap(Wrap { trim: false })
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(paragraph, area);
    }

    fn render_decision(&self, frame: &mut Frame, area: Rect) {
        let title = match &self.state {
            ProcessState::Actionable => " Is it actionable? ",
            ProcessState::NotActionable => " Not actionable - what to do? ",
            ProcessState::TwoMinRule => " Can you do it in under 2 minutes? ",
            ProcessState::Delegate => " Should someone else do it? ",
            ProcessState::Defer => " Where does it go? ",
            ProcessState::ChooseList => " Choose a project: ",
            ProcessState::Done(_) => " Result ",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));

        if let ProcessState::Done(msg) = &self.state {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", msg),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to continue...",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        if let ProcessState::ChooseList = &self.state {
            let mut lines = vec![Line::from("")];
            for (i, proj) in self.project_names.iter().enumerate() {
                let style = if i == self.selected_option {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if i == self.selected_option {
                    " > "
                } else {
                    "   "
                };
                lines.push(Line::from(Span::styled(
                    format!("{}  {}", prefix, proj),
                    style,
                )));
            }
            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        let options = self.options();
        let mut lines = vec![Line::from("")];
        for (i, (key, desc)) in options.iter().enumerate() {
            let is_selected = i == self.selected_option;
            let prefix = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("{}[{}] {}", prefix, key, desc),
                style,
            )));
        }
        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help = Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" select  "),
            Span::styled("q/Esc", Style::default().fg(Color::Yellow).bold()),
            Span::raw(" quit"),
        ]);
        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(help).block(block);
        frame.render_widget(paragraph, area);
    }
}

pub fn run() -> Result<()> {
    let intray_path = gtd::in_tray_dir().join("inbox.md");
    let items = markdown::parse_tasks(&intray_path)?;

    if items.is_empty() {
        println!(
            "{}",
            colored::Colorize::green("In-tray is empty. Nothing to process!")
        );
        return Ok(());
    }

    let descriptions: Vec<String> = items.iter().map(|t| t.description.clone()).collect();
    let project_names = gtd::list_projects()?;

    // Run TUI
    let mut terminal = ratatui::init();
    let mut app = ProcessApp::new(descriptions, project_names);

    let result = run_tui(&mut terminal, &mut app);

    ratatui::restore();

    result?;

    // Clear the in-tray after processing
    // Rewrite with only unprocessed items (those that were skipped)
    let remaining: Vec<String> = app.items.iter().skip(app.current).cloned().collect();

    let mut content = "# In-Tray\n".to_string();
    for item in &remaining {
        content.push_str(&format!("- [ ] {}\n", item));
    }
    std::fs::write(&intray_path, content)?;

    let processed = app.current;
    if processed > 0 {
        println!(
            "\n{} Processed {} items from in-tray.",
            colored::Colorize::bold(colored::Colorize::green("+")),
            processed
        );
    }

    Ok(())
}

fn run_tui(terminal: &mut DefaultTerminal, app: &mut ProcessApp) -> Result<()> {
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if app.should_quit {
            break;
        }

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code)?;
        }
    }
    Ok(())
}
