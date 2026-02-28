use anyhow::{Result, bail};
use colored::Colorize;

use crate::gtd;
use crate::markdown;

pub fn run(id: &str) -> Result<()> {
    // Search across all lists for the task ID
    let search_files = collect_all_task_files()?;

    for path in &search_files {
        if let Some(mut task) = markdown::remove_task_by_id(path, id)? {
            task.done = true;

            // Append to archive
            markdown::append_task(&gtd::archive_file(), &task)?;

            println!(
                "{} Completed: {} {}",
                "x".green().bold(),
                task.description,
                format!("({})", task.meta.id).dimmed()
            );
            println!(
                "  {} {}",
                "Archived to:".dimmed(),
                gtd::archive_file().display()
            );
            return Ok(());
        }
    }

    bail!("Task with ID '{id}' not found. Use `gtd list all` to see task IDs.");
}

fn collect_all_task_files() -> Result<Vec<std::path::PathBuf>> {
    let mut files = vec![
        gtd::tasks_file(),
        gtd::waiting_for_file(),
        gtd::someday_maybe_file(),
    ];

    // Add all project task files
    for project in gtd::list_projects()? {
        files.push(gtd::project_tasks_file(&project));
    }

    Ok(files)
}
