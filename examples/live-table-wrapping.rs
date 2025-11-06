use quick_perf_event::formats::LiveTable;

fn print_table<'a>(
    column_widths: Vec<usize>,
    line_width: usize,
    contents: impl Iterator<Item = &'a str>,
) {
    let mut table = LiveTable::new(column_widths, line_width);
    for cell in contents {
        table.push(cell.to_string()).unwrap();
    }
    table.end_table().unwrap();
}

fn main() {
    let content = [
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
    ];
    print_table(vec![10, 10, 10, 10, 10], 35, content.iter().copied());
    print_table(vec![5, 5, 5, 5, 5], 40, content.iter().copied());
    print_table(vec![10, 10, 10, 20, 25], 50, content.iter().copied());
    print_table(vec![10, 10, 10, 20, 30], 50, content.iter().copied());
    let content = [
        "label (pretty long!)",
        "val1",
        "val2",
        "val3",
        "val4 (sometimes long too)",
        "aaaaaaaaaaaaaaaaaaa",
        "3.5",
        "4.7",
        "10",
        "34",
        "vvvvvvvvvvvvvvvvvvv",
        "890",
        "55",
        "1234",
        "1000000000000",
    ];
    print_table(vec![20, 8, 8, 8, 8], 40, content.iter().copied());
    print_table(vec![20, 8, 8, 8, 16], 40, content.iter().copied());
    print_table(vec![20, 8, 8, 8, 16], 80, content.iter().copied());
}
