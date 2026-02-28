use anyhow::Result;
use colored::Colorize;

use crate::gtd;
use crate::markdown;

pub fn run(filter: &str, show_all: bool) -> Result<()> {
    match filter {
        "next" | "tasks" => show_file("Next Actions", &gtd::tasks_file(), show_all),
        "waiting" | "waiting-for" => show_file("Waiting For", &gtd::waiting_for_file(), show_all),
        "someday" | "someday-maybe" => {
            show_file("Someday / Maybe", &gtd::someday_maybe_file(), show_all)
        }
        "archive" => show_file("Archive", &gtd::archive_file(), true),
        "intray" | "inbox" => show_intray(),
        "all" => show_all_lists(show_all),
        f if f.starts_with("project:") => {
            let name = f.strip_prefix("project:").unwrap_or_default();
            let path = gtd::project_tasks_file(name);
            if !path.exists() {
                println!("{} Project '{}' not found.", "!".red().bold(), name);
                return Ok(());
            }
            show_file(&format!("Project: {name}"), &path, show_all)
        }
        _ => {
            // Try as project name
            let path = gtd::project_tasks_file(filter);
            if path.exists() {
                show_file(&format!("Project: {filter}"), &path, show_all)
            } else {
                println!(
                    "{} Unknown filter '{}'. Options: next, waiting, someday, intray, archive, all, project:<name>",
                    "!".red().bold(),
                    filter
                );
                Ok(())
            }
        }
    }
}

fn show_file(title: &str, path: &std::path::Path, show_all: bool) -> Result<()> {
    let tasks = markdown::parse_tasks(path)?;
    let filtered: Vec<_> = if show_all {
        tasks.iter().collect()
    } else {
        tasks.iter().filter(|t| !t.done).collect()
    };

    println!("\n{}", title.bold().underline());
    if filtered.is_empty() {
        println!("  {}", "(empty)".dimmed());
    } else {
        for task in &filtered {
            print_task(task);
        }
    }
    println!();
    Ok(())
}

fn show_intray() -> Result<()> {
    let intray_path = gtd::in_tray_dir().join("inbox.md");
    let tasks = markdown::parse_tasks(&intray_path)?;

    println!("\n{}", "In-Tray".bold().underline());
    if tasks.is_empty() {
        println!("  {}", "(empty - inbox zero!)".dimmed());
    } else {
        println!(
            "  {} items to process\n",
            tasks.len().to_string().yellow().bold()
        );
        for task in &tasks {
            println!("  - {}", task.description);
        }
    }
    println!();
    Ok(())
}

fn show_all_lists(show_all: bool) -> Result<()> {
    show_intray()?;
    show_file("Next Actions", &gtd::tasks_file(), show_all)?;
    show_file("Waiting For", &gtd::waiting_for_file(), show_all)?;
    show_file("Someday / Maybe", &gtd::someday_maybe_file(), show_all)?;

    // Show all projects
    let projects = gtd::list_projects()?;
    if !projects.is_empty() {
        println!("{}", "Projects".bold().underline());
        for proj in &projects {
            let path = gtd::project_tasks_file(proj);
            let tasks = markdown::parse_tasks(&path)?;
            let open = tasks.iter().filter(|t| !t.done).count();
            println!("  {} {} ({} open tasks)", "->".dimmed(), proj.cyan(), open);
        }
        println!();
    }

    Ok(())
}

fn print_task(task: &markdown::Task) {
    let checkbox = if task.done {
        "[x]".green().to_string()
    } else {
        "[ ]".to_string()
    };

    let mut line = format!(
        "  {} {} {}",
        checkbox,
        task.description,
        format!("({})", task.meta.id).dimmed()
    );

    if let Some(ref due) = task.meta.due {
        let today = chrono::Local::now().date_naive();
        let due_str = match (*due).cmp(&today) {
            std::cmp::Ordering::Less => format!(" OVERDUE:{due}").red().bold().to_string(),
            std::cmp::Ordering::Equal => " due:TODAY".to_string().yellow().bold().to_string(),
            std::cmp::Ordering::Greater => format!(" due:{due}").dimmed().to_string(),
        };
        line.push_str(&due_str);
    }

    if let Some(ref person) = task.meta.delegated_to {
        use std::fmt::Write;
        let _ = write!(line, " {}", format!("[-> {person}]").blue());
    }

    if task.meta.reminder {
        line.push_str(&" [R]".magenta().to_string());
    }
    if task.meta.in_calendar {
        line.push_str(&" [C]".cyan().to_string());
    }

    println!("{line}");
}
