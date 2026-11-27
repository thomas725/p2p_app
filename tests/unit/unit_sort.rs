use super::{SortableRow, sort_rows_by_column};

#[derive(Debug, Clone)]
struct Row {
    id: &'static str,
    name: &'static str,
    last_seen: &'static str,
    first_seen: &'static str,
    dm_count: i64,
    broadcasts: i64,
}

impl SortableRow for Row {
    fn display_name(&self) -> &str {
        self.name
    }

    fn stable_id(&self) -> &str {
        self.id
    }

    fn last_seen(&self) -> &str {
        self.last_seen
    }

    fn first_seen(&self) -> &str {
        self.first_seen
    }

    fn dm_count(&self) -> i64 {
        self.dm_count
    }

    fn broadcasts_sent(&self) -> i64 {
        self.broadcasts
    }
}

fn sample_rows() -> Vec<Row> {
    vec![
        Row {
            id: "peer-b",
            name: "Bob",
            last_seen: "2024-01-01 00:00:02",
            first_seen: "2024-01-01 00:00:01",
            dm_count: 1,
            broadcasts: 3,
        },
        Row {
            id: "peer-a",
            name: "alice",
            last_seen: "not-a-date",
            first_seen: "2024-01-01T00:00:03",
            dm_count: 1,
            broadcasts: 2,
        },
        Row {
            id: "peer-c",
            name: "Alice",
            last_seen: "2024-01-01T00:00:04",
            first_seen: "2024-01-01 00:00:02",
            dm_count: 2,
            broadcasts: 0,
        },
    ]
}

fn ids(rows: &[Row]) -> Vec<&str> {
    rows.iter().map(|r| r.id).collect()
}

#[test]
fn name_column_is_case_insensitive_and_ties_on_stable_id() {
    let rows = sample_rows();
    assert_eq!(
        ids(&sort_rows_by_column(rows, 0, true)),
        vec!["peer-a", "peer-c", "peer-b"]
    );
}

#[test]
fn unknown_column_falls_back_to_name_not_first_seen() {
    let rows = sample_rows();
    assert_eq!(
        ids(&sort_rows_by_column(rows, 99, true)),
        vec!["peer-a", "peer-c", "peer-b"]
    );
}

#[test]
fn count_columns_tie_on_stable_id_and_descending_reverses_the_whole_order() {
    let rows = sample_rows();
    assert_eq!(
        ids(&sort_rows_by_column(rows.clone(), 1, true)),
        vec!["peer-a", "peer-b", "peer-c"]
    );
    assert_eq!(
        ids(&sort_rows_by_column(rows, 1, false)),
        vec!["peer-c", "peer-b", "peer-a"]
    );
}

#[test]
fn broadcasts_column_orders_by_sent_count() {
    let rows = sample_rows();
    assert_eq!(
        ids(&sort_rows_by_column(rows, 2, true)),
        vec!["peer-c", "peer-a", "peer-b"]
    );
}

#[test]
fn seen_columns_use_the_canonical_parser_with_zero_on_failure() {
    let rows = sample_rows();
    assert_eq!(
        ids(&sort_rows_by_column(rows.clone(), 3, true)),
        vec!["peer-a", "peer-b", "peer-c"]
    );
    assert_eq!(
        ids(&sort_rows_by_column(rows, 4, true)),
        vec!["peer-b", "peer-c", "peer-a"]
    );
}
