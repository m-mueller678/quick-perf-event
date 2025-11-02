use std::{
    io::{self, StdoutLock, Write, stdout},
    iter,
};

pub struct StreamingTable {
    column_count: usize,
    columns_per_line: usize,
    cell_size: usize,
    line: Vec<String>,
    columns_written: usize,
    table_started: bool,
    record_seprator: String,
    wrapping_separator: String,
    field_separator: &'static str,
}

impl StreamingTable {
    pub fn new(column_count: usize, cell_size: usize, line_width: usize) -> Self {
        let columns_per_line = ((line_width - 1) / (cell_size + 1)).max(1);
        assert!(column_count > 0);
        assert!(cell_size > 2);
        let mut ret = StreamingTable {
            column_count,
            columns_per_line,
            cell_size,
            line: Vec::new(),
            columns_written: 0,
            table_started: false,
            record_seprator: String::new(),
            wrapping_separator: String::new(),
            field_separator: "│",
        };
        if ret.columns_per_line < ret.column_count {
            ret.wrapping_separator = ret.make_separator(&["│", "│", "│"], "-");
        } else {
            ret.columns_per_line = ret.column_count;
        }
        ret.record_seprator = ret.make_separator(&["├", "┼", "┤"], "─");
        ret
    }

    pub fn push(&mut self, x: String) -> io::Result<()> {
        self.line.push(x);
        if self.line.len() == self.columns_per_line
            || self.columns_written + self.line.len() == self.column_count
        {
            let stdout = stdout();
            let mut stdout = stdout.lock();
            if self.columns_written == 0 {
                if !self.table_started {
                    self.table_started = true;
                    write!(stdout, "{}\n", self.make_separator(&["┌", "┬", "┐"], "─"))?;
                } else {
                    write!(stdout, "{}\n", self.record_seprator)?;
                }
            } else {
                write!(stdout, "{}\n", self.wrapping_separator)?;
            }
            self.write_content_lines(&mut stdout)?;
            if self.columns_written >= self.column_count {
                self.columns_written = 0;
            }
        }
        Ok(())
    }

    fn write_content_lines(&mut self, stdout: &mut StdoutLock) -> io::Result<()> {
        let cells: Vec<_> = self
            .line
            .iter()
            .map(|s| textwrap::wrap(s, self.cell_size))
            .collect();
        let lines = cells.iter().map(|w| w.len()).max().unwrap();
        for i in 0..lines {
            write!(stdout, "{}", self.field_separator)?;
            for c in 0..self.columns_per_line {
                let width = self.cell_size;
                let content = cells
                    .get(c)
                    .and_then(|x| x.get(i))
                    .map(|x| x.as_ref())
                    .unwrap_or("");
                write!(stdout, "{content:^width$}")?;
                write!(stdout, "{}", self.field_separator)?;
            }
            write!(stdout, "\n")?;
        }
        self.columns_written += self.line.len();
        self.line.clear();
        Ok(())
    }

    fn make_separator(&self, crosses: &[&str; 3], line: &str) -> String {
        let mut ret = String::new();
        ret.push_str(crosses[0]);
        ret.extend(iter::repeat_n(line, self.cell_size));
        for _ in 1..self.columns_per_line {
            ret.push_str(crosses[1]);
            ret.extend(iter::repeat_n(line, self.cell_size));
        }
        ret.push_str(crosses[2]);
        ret
    }

    pub fn end_table(&mut self) -> io::Result<()> {
        write!(stdout(), "{}\n", self.make_separator(&["└", "┴", "┘"], "─"))?;
        self.table_started = false;
        Ok(())
    }

    pub fn table_started(&mut self) -> bool {
        self.table_started
    }
}
