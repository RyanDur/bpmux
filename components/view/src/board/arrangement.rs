use board::Column;

pub fn column_header(column: &Column) -> String {
    match column {
        Column::Icebox => "ICEBOX".to_string(),
        Column::Ready => "READY".to_string(),
        Column::Middle(stage) => stage.as_str().to_ascii_uppercase().replace('-', " "),
        Column::Acceptance => "ACCEPTANCE".to_string(),
        Column::Accepted => "ACCEPTED".to_string(),
        Column::Retired => "RETIRED".to_string(),
    }
}
