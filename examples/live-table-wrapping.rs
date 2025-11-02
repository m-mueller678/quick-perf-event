use quick_perf_event::LiveTable;

fn print_table<'a>(
    column_count: usize,
    line_width: usize,
    contents: impl Iterator<Item = &'a str>,
) {
    let mut table = LiveTable::new(column_count, 7, line_width);
    for cell in contents {
        table.push(cell.to_string()).unwrap();
    }
    table.end_table().unwrap();
}

fn main() {
    print_table(
        5,
        25,
        [
            "head 1",
            "head 2",
            "head 3",
            "head 4 (long)",
            "head 5 (even longer !!!!)",
            "A1",
            "A2",
            "A3",
            "A4",
            "A5",
            "B1",
            "B2",
            "B3 (long)",
            "B4",
            "B5",
        ]
        .iter()
        .copied(),
    );
}
