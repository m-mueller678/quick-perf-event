mod labels;
mod perf_counters;
mod streaming_table;
mod tabled_float;

pub use labels::Labels;
pub use perf_counters::{PerfCounters, PerfReading};
#[doc(hidden)]
pub use streaming_table::StreamingTable;
pub use tabled_float::TabledFloat;

use perf_event::CounterData;
use std::{
    borrow::Borrow,
    error::Error,
    io::{Write, stdout},
    iter,
    marker::PhantomData,
    mem,
    sync::Once,
    time::SystemTime,
};
use tabled::settings::Style;

pub struct PerfEvent<L> {
    inner: PerfEvent2,
    _p: PhantomData<L>,
}

struct PerfEvent2 {
    counters: PerfCounters,
    state: OutputState,
    label_names: &'static [&'static str],
}

enum OutputState {
    Tabled {
        readings: Vec<PerfReadingExtra>,
        markdown: bool,
    },
    Interactive {
        table: StreamingTable,
    },
    Csv {
        header_written: bool,
        writer: csv::Writer<Box<dyn Write>>,
    },
}

impl PerfEvent2 {
    fn push(
        &mut self,
        scale: usize,
        _start_time: SystemTime,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
    ) -> Result<(), Box<dyn Error>> {
        match &mut self.state {
            OutputState::Tabled {
                readings,
                markdown: _,
            } => {
                let mut label_vec = Vec::new();
                labels(&mut |l: &str| label_vec.push(l.to_string()));
                readings.push(PerfReadingExtra {
                    scale,
                    labels: label_vec,
                    counters: self.counters.read_counters(),
                });
            }
            OutputState::Interactive { table } => {
                if !table.table_started() {
                    for name in self.label_names {
                        table.push(name.to_string())?;
                    }
                    for val in Self::value_names(&self.counters) {
                        table.push(val.to_string())?;
                    }
                }
                let mut err = Ok(());
                labels(&mut |label| {
                    if err.is_ok() {
                        err = table.push(label.to_string());
                    }
                });
                err?;
                let mut discard_multiplexed = false;
                for val in Self::values(
                    &self.counters.read_counters(),
                    scale,
                    &mut discard_multiplexed,
                ) {
                    table.push(TabledFloat(val).to_string())?;
                }
            }
            OutputState::Csv {
                header_written,
                writer,
            } => {
                macro_rules! write_field {
                    ($x:expr) => {
                        writer.write_field($x)?;
                    };
                }
                if !*header_written {
                    *header_written = true;
                    for l in self.label_names {
                        write_field!(l);
                    }
                    for name in Self::value_names(&self.counters) {
                        write_field!(name);
                    }
                    write_field!("multiplexed");
                    writer.write_record(iter::empty::<&[u8]>())?;
                }
                let mut err = Ok(());
                labels(&mut |l| {
                    if err.is_ok() {
                        err = writer.write_field(l);
                    }
                });
                err?;
                let reading = self.counters.read_counters();
                let mut any_multiplexed = false;
                for value in Self::values(&reading, scale, &mut any_multiplexed) {
                    write_field!(&format!("{value}"));
                }
                write_field!(&format!("{any_multiplexed}"));
                writer.write_record(iter::empty::<&[u8]>())?;
                writer.flush()?;
            }
        }
        Ok(())
    }

    fn process_counter(counter: &CounterData, scale: usize) -> (f64, bool) {
        let multiplexed = counter.time_enabled() != counter.time_running();
        let scaled = counter.count() as f64 * counter.time_running().unwrap().as_secs_f64()
            / counter.time_enabled().unwrap().as_secs_f64()
            / scale as f64;
        (scaled, multiplexed)
    }

    fn report_error(err: Result<(), Box<dyn Error>>) {
        static ONCE: Once = Once::new();
        if let Err(err) = err {
            ONCE.call_once(|| {
                eprintln!("QPE failed to write. Future errors will not be reported. {err}");
            });
        }
    }

