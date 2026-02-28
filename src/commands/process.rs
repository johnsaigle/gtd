use anyhow::Result;
use colored::Colorize;
use dialoguer::{Input, Select};

use crate::gtd;
use crate::markdown::{self, Task};

pub fn run() -> Result<()> {
    let intray_path = gtd::in_tray_dir().join("inbox.md");
    let items = markdown::parse_tasks(&intray_path)?;

    if items.is_empty() {
        println!("{}", "In-tray is empty. Nothing to process!".green());
        return Ok(());
    }

    let descriptions: Vec<String> = items.iter().map(|t| t.description.clone()).collect();
    let project_names = gtd::list_projects()?;
    let total = descriptions.len();
    let mut processed: usize = 0;

    println!(
        "\n{} {} items in the in-tray\n",
        "Processing:".bold(),
        total
    );

    for (i, item) in descriptions.iter().enumerate() {
        println!(
            "\n{} [{}/{}] {}",
            "-->".cyan().bold(),
            i.saturating_add(1),
            total,
            item.bold()
        );

        let outcome = process_item(item, &project_names)?;
        match outcome {
            Outcome::Done => {
                processed = processed.saturating_add(1);
            }
            Outcome::Skip => {
                // Leave in in-tray
            }
            Outcome::Quit => {
                println!(
                    "\n{}",
                    "Stopped processing. Remaining items stay in in-tray.".yellow()
                );
                rewrite_intray(&intray_path, &descriptions, i)?;
                println!(
                    "{} Processed {} of {} items.",
                    "+".green().bold(),
                    processed,
                    total
                );
                return Ok(());
            }
        }
    }

    // All items processed — clear the in-tray
    std::fs::write(&intray_path, "# In-Tray\n")?;
    println!(
        "\n{} Processed all {} items. In-tray is empty!",
        "+".green().bold(),
        total
    );

    Ok(())
}

enum Outcome {
    Done,
    Skip,
    Quit,
}

fn process_item(item: &str, project_names: &[String]) -> Result<Outcome> {
    // Step 1: Is it actionable?
    let actionable = Select::new()
        .with_prompt("Is it actionable?")
        .items(&["Yes", "No", "Skip", "Quit"])
        .default(0)
        .interact()?;

    match actionable {
        1 => return handle_not_actionable(item),
        2 => return Ok(Outcome::Skip),
        3 => return Ok(Outcome::Quit),
        _ => {} // Yes — continue
    }

    // Step 2: Two-minute rule
    let two_min = Select::new()
        .with_prompt("Can you do it in under 2 minutes?")
        .items(&["Yes — do it now", "No — takes longer"])
        .default(0)
        .interact()?;

    if two_min == 0 {
        let mut task = Task::new(item.to_string());
        task.done = true;
        markdown::append_task(&gtd::archive_file(), &task)?;
        println!("  {} Done! (2-min rule) — archived", "x".green().bold());
        return Ok(Outcome::Done);
    }

    // Step 3: Delegate?
    let delegate = Select::new()
        .with_prompt("Should someone else do it?")
        .items(&["Yes — delegate it", "No — I'll do it"])
        .default(1)
        .interact()?;

    if delegate == 0 {
        let person: String = Input::new()
            .with_prompt("Delegated to")
            .default("(someone)".to_string())
            .interact_text()?;
        let mut task = Task::new(item.to_string());
        task.meta.delegated_to = Some(person.clone());
        markdown::append_task(&gtd::waiting_for_file(), &task)?;
        println!(
            "  {} Delegated to {} — added to waiting-for",
            "->".blue().bold(),
            person
        );
        return Ok(Outcome::Done);
    }

    // Step 4: Where does it go?
    let mut destinations = vec!["Next Actions".to_string()];
    for p in project_names {
        destinations.push(format!("Project: {p}"));
    }

    let dest = Select::new()
        .with_prompt("Add to which list?")
        .items(&destinations)
        .default(0)
        .interact()?;

    let task = Task::new(item.to_string());
    if dest == 0 {
        markdown::append_task(&gtd::tasks_file(), &task)?;
        println!("  {} Added to Next Actions", "+".green().bold());
    } else if let Some(proj) = project_names.get(dest.saturating_sub(1)) {
        markdown::append_task(&gtd::project_tasks_file(proj), &task)?;
        println!(
            "  {} Added to project '{}'",
            "+".green().bold(),
            proj.cyan()
        );
    }

    Ok(Outcome::Done)
}

fn handle_not_actionable(item: &str) -> Result<Outcome> {
    let choice = Select::new()
        .with_prompt("What to do with it?")
        .items(&["Trash it", "Someday/Maybe", "Reference (keep note)"])
        .default(0)
        .interact()?;

    match choice {
        0 => {
            println!("  {} Trashed", "x".red().bold());
        }
        1 => {
            let task = Task::new(item.to_string());
            markdown::append_task(&gtd::someday_maybe_file(), &task)?;
            println!("  {} Moved to Someday/Maybe", "->".yellow().bold());
        }
        2 => {
            println!("  {} Kept as reference note", "->".dimmed());
        }
        _ => {}
    }

    Ok(Outcome::Done)
}

fn rewrite_intray(
    path: &std::path::Path,
    all_items: &[String],
    processed_up_to: usize,
) -> Result<()> {
    let mut content = "# In-Tray\n".to_string();
    for item in all_items.iter().skip(processed_up_to) {
        content.push_str("- [ ] ");
        content.push_str(item);
        content.push('\n');
    }
    std::fs::write(path, content)?;
    Ok(())
}
