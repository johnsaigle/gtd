use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Root GTD data directory: ~/.local/share/gtd/
pub fn root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("gtd")
}

pub fn tasks_file() -> PathBuf {
    root().join("tasks.md")
}

pub fn waiting_for_file() -> PathBuf {
    root().join("waiting-for.md")
}

pub fn someday_maybe_file() -> PathBuf {
    root().join("someday-maybe.md")
}

pub fn in_tray_dir() -> PathBuf {
    root().join("in-tray")
}

pub fn projects_dir() -> PathBuf {
    root().join("projects")
}

pub fn archive_file() -> PathBuf {
    root().join("archive.md")
}

pub fn project_dir(name: &str) -> PathBuf {
    projects_dir().join(name)
}

pub fn project_tasks_file(name: &str) -> PathBuf {
    project_dir(name).join("tasks.md")
}

pub fn project_reference_dir(name: &str) -> PathBuf {
    project_dir(name).join("reference")
}

/// Ensure all GTD directories and files exist.
pub fn ensure_dirs() -> Result<()> {
    let dirs = [root(), in_tray_dir(), projects_dir()];
    for d in &dirs {
        fs::create_dir_all(d)
            .with_context(|| format!("Failed to create directory: {}", d.display()))?;
    }

    let files = [
        (tasks_file(), "# Next Actions\n"),
        (waiting_for_file(), "# Waiting For\n"),
        (someday_maybe_file(), "# Someday / Maybe\n"),
        (archive_file(), "# Archive\n"),
    ];
    for (path, header) in &files {
        if !path.exists() {
            fs::write(path, header)
                .with_context(|| format!("Failed to create file: {}", path.display()))?;
        }
    }

    Ok(())
}

/// List all project names (directory names under projects/).
pub fn list_projects() -> Result<Vec<String>> {
    let dir = projects_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut projects = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            projects.push(name.to_string());
        }
    }
    projects.sort();
    Ok(projects)
}

/// GTD list types that a task can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "public API for future use by external consumers")]
pub enum GtdList {
    NextActions,
    WaitingFor,
    SomedayMaybe,
    Project,
}

#[expect(dead_code, reason = "public API for future use by external consumers")]
impl GtdList {
    /// Resolve the file path for this list. For Project, pass the project name.
    pub fn file_path(&self, project_name: Option<&str>) -> Result<PathBuf> {
        match self {
            GtdList::NextActions => Ok(tasks_file()),
            GtdList::WaitingFor => Ok(waiting_for_file()),
            GtdList::SomedayMaybe => Ok(someday_maybe_file()),
            GtdList::Project => {
                let name = project_name
                    .ok_or_else(|| anyhow::anyhow!("project name required for Project list"))?;
                Ok(project_tasks_file(name))
            }
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GtdList::NextActions => "Next Actions",
            GtdList::WaitingFor => "Waiting For",
            GtdList::SomedayMaybe => "Someday/Maybe",
            GtdList::Project => "Project",
        }
    }
}