    fn dump_and_reset(&mut self) {
        match &mut self.state {
            OutputState::Tabled { readings, markdown } => {
                let mut table = tabled::builder::Builder::new();
                table.push_record(self.label_names.iter().copied());
                for reading in &mut *readings {
                    table.push_record(mem::take(&mut reading.labels));
                }
                let multiplexed = readings
                    .iter()
                    .flat_map(|x| x.counters.counters.iter())
                    .any(|x| x.time_enabled() != x.time_running());
                for (i, name) in self.counters.names().enumerate() {
                    let readings = || {
                        readings.iter().map(|x| {
                            let c = &x.counters.counters[i];
                            c.count() as f64 * c.time_enabled().unwrap().as_secs_f64()
                                / c.time_running().unwrap().as_secs_f64()
                                / x.scale as f64
                        })
                    };
                    // let max = readings().max_by(f64::total_cmp).unwrap();
                    // let min = readings().min_by(f64::total_cmp).unwrap();
                    // let max_scale = max.log10().floor() as isize;
                    // let min_scale = min.log10().floor() as isize;
                    table.push_column(
                        iter::once(name.to_string()).chain(readings().map(|x| format!("{x:3.3}"))),
                    );
                }
                let multiplex_warning = if multiplexed {
                    "⚠️ Some counter were multiplexed.\n"
                } else {
                    "\n"
                };
                let mut table = table.build();
                if *markdown {
                    table.with(Style::markdown());
                }
                println!("{multiplex_warning}{table}");
            }
            OutputState::Interactive { table } => {
                Self::report_error(table.end_table().map_err(Into::into));
            }
            OutputState::Csv {
                header_written,
                writer: _,
            } => {
                *header_written = false;
            }
        }
    }

    fn value_names(counters: &PerfCounters) -> impl Iterator<Item = &str> {
        ["time", "scale"].into_iter().chain(counters.names())
    }

    fn values(
        counters: &PerfReading,
        scale: usize,
        any_multiplexed: &mut bool,
    ) -> impl Iterator<Item = f64> {
        let time = counters.duration.as_secs_f64();
        let counters = counters.counters.iter();
        let counters = counters.map(move |x| {
            let x = Self::process_counter(x, scale);
            *any_multiplexed |= x.1;
            x.0
        });
        [time, scale as f64].into_iter().chain(counters)
    }
}

struct PerfReadingExtra {
    scale: usize,
    labels: Vec<String>,
    counters: PerfReading,
}

impl<L: Labels> Default for PerfEvent<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Labels> PerfEvent<L> {
    pub fn new() -> Self {
        Self::with_counters(PerfCounters::new())
    }

    pub fn with_counters(counters: PerfCounters) -> Self {
        PerfEvent {
            inner: PerfEvent2 {
                state: match std::env::var("QPE_FORMAT").as_deref() {
                    Ok("csv") => OutputState::Csv {
                        header_written: false,
                        writer: csv::Writer::from_writer(Box::new(stdout())),
                    },
                    Ok("md") => OutputState::Tabled {
                        readings: Vec::new(),
                        markdown: true,
                    },
                    x => {
                        match x {
                            Ok(requested) => {
                                eprintln!(
                                    "unrecognized value for QPE_FORMAT: {requested:?}.\nSupported values: csv, md"
                                );
                            }
                            Err(_) => {}
                        }
                        OutputState::Interactive {
                            table: StreamingTable::new(
                                L::names().len() + 2 + counters.names().count(),
                                9,
                                160,
                            ),
                        }
                    }
                },
                counters,
                label_names: L::names(),
            },

            _p: PhantomData,
        }
    }

    pub fn run<R>(&mut self, scale: usize, labels: impl Borrow<L>, f: impl FnOnce() -> R) -> R
    where
        L: Labels,
    {
        let start_time = SystemTime::now();
        self.inner.counters.reset();
        self.inner.counters.enable();
        let ret = f();
        self.inner.counters.disable();
        PerfEvent2::report_error(
            self.inner
                .push(scale, start_time, &mut |dst| labels.borrow().values(dst)),
        );
        ret
    }
}

impl Drop for PerfEvent2 {
    fn drop(&mut self) {
        self.dump_and_reset();
    }
}
