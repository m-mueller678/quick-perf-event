use std::time::{Duration, Instant};

use crate::counters::{CounterBackend, CounterReading};

/// A counter that records the duration of time it is enabled for.
///
/// The counter is named `time`.
pub struct TimeBackend {
    time: Result<Duration, Instant>,
}

impl Default for TimeBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeBackend {
    pub fn new() -> Self {
        Self {
            time: Ok(Duration::ZERO),
        }
    }
}

impl CounterBackend for TimeBackend {
    fn enable(&mut self) {
        let Ok(duration) = self.time else {
            panic!("already enabled")
        };
        self.time = Err(Instant::now() - duration);
    }

    fn disable(&mut self) {
        let Err(start) = self.time else {
            panic!("already disabled")
        };
        self.time = Ok(Instant::now() - start);
    }

    fn reset(&mut self) {
        assert!(self.time.is_ok(), "perf while enabled");
        self.time = Ok(Duration::ZERO)
    }

    fn read(&mut self, dst: &mut Vec<CounterReading>) {
        dst.push(CounterReading {
            value: self.time.expect("perf read while enabled").as_secs_f64(),
            multiplexed: false,
            enable_scale: false,
        });
    }

    fn names(&self, f: &mut dyn FnMut(&str)) {
        f("time")
    }
}
