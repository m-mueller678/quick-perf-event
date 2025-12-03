use super::{CounterReading, Counters};
use perf_event::{
    Builder, Counter,
    events::{Cache, CacheId, CacheOp, CacheResult, Hardware, Software},
};

/// A [`CounterBackend`] containing [`perf_event`] counters.
///
/// Note that this crate uses the `perf-event` crate from the `perf-event2` package, not the `perf-event` package.
///
/// By default, perf counter groups are not used.
/// This means that the provided counters might not all run for the exact same duration due to multiplexing performed by the kernel.
/// See [perf_event] documentation for more details.
/// You may provide your own set of counters using [`with_counters`](Self::with_counters).
pub struct PerfBackend {
    counters: Vec<(Option<String>, Counter, f64)>,
}

impl PerfBackend {
    /// Creates a new [`PerfBackend`] instance using counters listed in `QPE_EVENTS`
    /// or the default set if the variable is not defined.
    pub fn new() -> Self {
        let events = std::env::var("QPE_EVENTS");
        let events = events
            .as_deref()
            .unwrap_or("cycle,kcycle,instr,l1-miss,llc-miss,br-miss,t-clock")
            .split(",");
        Self::with_counter_names(events)
    }

    /// Builds a [`PerfBackend`] instance from a list of event names.
    ///
    /// These event names are not standard names.
    /// They are aliases for counter configurations defined by this crate.
    /// The names are chosen to fit in the output format table without line-wrapping.
    ///
    /// Invalid names and counters that cannot be opened (e.g. due to permission issues) are skipped with a warning message to stderr.
    pub fn with_counter_names<'a>(counters: impl IntoIterator<Item = &'a str>) -> Self {
        let counters = counters
            .into_iter()
            .filter_map(|name| {
                let mut scale = 1.0;

                // Keep this clean. Users are expected to read this match statement
                // to discover available counter names.
                let mut builder = match name {
                    "cycle" => Builder::new(Hardware::CPU_CYCLES),
                    "kcycle" => {
                        let mut builder = Builder::new(Hardware::CPU_CYCLES);
                        builder.exclude_user(true).exclude_kernel(false);
                        builder
                    }
                    "instr" => Builder::new(Hardware::INSTRUCTIONS),
                    "l1-miss" => Builder::new(Cache {
                        which: CacheId::L1D,
                        operation: CacheOp::READ,
                        result: CacheResult::MISS,
                    }),
                    "llc-miss" => Builder::new(Hardware::CACHE_MISSES),
                    "br-miss" => Builder::new(Hardware::BRANCH_MISSES),
                    "t-clock" => {
                        // time is reported by the kernel in nanoseconds, we convert to seconds.
                        scale = 1.0e-9;
                        Builder::new(Software::TASK_CLOCK)
                    }
                    _ => {
                        eprintln!("invalid counter name: {name:?}");
                        return None;
                    }
                };
                builder.inherit(true);
                match builder.build() {
                    Err(e) => {
                        eprintln!("failed to create counter {name:?}: {e}");
                        None
                    }
                    Ok(counter) => Some((Some(name.to_string()), counter, scale)),
                }
            })
            .collect();
        PerfBackend { counters }
    }
    /// Constructs a [`PerfBackend`] instance from a set of counters.
    ///
    /// Each counter may be associated with a name.
    /// Counters without a name will be affected by [`enable`](Self::enable), [`disable`](Self::disable), and [`reset`](Self::reset), but their values will not be recorded.
    /// Unnamed counters are intended to be used with perf counter groups.
    ///
    /// Additionally, each counter is associated with a scale.
    /// The value read from the counter is multiplied with the scale factor before reporting.
    pub fn with_counters(
        counters: impl IntoIterator<Item = (Option<String>, Counter, f64)>,
    ) -> Self {
        PerfBackend {
            counters: counters.into_iter().collect(),
        }
    }
}

impl Counters for PerfBackend {
    fn enable(&mut self) {
        for x in &mut self.counters {
            x.1.enable().unwrap();
        }
    }

    fn disable(&mut self) {
        for x in &mut self.counters {
            x.1.disable().unwrap();
        }
    }

    fn reset(&mut self) {
        for x in &mut self.counters {
            x.1.reset().unwrap();
        }
    }

    fn read(&mut self, dst: &mut Vec<CounterReading>) {
        dst.extend(self.counters.iter_mut().filter(|x| x.0.is_some()).map(
            |(_, counter, scale)| {
                let reading = counter.read_full().unwrap();
                CounterReading {
                    value: reading.count() as f64
                        * *scale
                        * reading.time_enabled().unwrap().as_secs_f64()
                        / reading.time_running().unwrap().as_secs_f64(),
                    multiplexed: reading.time_enabled() != reading.time_running(),
                    enable_scale: true,
                }
            },
        ));
    }

    fn names(&self, dst: &mut dyn FnMut(&str)) {
        for name in self.counters.iter().filter_map(|x| x.0.as_ref()) {
            dst(name);
        }
    }
}
