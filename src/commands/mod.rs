mod add;
mod capture;
mod done;
mod edit;
mod list;
mod process;
mod project;
mod review;
mod search;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gtd", about = "Getting Things Done - CLI workflow manager")]
#[command(version, propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Quick-capture a thought into the in-tray
    #[command(alias = "inbox")]
    Capture {
        /// The item to capture (omit for interactive prompt)
        description: Vec<String>,
    },

    /// Process in-tray items through the GTD decision tree
    Process,

    /// Add a task directly to a specific list
    Add {
        /// Task description
        description: Vec<String>,

        /// Add to waiting-for list
        #[arg(short = 'w', long)]
        waiting: bool,

        /// Add to someday/maybe list
        #[arg(short = 's', long)]
        someday: bool,

        /// Add to a project
        #[arg(short, long)]
        project: Option<String>,

        /// Due date (YYYY-MM-DD)
        #[arg(short, long)]
        due: Option<String>,

        /// Delegated to (person name)
        #[arg(long)]
        delegated_to: Option<String>,
    },

    /// Mark a task as done and archive it
    Done {
        /// Task ID (short hash)
        id: String,
    },

    /// List tasks, optionally filtered
    #[command(alias = "ls")]
    List {
        /// Filter: next, waiting, someday, project:<name>, all
        #[arg(default_value = "next")]
        filter: String,

        /// Show completed tasks too
        #[arg(short = 'a', long)]
        show_all: bool,
    },

    /// Create or manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Weekly review workflow
    Review,

    /// Open a GTD file in $EDITOR
    Edit {
        /// File to edit: tasks, waiting, someday, intray, project:<name>
        #[arg(default_value = "tasks")]
        target: String,
    },

    /// Search across all GTD files
    Search {
        /// Search query
        query: Vec<String>,
    },
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// Create a new project
    New {
        /// Project name (used as directory name)
        name: String,
    },
    /// List all projects
    List,
    /// Delete a project (moves tasks to archive)
    Delete {
        /// Project name
        name: String,
    },
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Capture { description } => capture::run(description),
        Command::Process => process::run(),
        Command::Add {
            description,
            waiting,
            someday,
            project,
            due,
            delegated_to,
        } => add::run(description, waiting, someday, project, due, delegated_to),
        Command::Done { id } => done::run(&id),
        Command::List { filter, show_all } => list::run(&filter, show_all),
        Command::Project { action } => project::run(action),
        Command::Review => review::run(),
        Command::Edit { target } => edit::run(&target),
        Command::Search { query } => search::run(query),
    }
}
