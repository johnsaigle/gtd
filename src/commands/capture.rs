use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::gtd;
use crate::markdown;

pub fn run(description: &[String]) -> Result<()> {
    let desc = if description.is_empty() {
        // Interactive prompt
        dialoguer::Input::<String>::new()
            .with_prompt("Capture")
            .interact_text()?
    } else {
        description.join(" ")
    };

    if desc.trim().is_empty() {
        println!("{}", "Nothing to capture.".yellow());
        return Ok(());
    }

    let intray = gtd::in_tray_dir().join("inbox.md");
    if !intray.exists() {
        fs::write(&intray, "# In-Tray\n")?;
    }

    markdown::append_intray_item(&intray, desc.trim())?;
    println!(
        "{} Captured to in-tray: {}",
        "->".green().bold(),
        desc.trim()
    );
    Ok(())
}
