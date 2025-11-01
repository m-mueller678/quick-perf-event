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

use crate::perf_counters::{PerfCounters, PerfReading};
use std::{iter, marker::PhantomData, mem, time::SystemTime};

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
