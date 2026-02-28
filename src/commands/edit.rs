use anyhow::{Result, bail};
use std::process::Command;

use crate::gtd;

pub fn run(target: &str) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    let path = match target {
        "tasks" | "next" => gtd::tasks_file(),
        "waiting" | "waiting-for" => gtd::waiting_for_file(),
        "someday" | "someday-maybe" => gtd::someday_maybe_file(),
        "intray" | "inbox" => gtd::in_tray_dir().join("inbox.md"),
        "archive" => gtd::archive_file(),
        t if t.starts_with("project:") => {
            let name = t.strip_prefix("project:").unwrap_or_default();
            let path = gtd::project_tasks_file(name);
            if !path.exists() {
                bail!("Project '{name}' does not exist");
            }
            path
        }
        _ => {
            // Try as project name
            let path = gtd::project_tasks_file(target);
            if path.exists() {
                path
            } else {
                bail!(
                    "Unknown target '{target}'. Options: tasks, waiting, someday, intray, archive, project:<name>"
                );
            }
        }
    };

    Command::new(&editor).arg(&path).status()?;

    Ok(())
}
