use clap::Subcommand;

#[derive(Subcommand)]
pub enum LessonAction {
    #[command(about = "Save a lesson under a topic (author_role from BPMUX_ROLE)")]
    Save {
        #[arg(long, help = "Topic name")]
        topic: String,
        #[arg(long, help = "The lesson text")]
        lesson: String,
        #[arg(
            long,
            value_delimiter = ',',
            help = "Optional comma-separated evidence refs"
        )]
        evidence: Vec<String>,
    },
    #[command(about = "List lessons, optionally filtered by topic")]
    List {
        #[arg(long, help = "Filter to a topic")]
        topic: Option<String>,
        #[arg(long, help = "Max lessons to show")]
        limit: Option<u64>,
    },
    #[command(about = "List a connected peer's lessons")]
    ListPeer {
        #[arg(long, help = "Peer name (optional)")]
        peer: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum CiAction {
    #[command(about = "Show the current CI state")]
    State,
    #[command(about = "Show the CI log for a run")]
    Log {
        #[arg(long, help = "CI run id")]
        run: String,
    },
    #[command(about = "Show per-job status for a run")]
    Jobs {
        #[arg(long, help = "CI run id")]
        run: String,
    },
    #[command(about = "Rerun a CI run")]
    Rerun {
        #[arg(long, help = "CI run id")]
        run: String,
    },
}

#[derive(Subcommand)]
pub enum PrincipleAction {
    #[command(about = "Add a principle under a value")]
    Add {
        #[arg(long, help = "Value name the principle belongs to")]
        value: String,
        #[arg(long, help = "The principle text")]
        principle: String,
    },
    #[command(about = "Update a principle's text under a value")]
    Update {
        #[arg(long, help = "Value name")]
        value: String,
        #[arg(long, help = "Existing principle text")]
        old: String,
        #[arg(long, help = "Replacement principle text")]
        new: String,
    },
    #[command(about = "Remove a principle from a value")]
    Remove {
        #[arg(long, help = "Value name")]
        value: String,
        #[arg(long, help = "The principle text to remove")]
        principle: String,
    },
}

#[derive(Subcommand)]
pub enum ValueAction {
    #[command(about = "Add an identity value")]
    Add {
        #[arg(long, help = "Value name")]
        name: String,
        #[arg(long, help = "Value description")]
        description: String,
    },
}

#[derive(Subcommand)]
pub enum BookmarkAction {
    #[command(about = "Save a session bookmark for this role")]
    Save {
        #[arg(long, help = "Current task")]
        current_task: String,
        #[arg(long, help = "Next step (optional)")]
        next_step: Option<String>,
        #[arg(long, help = "Notes (optional)")]
        notes: Option<String>,
        #[arg(
            long = "decision",
            value_name = "DECISION",
            help = "A decision (repeatable)"
        )]
        decisions: Vec<String>,
        #[arg(
            long = "open-question",
            value_name = "QUESTION",
            help = "An open question (repeatable)"
        )]
        open_questions: Vec<String>,
    },
    #[command(about = "Get a session bookmark (defaults to this role)")]
    Get {
        #[arg(long, help = "Role whose bookmark to load (optional)")]
        role: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BriefingAction {
    #[command(about = "Save this role's briefing")]
    Save {
        #[arg(help = "Briefing content (markdown)")]
        content: String,
    },
    #[command(about = "Get a briefing (defaults to this role)")]
    Get {
        #[arg(long, help = "Role whose briefing to load (optional)")]
        role: Option<String>,
    },
}
