use anyhow::{Result, bail};
use chrono::NaiveDate;
use colored::Colorize;

use crate::gtd;
use crate::markdown::{self, Task};

pub fn run(
    description: &[String],
    waiting: bool,
    someday: bool,
    project: Option<&String>,
    due: Option<&String>,
    delegated_to: Option<&String>,
) -> Result<()> {
    let desc = if description.is_empty() {
        dialoguer::Input::<String>::new()
            .with_prompt("Task description")
            .interact_text()?
    } else {
        description.join(" ")
    };

    if desc.trim().is_empty() {
        bail!("Task description cannot be empty");
    }

    let mut task = Task::new(desc.trim().to_string());

    // Parse due date
    if let Some(due_str) = &due {
        task.meta.due = Some(
            NaiveDate::parse_from_str(due_str, "%Y-%m-%d")
                .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?,
        );
    }

    // Set delegated_to
    if let Some(person) = delegated_to {
        task.meta.delegated_to = Some((*person).clone());
    }

    // Determine target file
    let (target_path, list_name) = if let Some(proj) = project {
        let path = gtd::project_tasks_file(proj);
        if !path.exists() {
            bail!("Project '{proj}' does not exist. Create it first with: gtd project new {proj}");
        }
        (path, format!("project:{proj}"))
    } else if waiting {
        (gtd::waiting_for_file(), "waiting-for".to_string())
    } else if someday {
        (gtd::someday_maybe_file(), "someday/maybe".to_string())
    } else {
        (gtd::tasks_file(), "next-actions".to_string())
    };

    markdown::append_task(&target_path, &task)?;

    println!(
        "{} Added to {}: {} {}",
        "+".green().bold(),
        list_name.cyan(),
        task.description,
        format!("({})", task.meta.id).dimmed()
    );

    if let Some(d) = task.meta.due {
        println!("  {} {}", "Due:".dimmed(), d);
    }
    if let Some(ref person) = task.meta.delegated_to {
        println!("  {} {}", "Delegated to:".dimmed(), person);
    }

    Ok(())
}
