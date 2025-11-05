use super::Format;
use crate::counters::{Counters, CounterReading};
use std::{error::Error, iter, mem};
use tabled::settings::Style;

struct PerfReadingExtra {
    scale: usize,
    labels: Vec<String>,
    counters: Vec<CounterReading>,
}

pub struct Tabled {
    readings: Vec<PerfReadingExtra>,
    markdown: bool,
}

impl Tabled {
    pub fn new() -> Self {
        Tabled {
            readings: Vec::new(),
            markdown: true,
        }
    }
}

impl Format for Tabled {
    fn push(
        &mut self,
        scale: usize,
        _start_time: std::time::SystemTime,
        counters: &mut dyn Counters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
        _label_names: &'static [&'static str],
    ) -> Result<(), Box<dyn Error>> {
        let mut label_vec = Vec::new();
        labels(&mut |l: &str| label_vec.push(l.to_string()));
        self.readings.push(PerfReadingExtra {
            scale,
            labels: label_vec,
            counters: {
                let mut dst = Vec::new();
                counters.read(&mut dst);
                dst
            },
        });
        Ok(())
    }

    fn dump_and_reset(
        &mut self,
        label_names: &'static [&'static str],
        counters: &mut dyn Counters,
    ) -> Result<(), Box<dyn Error>> {
        let mut table = tabled::builder::Builder::new();
        table.push_record(label_names.iter().copied());
        for reading in &mut self.readings {
            table.push_record(mem::take(&mut reading.labels));
        }
        let any_multiplexed = self
            .readings
            .iter()
            .flat_map(|x| &x.counters)
            .any(|x| x.multiplexed);
        let mut name_i = 0;
        counters.names(&mut |name| {
            let readings = || {
                self.readings
                    .iter()
                    .map(|x| x.counters[name_i].scaled_value(x.scale))
            };
            table.push_column(
                iter::once(name.to_string()).chain(readings().map(|x| format!("{x:3.3}"))),
            );
            name_i += 1;
        });
        let multiplex_warning = if any_multiplexed {
            "⚠️ Some counters were multiplexed.\n"
        } else {
            "\n"
        };
        let mut table = table.build();
        if self.markdown {
            table.with(Style::markdown());
        }
        println!("{multiplex_warning}{table}");
        Ok(())
    }
}
