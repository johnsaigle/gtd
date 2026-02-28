use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Frontmatter metadata for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMeta {
    pub id: String,
    pub created: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegated_to: Option<String>,
    #[serde(default)]
    pub reminder: bool,
    #[serde(default)]
    pub in_calendar: bool,
}

impl TaskMeta {
    pub fn new() -> Self {
        let full_id = Uuid::new_v4().to_string();
        // UUID v4 is always ASCII hex + hyphens, so char_boundary is safe,
        // but we use get() for clippy compliance.
        let short_id = full_id.get(..8).unwrap_or(&full_id).to_string();
        Self {
            id: short_id,
            created: chrono::Local::now().date_naive(),
            due: None,
            delegated_to: None,
            reminder: false,
            in_calendar: false,
        }
    }
}

/// A single GTD task: frontmatter + description + done state.
#[derive(Debug, Clone)]
pub struct Task {
    pub meta: TaskMeta,
    pub description: String,
    pub done: bool,
}

impl Task {
    pub fn new(description: String) -> Self {
        Self {
            meta: TaskMeta::new(),
            description,
            done: false,
        }
    }

    /// Serialize to the markdown block format:
    /// ```
    /// <!-- id:abc123 -->
    /// ---
    /// created: 2026-02-23
    /// ...
    /// ---
    /// - [ ] Description
    /// ```
    pub fn to_markdown(&self) -> String {
        let checkbox = if self.done { "[x]" } else { "[ ]" };
        let yaml = serde_yaml::to_string(&self.meta).unwrap_or_default();
        format!(
            "<!-- id:{} -->\n---\n{}---\n- {} {}\n",
            self.meta.id, yaml, checkbox, self.description
        )
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let checkbox = if self.done { "[x]" } else { "[ ]" };
        let due = self
            .meta
            .due
            .map(|d| format!(" (due: {d})"))
            .unwrap_or_default();
        let delegated = self
            .meta
            .delegated_to
            .as_ref()
            .map(|d| format!(" [-> {d}]"))
            .unwrap_or_default();
        write!(
            f,
            "- {} {}{}{}  ({})",
            checkbox, self.description, due, delegated, self.meta.id
        )
    }
}

/// Parse all tasks from a GTD markdown file.
pub fn parse_tasks(path: &Path) -> Result<Vec<Task>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(parse_tasks_from_str(&content))
}

/// Parse tasks from markdown string content.
pub fn parse_tasks_from_str(content: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        // Look for task ID comment
        if let Some(id) = line
            .strip_prefix("<!-- id:")
            .and_then(|s| s.strip_suffix(" -->"))
        {
            let id = id.to_string();

            // Expect --- frontmatter start
            if lines.peek().is_some_and(|l| l.trim() == "---") {
                lines.next(); // consume ---
                let mut yaml_lines = Vec::new();
                for line in lines.by_ref() {
                    if line.trim() == "---" {
                        break;
                    }
                    yaml_lines.push(line);
                }
                let yaml_str = yaml_lines.join("\n");
                let mut meta: TaskMeta =
                    serde_yaml::from_str(&yaml_str).unwrap_or_else(|_| TaskMeta::new());
                meta.id = id;

                // Next line should be the checkbox line
                if let Some(task_line) = lines.next() {
                    let (done, desc) = parse_checkbox_line(task_line);
                    tasks.push(Task {
                        meta,
                        description: desc,
                        done,
                    });
                }
            }
        } else if line.starts_with("- [ ] ") || line.starts_with("- [x] ") {
            // Also handle bare checkbox lines (no frontmatter) for in-tray items
            let (done, desc) = parse_checkbox_line(line);
            let mut task = Task::new(desc);
            task.done = done;
            tasks.push(task);
        }
    }

    tasks
}

fn parse_checkbox_line(line: &str) -> (bool, String) {
    let trimmed = line.trim();
    trimmed.strip_prefix("- [x] ").map_or_else(
        || {
            trimmed.strip_prefix("- [ ] ").map_or_else(
                || (false, trimmed.to_string()),
                |desc| (false, desc.to_string()),
            )
        },
        |desc| (true, desc.to_string()),
    )
}

/// Write tasks back to a file, preserving the header line.
pub fn write_tasks(path: &Path, tasks: &[Task]) -> Result<()> {
    let header = read_header(path);
    let mut content = header;
    content.push('\n');
    for task in tasks {
        content.push_str(&task.to_markdown());
        content.push('\n');
    }
    fs::write(path, &content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Append a single task to a file.
pub fn append_task(path: &Path, task: &Task) -> Result<()> {
    let mut content = fs::read_to_string(path).unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&task.to_markdown());
    fs::write(path, &content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Read the first line (header) of a file, or return a default.
fn read_header(path: &Path) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|c| c.lines().next().map(std::string::ToString::to_string))
        .unwrap_or_else(|| "# Tasks".to_string())
}

/// Write a simple in-tray item (no frontmatter, just a checkbox line).
pub fn append_intray_item(path: &Path, description: &str) -> Result<()> {
    use std::fmt::Write;
    let mut content = fs::read_to_string(path).unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let _ = writeln!(content, "- [ ] {description}");
    fs::write(path, &content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Remove a task by ID from a file. Returns the removed task if found.
pub fn remove_task_by_id(path: &Path, id: &str) -> Result<Option<Task>> {
    let mut tasks = parse_tasks(path)?;
    let pos = tasks.iter().position(|t| t.meta.id == id);
    let removed = pos.map(|i| tasks.remove(i));
    write_tasks(path, &tasks)?;
    Ok(removed)
}
