use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum EpicAction {
    #[command(about = "Group a child story under an Epic")]
    SetParent {
        #[arg(help = "Child story number")]
        number: u64,
        #[arg(long, help = "Epic issue number that owns the child")]
        epic: u64,
    },
    #[command(about = "Ungroup a child story from its current Epic")]
    UnsetParent {
        #[arg(help = "Child story number")]
        number: u64,
    },
    #[command(about = "List the child stories under an Epic")]
    Children {
        #[arg(help = "Epic issue number")]
        epic: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum IterationAction {
    #[command(about = "Create an iteration with a theme")]
    Create {
        #[arg(long, help = "Iteration theme name")]
        theme: String,
        #[arg(long, help = "Insert after this iteration number (optional)")]
        position_after: Option<u64>,
    },
    #[command(about = "Designate the current iteration by number")]
    Designate {
        #[arg(help = "Iteration number")]
        number: u64,
    },
    #[command(about = "Add a member story to an iteration at a 1-indexed position")]
    Add {
        #[arg(long, help = "Iteration entity issue number")]
        iteration: u64,
        #[arg(long, help = "Member story issue number")]
        story: u64,
        #[arg(long, help = "1-indexed position within the iteration")]
        position: u64,
    },
    #[command(about = "Close an iteration (terminal; the ready queue must be empty)")]
    Close {
        #[arg(long, help = "Iteration entity issue number")]
        iteration: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum LessonAction {
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
pub(crate) enum CiAction {
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
pub(crate) enum PrincipleAction {
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
pub(crate) enum ValueAction {
    #[command(about = "Add an identity value")]
    Add {
        #[arg(long, help = "Value name")]
        name: String,
        #[arg(long, help = "Value description")]
        description: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum BookmarkAction {
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
pub(crate) enum RoomAction {
    Spawn {
        role: String,
        #[arg(long, default_value = "claude-code")]
        agent: String,
        #[arg(long)]
        headless: bool,
    },
    List {},
    Read {
        role: String,
        #[arg(long)]
        lines: Option<usize>,
    },
    Kill {
        role: String,
    },
}
