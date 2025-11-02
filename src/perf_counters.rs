use perf_event::{
    Builder, Counter, CounterData,
    events::{Cache, CacheId, CacheOp, CacheResult, Hardware, Software},
};
use std::time::{Duration, Instant};

/// A wrapper containing [`perf_event`] counters for use in [QuickPerfEvent](crate::QuickPerfEvent).
///
/// Note that this crate uses the `perf-event` crate from the `perf-event2` package, not the `perf-event` package.
///
/// `PerfCounters` provides an ergonomic interface for measuring CPU hardware
/// and software performance events. It supports starting, stopping, resetting,
/// and reading counters as a group, as well as tracking the elapsed wall-clock time.
///
/// By default, perf counter groups are not used.
/// This means that the provided counters might not all run for the exact same duration due to multiplexing performed by the kernel.
/// See [perf_event] documentation for more details.
/// You may provide your own set of counters using [`with_counters`](Self::with_counters).
pub struct PerfCounters {
    counters: Vec<(Option<String>, Counter)>,
    time: Result<Duration, Instant>,
}

impl Default for PerfCounters {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfCounters {
    /// Creates a new [`PerfCounters`] instance using counters listed in `QPE_EVENTS`
    /// or the default set if the variable is not defined.
    pub fn new() -> Self {
        let events = std::env::var("QPE_EVENTS");
        let events = events
            .as_deref()
            .unwrap_or("cycle,kcycle,l1-miss,llc-miss,br-miss")
            .split(",");
        Self::with_counter_names(events)
    }

    /// Builds a [`PerfCounters`] instance from a list of event names.
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
                    "t-clock" => Builder::new(Software::TASK_CLOCK),
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
                    Ok(counter) => Some((Some(name.to_string()), counter)),
                }
            })
            .collect();
        PerfCounters {
            counters,
            time: Ok(Duration::ZERO),
        }
    }
    /// Constructs a [`PerfCounters`] instance from a set of counters.
    ///
    /// Each counter may be associated with a name.
    /// Counters without a name will be affected by [`enable`](Self::enable), [`disable`](Self::disable), and [`reset`](Self::reset), but their values will not be recorded.
    /// Unnamed counters are intended to be used with perf counter groups.
    pub fn with_counters(counters: impl IntoIterator<Item = (Option<String>, Counter)>) -> Self {
        PerfCounters {
            counters: counters.into_iter().collect(),
            time: Ok(Duration::ZERO),
        }
    }

    /// Enables all counters and starts timing.
    ///
    /// Panics if counters are already enabled.
    pub fn enable(&mut self) {
        for x in &mut self.counters {
            x.1.enable().unwrap();
        }
        let Ok(duration) = self.time else {
            panic!("perf already enabled")
        };
        self.time = Err(Instant::now() - duration);
    }

    /// Disables all counters and records elapsed time.
    ///
    /// Panics if counters are already disabled.
    pub fn disable(&mut self) {
        for x in &mut self.counters {
            x.1.disable().unwrap();
        }
        let Err(start) = self.time else {
            panic!("perf already disabled")
        };
        self.time = Ok(Instant::now() - start);
    }

    /// Resets all counters and clears accumulated time.
    ///
    /// Panics if counters are currently enabled.
    pub fn reset(&mut self) {
        assert!(self.time.is_ok(), "perf reset while enabled");
        for x in &mut self.counters {
            x.1.reset().unwrap();
        }
        self.time = Ok(Duration::ZERO)
    }

    /// Reads all counters and returns a [`PerfCountersReading`] snapshot.
    ///
    /// If this instance was created via [`with_counters`](Self::with_counters), only values of named counters are returned.
    ///
    /// Panics if counters are currently enabled.
    pub fn read_counters(&mut self) -> PerfCountersReading {
        PerfCountersReading {
            duration: self.time.expect("perf read while enabled"),
            counters: self
                .counters
                .iter_mut()
                .filter(|x| x.0.is_some())
                .map(|(_, counter)| counter.read_full().unwrap())
                .collect(),
        }
    }

    /// Returns an iterator over the counter names in this set.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.counters.iter().filter_map(|x| x.0.as_deref())
    }
}

/// A single measurement snapshot returned by [`PerfCounters::read_counters`].
///
/// Contains the total measurement duration and raw counter data for each event.
pub struct PerfCountersReading {
    /// Total time elapsed while the counters were enabled.
    pub duration: Duration,
    /// Collected [`CounterData`] values for each enabled counter.
    /// The values are in the same order as the names returned by [`PerfCounters::names`].
    pub counters: Vec<CounterData>,
}
