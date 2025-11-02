//! # Quick Perf Event
//!
//! This crate provides a lightweight framework for measuring and reporting performance
//! counters across labeled workloads or benchmarks.
//! The output format can be controlled using the `QPE_FORMAT` environment variable:
//!
//! - `QPE_FORMAT=live` (default) — Uses **live table mode**, designed for development and
//!   debugging. Each result is printed as soon as it’s available, with compact,
//!   fixed-width, line-wrapped cells to fit many columns in narrow terminals.
//!   If the requested columns still do not fit, table rows are line wrapped as well.
//! - `QPE_FORMAT=md` — Generates a **Markdown table** after all runs have completed,
//!   choosing column widths automatically for clean, publication-ready output.
//! - `QPE_FORMAT=csv` — Streams results as **CSV** records to stdout, suitable for
//!   further processsing.
//!
//! ## Example
//! ```
#![doc = include_str!("../examples/short.rs")]
//! ```
//! ```text
//!failed to create counter "kcycle": Permission denied (os error 13)
//!┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
//!│  label  │  time   │  scale  │  cycle  │ l1-miss │llc-miss │ br-miss │
//!├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
//!│   sum   │   0.194 │   1.0 G │   1.007 │ 364.0 n │ 166.0 n │ 345.0 n │
//!└─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘
//! ```
//! This will produce somethinglike the above.
//! Note that the program was unable to record the number of cpu cycles spent in the kernel.
//! If you run into similar issues, you may need to configure `perf_event_paranoid`.
//! See [`man 2 perf_event_open`](https://www.man7.org/linux/man-pages/man2/perf_event_open.2.html) for what the different restiction levels mean.
//!
//! Performance counters are divided by the scale passed to the [`record`](PerfReading::record) method to give the number of events per operation.
//! The `time` column reports the wall-time in seconds elapsed over the emasurement.
//! It is not normalized.
//!
//! ## Usage
//! To start benchmarking you first need a [`QuickPerfEvent`] object.
//! [`QuickPerfEvent`] manmanages both recording and reporting of benchmarks.
//! You may configure the set of performance counters using either the environment variable `QPE_EVENTS` or [`with_counters`](QuickPerfEvent::with_counters).
//! For basic usage, you should prefer `QPE_EVENTS`.
//! For example, to count cpu cycles and branch misses, set it to `cycles,br-miss`.
//! For an up-to-date list of supported values see the implementation of [`with_counter_names`](PerfCounters::with_counter_names).
//! If your program is multi-threaded, construct [`QuickPerfEvent`] **before spawning threads** to ensure counts include other threads.
//!
//! Now that you have a [`QuickPerfEvent`] object, you may start taking measurements using its [`run`](QuickPerfEvent::run) method.
//! After each run, you **must** call [`record`](PerfReading::record) on the returned value to log the measurement.
//! The [`record`](PerfReading::record) method takes two parameters:
//!
//! - **`scale`** – a normalization factor (e.g. number of iterations).  
//!   All performance counters are divided by this value, producing results
//!   such as *branch misses per operation* or *cycles per iteration*.
//!   Note that the time column is not normalized.
//!   It reports the absolute amount of time elapsed over the measurement.
//!   Dividing this by scale would be misleading when multiple threads are involved.
//!   If you want a measure of time spent per operation, consider using the task clock counter `t-clock`.
//!
//! - **`labels`** – metadata describing the measurement.  
//!   This can be:
//!   - the unit type `()` (no labels),
//!   - a string `&str` (single label),
//!   - or a user-defined struct implementing [`Labels`].
mod labels;
mod live_table;
mod perf_counters;
mod tabled_float;

pub use labels::Labels;
pub use live_table::LiveTable;
pub use perf_counters::{PerfCounters, PerfCountersReading};
pub use tabled_float::TabledFloat;

use perf_event::CounterData;
use std::{
    borrow::Borrow,
    env,
    error::Error,
    io::{Write, stdout},
    iter,
    marker::PhantomData,
    mem,
    sync::Once,
    time::SystemTime,
};
use tabled::settings::Style;

/// Main entry point for performance measurement.
///
/// `QuickPerfEvent` encapsulates a collection of hardware performance counters (`PerfCounters`) and configuration for reporting results.
/// See the crate level documentation for more information.
///
/// The generic parameter `L` must implement [`Labels`], providing a fixed schema
/// of label names and values for each recorded sample.
/// [`Labels`] is implemented for `()` and `str`.
/// You can define a label struct conveniently using the
/// [`struct_labels!`](crate::struct_labels) macro.
pub struct QuickPerfEvent<L: ?Sized> {
    inner: PerfEventInner,
    _p: PhantomData<L>,
}

/// See [`QuickPerfEvent::run`] and the crate level docs.
#[must_use]
pub struct PerfReading<'a, L: ?Sized, T> {
    pe: &'a mut QuickPerfEvent<L>,
    start_time: SystemTime,
    ret: T,
}

impl<L: Labels + ?Sized> Default for QuickPerfEvent<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Labels + ?Sized> QuickPerfEvent<L> {
    /// Create a `QuickPerfEvent` configured from environment variables.
    pub fn new() -> Self {
        Self::with_counters(PerfCounters::new())
    }

    /// Create a `QuickPerfEvent` with a custom set of performance counters.
    pub fn with_counters(counters: PerfCounters) -> Self {
        QuickPerfEvent {
            inner: PerfEventInner::new(counters, L::names()),
            _p: PhantomData,
        }
    }

    /// Perform a measurement.
    ///
    /// You **must** call [`record`](PerfReading::record) on the returned value.
    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> PerfReading<'_, L, R> {
        let start_time = SystemTime::now();
        self.inner.counters.reset();
        self.inner.counters.enable();
        let ret = f();
        self.inner.counters.disable();
        PerfReading {
            start_time,
            ret,
            pe: self,
        }
    }
}

impl<L: Labels + ?Sized, T> PerfReading<'_, L, T> {
    /// Records the measured result.
    ///
    /// The `scale` argument normalizes counter values (e.g. per iteration count).
    /// The given `labels` instance supplies the labels for this sample.
    pub fn record(self, scale: usize, labels: impl Borrow<L>) -> T {
        PerfEventInner::report_error(self.pe.inner.push(scale, self.start_time, &mut |dst| {
            labels.borrow().values(dst)
        }));
        self.ret
    }
}

struct PerfEventInner {
    counters: PerfCounters,
    state: OutputState,
    label_names: &'static [&'static str],
}

impl Drop for PerfEventInner {
    fn drop(&mut self) {
        self.dump_and_reset();
    }
}

enum OutputState {
    Tabled {
        readings: Vec<PerfReadingExtra>,
        markdown: bool,
    },
    Live {
        table: LiveTable,
    },
    Csv {
        header_written: bool,
        writer: csv::Writer<Box<dyn Write>>,
    },
}

impl PerfEventInner {
    fn new(counters: PerfCounters, label_names: &'static [&'static str]) -> Self {
        PerfEventInner {
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
                    OutputState::Live {
                        table: LiveTable::new(
                            label_names.len() + 2 + counters.names().count(),
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
                        ),
                    }
                }
            },
            counters,
            label_names,
        }
    }
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
            OutputState::Live { table } => {
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
            OutputState::Live { table } => {
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
        counters: &PerfCountersReading,
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
    counters: PerfCountersReading,
}
