#[macro_export]
macro_rules! perf_reading_labels {
    ($vis:vis struct $Name:ident{
        $($fv:vis $f:ident:$F:ty,)*
    }) => {
        $vis struct $Name{
            $($fv $f:$F,)*
        }

        impl $crate::PerfReadingLabels for $Name{
            fn names()->&'static [&'static str]{
                &[
                    $(std::stringify!($f),)*
                ]
            }

            fn values(&self,f:&mut dyn FnMut(&str)){
                $(f(&self.$f);)*
            }
        }
    };
}

mod perf_counters;

use perf_event::CounterData;

use crate::perf_counters::{PerfCounters, PerfReading};
use std::{
    cell::OnceCell,
    io::{Write, stdout},
    iter,
    marker::PhantomData,
    mem,
    time::SystemTime,
};

pub struct PerfEvent<L> {
    counters: PerfCounters,
    state: OutputState,
    _p: PhantomData<L>,
}

enum OutputState {
    Interactive {
        readings: Vec<PerfReadingExtra>,
        label_names: &'static [&'static str],
    },
    Csv {
        header_written: OnceCell<()>,
        label_names: &'static [&'static str],
        writer: csv::Writer<Box<dyn Write>>,
    },
}

impl OutputState {
    fn push(
        &mut self,
        scale: usize,
        _start_time: SystemTime,
        counters: &mut PerfCounters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
    ) {
        match self {
            OutputState::Interactive {
                readings,
                label_names: _,
            } => {
                let mut label_vec = Vec::new();
                labels(&mut |l: &str| label_vec.push(l.to_string()));
                readings.push(PerfReadingExtra {
                    scale,
                    labels: label_vec,
                    counters: counters.read_counters(),
                });
            }
            OutputState::Csv {
                header_written,
                label_names,
                writer,
            } => {
                let mut err = Ok(());
                macro_rules! write_field {
                    ($x:expr) => {
                        if err.is_ok() {
                            err = writer.write_field($x);
                        }
                    };
                }
                header_written.get_or_init(|| {
                    for l in label_names.iter() {
                        write_field!(l);
                    }
                    for name in counters.names() {
                        write_field!(name);
                    }
                    write_field!("multiplexed");
                    if err.is_ok() {
                        err = writer.write_record(iter::empty::<&[u8]>());
                    }
                });
                labels(&mut |l| {
                    write_field!(l);
                });
                let reading = counters.read_counters();
                let mut any_multiplexed = false;
                for counter in &reading.counters {
                    let (scaled, multiplexed) = Self::process_counter(counter, scale);
                    any_multiplexed |= multiplexed;
                    write_field!(&format!("{scaled}"));
                }
                write_field!(&format!("{any_multiplexed}"));
                if err.is_ok() {
                    err = writer.write_record(iter::empty::<&[u8]>());
                }
                Self::handle_csv_result(err);
            }
        }
    }

    fn process_counter(counter: &CounterData, scale: usize) -> (f64, bool) {
        let multiplexed = counter.time_enabled() != counter.time_running();
        let scaled = counter.count() as f64 * counter.time_running().unwrap().as_secs_f64()
            / counter.time_enabled().unwrap().as_secs_f64()
            / scale as f64;
        (scaled, multiplexed)
    }

    fn handle_csv_result(err: csv::Result<()>) {
        if let Err(err) = err {
            eprintln!("error writing csv: {err}");
        }
    }

    fn dump_and_reset(&mut self, counters: &PerfCounters) {
        match self {
            OutputState::Interactive {
                readings,
                label_names,
            } => {
                let mut table = tabled::builder::Builder::new();
                table.push_record(label_names.iter().copied());
                for reading in &mut *readings {
                    table.push_record(mem::take(&mut reading.labels));
                }
                let multiplexed = readings
                    .iter()
                    .flat_map(|x| x.counters.counters.iter())
                    .any(|x| x.time_enabled() != x.time_running());
                for (i, name) in counters.names().enumerate() {
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
                println!("{multiplex_warning}{}", table.build());
            }
            OutputState::Csv {
                header_written,
                label_names: _,
                writer,
            } => {
                header_written.take();
                Self::handle_csv_result(writer.flush().map_err(Into::into));
            }
        }
    }
}

struct PerfReadingExtra {
    scale: usize,
    labels: Vec<String>,
    counters: PerfReading,
}

impl<L: PerfReadingLabels> Default for PerfEvent<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: PerfReadingLabels> PerfEvent<L> {
    pub fn new() -> Self {
        Self::with_counters(PerfCounters::new())
    }

    pub fn with_counters(counters: PerfCounters) -> Self {
        PerfEvent {
            counters,
            state: match std::env::var("QPE_FORMAT").as_deref() {
                Ok("csv") => OutputState::Csv {
                    header_written: OnceCell::new(),
                    label_names: L::names(),
                    writer: csv::Writer::from_writer(Box::new(stdout())),
                },
                x => {
                    if let Ok(requested) = x {
                        eprintln!("unrecognized value for QPE_FORMAT: {requested:?}");
                    }
                    OutputState::Interactive {
                        readings: Vec::new(),
                        label_names: L::names(),
                    }
                }
            },
            _p: PhantomData,
        }
    }

    pub fn run<R>(&mut self, scale: usize, labels: L, f: impl FnOnce() -> R) -> R
    where
        L: PerfReadingLabels,
    {
        let start_time = SystemTime::now();
        self.counters.reset();
        self.counters.enable();
        let ret = f();
        self.counters.disable();
        self.state
            .push(scale, start_time, &mut self.counters, &mut |dst| {
                labels.values(dst)
            });
        ret
    }
}

pub trait PerfReadingLabels {
    fn names() -> &'static [&'static str];
    fn values(&self, f: &mut dyn FnMut(&str));
}

impl<L> Drop for PerfEvent<L> {
    fn drop(&mut self) {
        self.state.dump_and_reset(&mut self.counters);
    }
}
