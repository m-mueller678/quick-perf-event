use std::{
    io::{self, StdoutLock, Write, stdout},
    iter,
};

/// This is exported for use in examples and not considered part of the public API.
/// Users should not rely on it.
#[doc(hidden)]
pub struct LiveTable {
    column_groups: Vec<Vec<usize>>,
    current_cgroup: usize,
    column_group_separators: Vec<String>,
    head_separator: String,
    end_separator: String,
    line: Vec<String>,
    table_started: bool,
    field_separator: &'static str,
    line_delimiter: &'static str,
}

fn group_width(g: &[usize]) -> usize {
    g.iter().sum::<usize>() + g.len() + 1
}

fn compute_column_widths(requested: Vec<usize>, line_width: usize) -> Vec<Vec<usize>> {
    let mut column_groups = Vec::new();
    {
        let mut group = Vec::new();
        let mut group_width = 1;
        for w in requested {
            if group_width + w + 1 > line_width {
                column_groups.push(std::mem::take(&mut group));
                group_width = 1;
            }
            group.push(w.min(line_width - group_width - 1));
            group_width += w + 1;
        }
        if !group.is_empty() {
            column_groups.push(group);
        }
    }
    let line_width = column_groups.iter().map(|x| group_width(x)).max().unwrap();
    for g in &mut column_groups {
        let rest1 = line_width.saturating_sub(group_width(g));
        g[0] += rest1;
    }
    column_groups
}

impl LiveTable {
    pub fn new(columns_widths: Vec<usize>, line_width: usize) -> Self {
        let line_width = line_width.max(9);
        let column_groups = compute_column_widths(columns_widths, line_width);
        let cgl = column_groups.len();
        LiveTable {
            line: Vec::new(),
            table_started: false,
            field_separator: "│",
            line_delimiter: "│",
            current_cgroup: 0,
            column_group_separators: (0..column_groups.len())
                .map(|i| {
                    if i == 0 {
                        Self::make_separtor(
                            &column_groups[(i + cgl - 1) % cgl],
                            &column_groups[i],
                            line_width,
                            "├",
                            &["─", "┴", "┬", "┼"],
                            "┤",
                        )
                    } else {
                        Self::make_separtor(
                            &column_groups[(i + cgl - 1) % cgl],
                            &column_groups[i],
                            line_width,
                            "├",
                            &["╌", "┴", "┬", "┼"],
                            "┤",
                        )
                    }
                })
                .collect(),
            head_separator: Self::make_separtor(
                &column_groups[0],
                &column_groups[0],
                line_width,
                "┌",
                &["─", "─", "┬", "┬"],
                "┐",
            ),
            end_separator: Self::make_separtor(
                &column_groups[cgl - 1],
                &column_groups[cgl - 1],
                line_width,
                "└",
                &["─", "┴", "─", "┴"],
                "┘",
            ),
            column_groups,
        }
    }

    pub fn push(&mut self, x: String) -> io::Result<()> {
        self.line.push(x);
        assert!(self.current_cgroup < self.column_groups.len());
        if self.line.len() == self.column_groups[self.current_cgroup].len() {
            let stdout = stdout();
            let mut stdout = stdout.lock();
            if !self.table_started {
                self.table_started = true;
                write!(stdout, "{}", self.head_separator)?;
            } else {
                write!(
                    stdout,
                    "{}",
                    self.column_group_separators[self.current_cgroup]
                )?;
            }
            self.write_content_lines(&mut stdout)?;
            self.current_cgroup = (self.current_cgroup + 1) % self.column_groups.len();
        }
        Ok(())
    }

    fn write_content_lines(&mut self, stdout: &mut StdoutLock) -> io::Result<()> {
        let col_widths = &self.column_groups[self.current_cgroup];
        let cells = self.line.iter().zip(col_widths);
        let cells: Vec<_> = cells.map(|(s, w)| textwrap::wrap(s, *w)).collect();
        let lines = cells.iter().map(|w| w.len()).max().unwrap();
        for l in 0..lines {
            write!(stdout, "{}", self.line_delimiter)?;
            for (ci, (cell, width)) in cells.iter().zip(col_widths).enumerate() {
                let content = cell.get(l).map(|x| x.as_ref()).unwrap_or("");
                write!(stdout, "{content:^width$}")?;
                if ci + 1 == cells.len() {
                    write!(stdout, "{}\n", self.line_delimiter)?;
                } else {
                    write!(stdout, "{}", self.field_separator)?;
                }
            }
        }
        self.line.clear();
        Ok(())
    }

    fn make_separtor(
        above: &[usize],
        below: &[usize],
        line_width: usize,
        start: &str,
        crosses: &[&str],
        end: &str,
    ) -> String {
        let mut cross_types = vec![0u8; group_width(above) - 2];
        let mut set_ticks = |widths: &[usize], mask: u8| {
            let mut i = 0;
            for w in &widths[..widths.len() - 1] {
                i += w;
                cross_types[i] |= mask;
                i += 1;
            }
        };
        set_ticks(above, 1);
        set_ticks(below, 2);
        let parts = || {
            iter::once(start)
                .chain(cross_types.iter().map(|x| crosses[*x as usize]))
                .chain([end, "\n"])
        };
        let mut output = String::with_capacity(parts().map(|x| x.len()).sum());
        output.extend(parts());
        output
    }

    pub fn end_table(&mut self) -> io::Result<()> {
        if self.table_started {
            write!(stdout(), "{}", self.end_separator)?;
            self.table_started = false;
        }
        Ok(())
    }

    pub fn table_started(&mut self) -> bool {
        self.table_started
    }
}

#[test]
fn test_column_widths() {
    let cases = vec![
        (vec![10, 10, 10], 40, vec![vec![10, 10, 10]]),
        (vec![10, 10, 10], 25, vec![vec![10, 10], vec![21]]),
        (vec![10, 5, 10], 25, vec![vec![10, 5], vec![16]]),
        (vec![20, 10, 5], 25, vec![vec![20], vec![14, 5]]),
        (vec![20, 10, 30], 25, vec![vec![23], vec![23], vec![23]]),
        (
            vec![20, 10, 3, 30],
            25,
            vec![vec![23], vec![19, 3], vec![23]],
        ),
    ];
    for (requested, line, expected) in cases {
        let computed = compute_column_widths(requested, line);
        for l in &expected {
            let ll = group_width(&l);
            assert!(
                ll == group_width(&expected[0]),
                "bad test case: {l:?} has length {ll}"
            );
        }
        assert_eq!(computed, expected);
    }
}
