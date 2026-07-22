pub fn run_help(json: bool) -> anyhow::Result<()> {
    let command = <crate::bootstrap::Cli as clap::CommandFactory>::command();
    if json {
        let verbs: Vec<serde_json::Value> = command
            .get_subcommands()
            .map(|sub| {
                serde_json::json!({
                    "verb": sub.get_name(),
                    "about": sub.get_about().map(|about| about.to_string()).unwrap_or_default(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "verbs": verbs }))?
        );
    } else {
        for sub in command.get_subcommands() {
            let about = sub
                .get_about()
                .map(|about| about.to_string())
                .unwrap_or_default();
            println!("{:20} {about}", sub.get_name());
        }
    }
    Ok(())
}
