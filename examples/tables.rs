use quick_perf_event::StreamingTable;

fn print_table<'a>(
    column_count: usize,
    line_width: usize,
    contents: impl Iterator<Item = &'a str>,
) {
    let mut table = StreamingTable::new(column_count, 7, line_width);
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
            "c1",
            "c2",
            "c3",
            "c4",
            "c5",
        ]
        .iter()
        .copied(),
    );
}
