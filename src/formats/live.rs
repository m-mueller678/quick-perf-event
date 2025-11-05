use super::{Format, LiveTable, TabledFloat};
use crate::{
    counters::{CounterReading, Counters, count_counters},
    visit,
};
use std::{env, error::Error};

pub struct Live {
    inner: Option<Inner>,
}

struct Inner {
    table: LiveTable,
    reading_buffer: Vec<CounterReading>,
}

impl Live {
    pub fn new() -> Self {
        Live { inner: None }
    }
}

impl Format for Live {
    fn push(
        &mut self,
        scale: usize,
        _start_time: std::time::SystemTime,
        counters: &mut dyn Counters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
        label_names: &'static [&'static str],
    ) -> Result<(), Box<dyn Error>> {
        let mut err = Ok(());
        let this = self.inner.get_or_insert_with(|| {
            let num_counters = count_counters(counters);
            let mut table = LiveTable::new(
                label_names.len() + 1 + num_counters,
                9,
                env::var("QPE_LINE_LEN")
                    .ok()
                    .and_then(|x| {
                        x.parse()
                            .map_err(|_| {
                                eprintln!("failed to parse line len: {x:?}");
                            })
                            .ok()
                    })
                    .or_else(|| terminal_size::terminal_size().map(|x| x.0.0 as usize))
                    .unwrap_or(160),
            );
            let push = &mut |x: &str| {
                if err.is_ok() {
                    err = table.push(x.to_string());
                }
            };
            visit(label_names, push);
            push("scale");
            counters.names(push);
            Inner {
                table,
                reading_buffer: Vec::with_capacity(num_counters),
            }
        });
        let push = &mut |x: &str| {
            if err.is_ok() {
                err = this.table.push(x.to_string());
            }
        };
        labels(push);
        this.reading_buffer.clear();
        counters.read(&mut this.reading_buffer);
        err?;
        this.table.push(TabledFloat(scale as f64).to_string())?;
        for reading in &this.reading_buffer {
            this.table
                .push(TabledFloat(reading.scaled_value(scale)).to_string())?;
        }
        Ok(())
    }

    fn dump_and_reset(
        &mut self,
        _label_names: &'static [&'static str],
        _counters: &mut dyn Counters,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(this) = &mut self.inner {
            this.table.end_table()?;
        }
        Ok(())
    }
}
