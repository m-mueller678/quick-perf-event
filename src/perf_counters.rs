use perf_event::{
    Builder, Counter, CounterData,
    events::{Cache, CacheId, CacheOp, CacheResult, Hardware},
};
use std::time::{Duration, Instant};

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
    pub fn new() -> Self {
        let events = std::env::var("QPE_EVENTS");
        let events = events
            .as_deref()
            .unwrap_or("cycle,kcycle,l1-miss,llc-miss,b-miss")
            .split(",");
        Self::with_counter_names(events)
    }

    pub fn with_counter_names<'a>(counters: impl IntoIterator<Item = &'a str>) -> Self {
        let counters = counters
            .into_iter()
            .filter_map(|name| {
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
                    "b-miss" => Builder::new(Hardware::BRANCH_MISSES),
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

    pub fn with_counters(counters: impl IntoIterator<Item = (Option<String>, Counter)>) -> Self {
        PerfCounters {
            counters: counters.into_iter().collect(),
            time: Ok(Duration::ZERO),
        }
    }

    pub fn enable(&mut self) {
        for x in &mut self.counters {
            x.1.enable().unwrap();
        }
        let Ok(duration) = self.time else {
            panic!("perf already enabled")
        };
        self.time = Err(Instant::now() - duration);
    }
    pub fn disable(&mut self) {
        for x in &mut self.counters {
            x.1.disable().unwrap();
        }
        let Err(start) = self.time else {
            panic!("perf already disabled")
        };
        self.time = Ok(Instant::now() - start);
    }
    pub fn reset(&mut self) {
        assert!(self.time.is_ok(), "perf reset while enabled");
        for x in &mut self.counters {
            x.1.reset().unwrap();
        }
        self.time = Ok(Duration::ZERO)
    }

    pub fn read_counters(&mut self) -> PerfReading {
        PerfReading {
            duration: self.time.expect("perf read while enabled"),
            counters: self
                .counters
                .iter_mut()
                .filter(|x| x.0.is_some())
                .map(|(_, counter)| counter.read_full().unwrap())
                .collect(),
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.counters.iter().filter_map(|x| x.0.as_deref())
    }
}

pub struct PerfReading {
    pub duration: Duration,
    pub counters: Vec<CounterData>,
}
