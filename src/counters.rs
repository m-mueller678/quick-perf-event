mod manual_backend;
#[cfg(target_os = "linux")]
mod perf_backend;
mod time_backend;

pub use manual_backend::ManualBackend;
#[cfg(target_os = "linux")]
pub use perf_backend::PerfBackend;
pub use time_backend::TimeBackend;

/// A `CounterBackend` is used by a [`QuickPerfEvent`] to record performance counters.
/// Each `CounterBackend` contains a set of named performance counters.
/// It supports starting, stopping, resetting, and reading counter values and names.
///
/// This crate comes with various implementations.
/// The counter backend used by a default [`QuickPerfEvent`] is [`default_counter_backend`].
pub trait Counters {
    /// Enable counters.
    fn enable(&mut self);
    /// Disable counters.
    fn disable(&mut self);
    /// Reset counters.
    fn reset(&mut self);
    /// Read all counters and append the readings to `dst`.
    fn read(&mut self, dst: &mut Vec<CounterReading>);
    /// Appends the counter names to `dst`.
    ///
    /// Names must be appended in the same order as the values appended by [`read`].
    fn names(&self, dst: &mut dyn FnMut(&str));
}

impl Counters for Box<dyn Counters> {
    fn enable(&mut self) {
        (**self).enable();
    }

    fn disable(&mut self) {
        (**self).disable();
    }

    fn reset(&mut self) {
        (**self).reset();
    }

    fn read(&mut self, dst: &mut Vec<CounterReading>) {
        (**self).read(dst);
    }

    fn names(&self, dst: &mut dyn FnMut(&str)) {
        (**self).names(dst);
    }
}

impl<A: Counters, B: Counters> Counters for (A, B) {
    /// Enables A, then B
    fn enable(&mut self) {
        self.0.enable();
        self.1.enable();
    }

    /// Disables B, then A (reverse starting order)
    fn disable(&mut self) {
        self.1.disable();
        self.0.disable();
    }

    fn reset(&mut self) {
        self.0.reset();
        self.1.reset();
    }

    fn read(&mut self, dst: &mut Vec<CounterReading>) {
        self.0.read(dst);
        self.1.read(dst);
    }

    fn names(&self, dst: &mut dyn FnMut(&str)) {
        self.0.names(dst);
        self.1.names(dst);
    }
}

/// Construct a default [`CounterBackend`] from environment variables.
///
/// The exact set of counters it includes is subject to change.
/// Currently, it consists of a [`TimeBackEnd`] and a default [`PerfBackEnd`].
pub fn counters_from_env() -> Box<dyn Counters> {
    if let Some(manual) = ManualBackend::from_env() {
        return Box::new((manual, TimeBackend::new()));
    }
    #[cfg(target_os = "linux")]
    return Box::new((TimeBackend::new(), PerfBackend::new()));
    #[cfg(not(target_os = "linux"))]
    return Box::new(TimeBackend::new());
}

/// A reading of a performance counter.
pub struct CounterReading {
    /// The value to report to the user
    pub value: f64,
    /// if `true`, the reading was multiplexed and may therefore be less reliable.
    /// Setting this causes some output formats to include a warning.
    /// This is currently only used by the [`PerfBackEnd`]
    pub multiplexed: bool,
    /// if `true`, the reading should be divided by the `scale` parameter of the benchmark.
    pub enable_scale: bool,
}

impl CounterReading {
    pub(crate) fn scaled_value(&self, scale: usize) -> f64 {
        if self.enable_scale {
            self.value / scale as f64
        } else {
            self.value
        }
    }
}

pub(crate) fn count_counters(counters: &dyn Counters) -> usize {
    let mut num_counters = 0;
    counters.names(&mut |_| {
        num_counters += 1;
    });
    num_counters
}
