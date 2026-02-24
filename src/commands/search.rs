use anyhow::Result;
use colored::Colorize;
use regex::RegexBuilder;
use walkdir::WalkDir;

use crate::gtd;

pub fn run(query: Vec<String>) -> Result<()> {
    let query_str = query.join(" ");
    if query_str.trim().is_empty() {
        println!("{}", "Provide a search query.".yellow());
        return Ok(());
    }

    let pattern = RegexBuilder::new(&regex::escape(&query_str))
        .case_insensitive(true)
        .build()?;

    let root = gtd::root();
    let mut found: usize = 0;

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
    {
        let path = entry.path();
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let relative = path.strip_prefix(&root).unwrap_or(path);
        let mut file_printed = false;

        for (i, line) in content.lines().enumerate() {
            if pattern.is_match(line) {
                if !file_printed {
                    println!("\n{}", relative.display().to_string().cyan().bold());
                    file_printed = true;
                }
                let line_num = i.saturating_add(1);
                println!(
                    "  {}: {}",
                    line_num.to_string().dimmed(),
                    highlight_match(line, &query_str)
                );
                found = found.saturating_add(1);
            }
        }
    }

    if found == 0 {
        println!("{} No results for '{}'", "!".yellow(), query_str);
    } else {
        println!(
            "\n{} {} matches found",
            "->".green(),
            found.to_string().bold()
        );
    }

    Ok(())
}

fn highlight_match(line: &str, query: &str) -> String {
    let lower_line = line.to_lowercase();
    let lower_query = query.to_lowercase();

    if let Some(pos) = lower_line.find(&lower_query) {
        let end = pos.saturating_add(query.len());
        match (line.get(..pos), line.get(pos..end), line.get(end..)) {
            (Some(before), Some(matched), Some(after)) => {
                format!("{}{}{}", before, matched.red().bold(), after)
            }
            _ => line.to_string(),
        }
    } else {
        line.to_string()
    }
}
