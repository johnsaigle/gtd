use anyhow::{Result, bail};
use colored::Colorize;
use std::fs;

use crate::commands::ProjectAction;
use crate::gtd;
use crate::markdown;

pub fn run(action: ProjectAction) -> Result<()> {
    match action {
        ProjectAction::New { name } => create_project(&name),
        ProjectAction::List => list_projects(),
        ProjectAction::Delete { name } => delete_project(&name),
    }
}

fn create_project(name: &str) -> Result<()> {
    let dir = gtd::project_dir(name);
    if dir.exists() {
        bail!("Project '{}' already exists", name);
    }

    fs::create_dir_all(gtd::project_reference_dir(name))?;
    fs::write(
        gtd::project_tasks_file(name),
        format!("# Project: {}\n", name),
    )?;

    println!(
        "{} Created project: {}",
        "+".green().bold(),
        name.cyan().bold()
    );
    println!(
        "  {}",
        gtd::project_dir(name).display().to_string().dimmed()
    );
    Ok(())
}

fn list_projects() -> Result<()> {
    let projects = gtd::list_projects()?;
    if projects.is_empty() {
        println!(
            "{}",
            "No projects yet. Create one with: gtd project new <name>".dimmed()
        );
        return Ok(());
    }

    println!("\n{}", "Projects".bold().underline());
    for proj in &projects {
        let path = gtd::project_tasks_file(proj);
        let tasks = markdown::parse_tasks(&path)?;
        let total = tasks.len();
        let open = tasks.iter().filter(|t| !t.done).count();
        let done = total.saturating_sub(open);

        let ref_dir = gtd::project_reference_dir(proj);
        let ref_count = if ref_dir.exists() {
            fs::read_dir(&ref_dir)?.count()
        } else {
            0
        };

        println!(
            "  {} {} - {} open, {} done, {} reference files",
            "->".dimmed(),
            proj.cyan().bold(),
            open.to_string().yellow(),
            done.to_string().green(),
            ref_count
        );
    }
    println!();
    Ok(())
}

fn delete_project(name: &str) -> Result<()> {
    let dir = gtd::project_dir(name);
    if !dir.exists() {
        bail!("Project '{}' does not exist", name);
    }

    // Archive remaining tasks
    let tasks_path = gtd::project_tasks_file(name);
    let tasks = markdown::parse_tasks(&tasks_path)?;
    let open_tasks: Vec<_> = tasks.iter().filter(|t| !t.done).collect();

    if !open_tasks.is_empty() {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Project '{}' has {} open tasks. Archive and delete?",
                name,
                open_tasks.len()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        }

        // Archive open tasks
        for task in &open_tasks {
            markdown::append_task(&gtd::archive_file(), task)?;
        }
        println!("  {} {} tasks archived", "->".dimmed(), open_tasks.len());
    }

    fs::remove_dir_all(&dir)?;
    println!("{} Deleted project: {}", "x".red().bold(), name.cyan());
    Ok(())
}
