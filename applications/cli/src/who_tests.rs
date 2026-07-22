use super::who::who_lines;

#[test]
fn who_lists_one_line_per_linked_agent_with_the_id_first_so_it_can_be_copied_into_a_to_id() {
    let body = r#"{"fetched_at":"2026-07-22T18:00:00Z","roster":[
        {"id":"pm-alpha","identity":"pm","role":"pm","presence":"Online"},
        {"id":"6b3054e9-dir-keyed","identity":"dev","role":"dev","presence":"Busy"}
    ]}"#;

    let lines = who_lines(body);

    assert_eq!(
        lines,
        vec![
            "pm-alpha  pm  pm  Online",
            "6b3054e9-dir-keyed  dev  dev  Busy",
        ]
    );
}

#[test]
fn who_reports_an_empty_room_plainly_instead_of_printing_nothing() {
    let lines = who_lines(r#"{"fetched_at":"2026-07-22T18:00:00Z","roster":[]}"#);
    assert_eq!(lines, vec!["no linked agents"]);
}
