use anyhow::Result;
use colored::Colorize;
use dialoguer::Confirm;

use crate::gtd;
use crate::markdown::{self, Task};

struct ReviewData {
    intray_tasks: Vec<Task>,
    next_open: Vec<Task>,
    waiting_open: Vec<Task>,
    someday_open: Vec<Task>,
    project_names: Vec<String>,
}

impl ReviewData {
    fn gather() -> Result<Self> {
        let intray_path = gtd::in_tray_dir().join("inbox.md");
        let intray_tasks = markdown::parse_tasks(&intray_path)?;
        let next = markdown::parse_tasks(&gtd::tasks_file())?;
        let waiting = markdown::parse_tasks(&gtd::waiting_for_file())?;
        let someday = markdown::parse_tasks(&gtd::someday_maybe_file())?;
        let project_names = gtd::list_projects()?;

        Ok(Self {
            intray_tasks,
            next_open: next.into_iter().filter(|t| !t.done).collect(),
            waiting_open: waiting.into_iter().filter(|t| !t.done).collect(),
            someday_open: someday.into_iter().filter(|t| !t.done).collect(),
            project_names,
        })
    }
}

pub fn run() -> Result<()> {
    println!("\n{}", "=== Weekly Review ===".bold().cyan());
    println!(
        "{}",
        "Walk through each GTD list. Update what's changed, capture anything new.\n".dimmed()
    );

    let data = ReviewData::gather()?;

    print_overview(&data);
    review_intray(&data)?;
    review_next_actions(&data)?;
    review_waiting_for(&data)?;
    review_projects(&data)?;
    review_someday_maybe(&data)?;
    print_summary(&data)?;

    Ok(())
}

fn print_overview(data: &ReviewData) {
    println!("{}", "Current state:".bold());
    println!(
        "  In-Tray:       {}",
        format_count(data.intray_tasks.len(), data.intray_tasks.is_empty())
    );
    println!(
        "  Next Actions:  {}",
        format_count(data.next_open.len(), false)
    );
    println!(
        "  Waiting For:   {}",
        format_count(data.waiting_open.len(), false)
    );
    println!("  Projects:      {}", data.project_names.len());
    println!(
        "  Someday/Maybe: {}",
        format_count(data.someday_open.len(), false)
    );
    println!();
}

fn review_intray(data: &ReviewData) -> Result<()> {
    println!("{}", "--- 1. In-Tray ---".bold().yellow());
    if data.intray_tasks.is_empty() {
        println!("  {} Inbox zero!", "OK".green().bold());
    } else {
        let count = data.intray_tasks.len();
        println!("  {} {count} items need processing.", "!!".red().bold());
        println!("  Run `gtd process` after this review to clear them.");
    }
    wait_for_continue()
}

fn review_next_actions(data: &ReviewData) -> Result<()> {
    println!("\n{}", "--- 2. Next Actions ---".bold().yellow());
    if data.next_open.is_empty() {
        println!("  {}", "(empty -- consider adding some!)".dimmed());
    } else {
        println!("  Review each action. Is it still relevant? Still the right next step?\n");
        for task in &data.next_open {
            println!("  {task}");
        }
    }
    println!();
    println!("  {}", "Edit with: gtd edit tasks".dimmed());
    wait_for_continue()
}

fn review_waiting_for(data: &ReviewData) -> Result<()> {
    println!("\n{}", "--- 3. Waiting For ---".bold().yellow());
    if data.waiting_open.is_empty() {
        println!("  {}", "(nothing pending)".dimmed());
    } else {
        println!("  Check on each item. Follow up if needed. Remove anything received.\n");
        for task in &data.waiting_open {
            println!("  {task}");
        }
    }
    println!();
    println!("  {}", "Edit with: gtd edit waiting".dimmed());
    wait_for_continue()
}

fn review_projects(data: &ReviewData) -> Result<()> {
    println!("\n{}", "--- 4. Projects ---".bold().yellow());
    if data.project_names.is_empty() {
        println!("  {}", "(no projects)".dimmed());
    } else {
        println!("  Does each project have a clear next action? Still active?\n");
        for name in &data.project_names {
            let tasks = markdown::parse_tasks(&gtd::project_tasks_file(name))?;
            let open = tasks.iter().filter(|t| !t.done).count();
            let status = if open == 0 {
                "no next action!".red().bold().to_string()
            } else {
                format!("{open} open")
            };
            println!("  {} {} ({status})", "->".dimmed(), name.cyan());
        }
    }
    println!();
    wait_for_continue()
}

fn review_someday_maybe(data: &ReviewData) -> Result<()> {
    println!("\n{}", "--- 5. Someday/Maybe ---".bold().yellow());
    if data.someday_open.is_empty() {
        println!("  {}", "(empty)".dimmed());
    } else {
        println!("  Anything ready to activate? Anything to remove?\n");
        for task in &data.someday_open {
            println!("  {task}");
        }
    }
    println!();
    println!("  {}", "Edit with: gtd edit someday".dimmed());
    wait_for_continue()
}

fn print_summary(data: &ReviewData) -> Result<()> {
    println!("\n{}", "=== Review Complete ===".bold().green());
    println!("  Recommended next steps:");
    if !data.intray_tasks.is_empty() {
        println!("    1. Process in-tray: {}", "gtd process".cyan());
    }
    if !data.waiting_open.is_empty() {
        println!("    2. Follow up on waiting-for items");
    }
    for name in &data.project_names {
        let tasks = markdown::parse_tasks(&gtd::project_tasks_file(name))?;
        let open = tasks.iter().filter(|t| !t.done).count();
        if open == 0 {
            println!("    - Add a next action to project '{}'", name.cyan());
        }
    }
    println!();
    Ok(())
}

fn format_count(count: usize, is_good_when_zero: bool) -> String {
    if count == 0 && is_good_when_zero {
        "0 (inbox zero!)".green().to_string()
    } else if count == 0 {
        "0".to_string()
    } else {
        count.to_string().yellow().bold().to_string()
    }
}

fn wait_for_continue() -> Result<()> {
    Confirm::new()
        .with_prompt("Continue to next phase?")
        .default(true)
        .interact()?;
    Ok(())
}
