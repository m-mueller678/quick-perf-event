//! This crate provides a lightweight framework for measuring and reporting performance
//! counters across labeled workloads or benchmarks.
//! The output format can be controlled using the `QPE_FORMAT` environment variable:
//!
//! - **`QPE_FORMAT=live`** (default) - Outputs a **live table**, designed for development and
//!   debugging. Each result is printed as soon as it is available, with compact,
//!   fixed-width, line-wrapped cells to fit many columns in narrow terminals.
//!   If the requested columns still do not fit, table rows are line wrapped as well.
//! - **`QPE_FORMAT=md`** - Generates a **Markdown table** after all runs have completed,
//!   choosing column widths automatically for clean, publication-ready output.
//! - **`QPE_FORMAT=csv`** - Streams results as **CSV** records to stdout, suitable for
//!   further processing.
//!
//! # Example
//! This benchmark measures computing the sum of an iterator.
//! ```
#![doc = include_str!("../examples/short.rs")]
//! ```
//! The results show that it takes about 1 CPU cycle to process each number.
//! Practically no branch or cache misses are encountered.
//! In total, the benchmark took around 0.2 seconds to run.
//! ```text
//!failed to create counter "kcycle": Permission denied (os error 13)
//!┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
//!│  label  │  time   │  scale  │  cycle  │ l1-miss │llc-miss │ br-miss │
//!├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
//!│   sum   │   0.194 │   1.0 G │   1.007 │ 364.0 n │ 166.0 n │ 345.0 n │
//!└─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘
//! ```
//!
//! # System Configuration
//! Note that the example above was unable to record the number of CPU cycles spent in the kernel (`kcycle`).
//! If you run into similar issues, you probably need to configure `perf_event_paranoid`.
//! You can change this value until the next reboot using `sudo sysctl -w kernel.perf_event_paranoid=0` or permanently by adding `kernel.perf_event_paranoid = 0` to `/etc/sysctl.conf`.
//! Lower values mean more permissive handling.
//! See [`man 2 perf_event_open`](https://www.man7.org/linux/man-pages/man2/perf_event_open.2.html) for what the different restriction levels mean.
//!
//! # Usage
//! To start benchmarking, you first need a [`QuickPerfEvent`] object.
//! [`QuickPerfEvent`] manages both recording and reporting of benchmarks.
//! You may configure the set of performance counters using either the environment variable `QPE_EVENTS` or [`with_counters`](QuickPerfEvent::with_counters).
//! For basic usage, you should prefer `QPE_EVENTS`.
//! For example, to count CPU cycles and branch misses, set it to `cycles,br-miss`.
//! For an up-to-date list of supported values see the implementation of [`with_counter_names`](PerfCounters::with_counter_names).
//! If your program is multi-threaded, construct [`QuickPerfEvent`] **before spawning threads** to ensure counts include other threads.
//!
//! Now that you have a [`QuickPerfEvent`] object, you may start taking measurements using its [`run`](QuickPerfEvent::run) method.
//! After each run, you **must** call [`record`](PerfReading::record) on the returned value to log the measurement.
//! The [`record`](PerfReading::record) method takes two parameters:
//!
//! - **`scale`** - a normalization factor (e.g. number of iterations).  
//!   All performance counters are divided by this value, producing results
//!   such as *branch misses per operation* or *cycles per iteration*.
//!   Note that the time column is not normalized.
//!   It reports the absolute amount of time elapsed over the measurement.
//!   Dividing this by scale would be misleading when multiple threads are involved.
//!   If you want a measure of time spent per operation, consider using the task clock counter `t-clock`.
//!
//! - **`labels`** - metadata describing the measurement.  
//!   This can be:
//!   - the unit type `()` (no labels),
//!   - a string `&str` (single label),
//!   - or a user-defined struct implementing [`Labels`].
//!
//! # Environment Variables
//! Quick Perf Event can be configured using various environment variables.
//! - **`QPE_FORMAT`** - set the output format, see above.
//! - **`QPE_EVENTS`** - set the counters recorded by a default [PerfCounters] instance.
//! - **`QPE_LINE_LEN`** - override the line length used for line wrapping live tables. If not set, terminal size is detected automatically.
//!
//! # Acknowledgements
//! This crate is heavily inspired by [the C++ header only library](https://github.com/viktorleis/perfevent).

pub mod counters;
pub mod formats;
mod labels;

pub use labels::Labels;

use crate::{
    counters::{Counters, counters_from_env},
    formats::{Format, format_from_env},
};
use std::{borrow::Borrow, marker::PhantomData, time::SystemTime};

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
pub struct QuickPerfEvent<
    L: ?Sized + Labels,
    C: Counters = Box<dyn Counters>,
    F: Format = Box<dyn Format>,
> {
    running: bool,
    counters: C,
    format: F,
    error_printed: bool,
    _p: PhantomData<L>,
}

/// See [`QuickPerfEvent::run`] and the crate level docs.
#[must_use]
pub struct Reading<
    'a,
    L: ?Sized + Labels,
    T = (),
    C: Counters = Box<dyn Counters>,
    F: Format = Box<dyn Format>,
> {
    pe: &'a mut QuickPerfEvent<L, C, F>,
    start_time: SystemTime,
    ret: T,
}

pub struct Running<
    'a,
    L: ?Sized + Labels,
    C: Counters = Box<dyn Counters>,
    F: Format = Box<dyn Format>,
> {
    pe: &'a mut QuickPerfEvent<L, C, F>,
    start_time: SystemTime,
}

/// Create a `QuickPerfEvent` configured from environment variables.
pub fn from_env<L: Labels + ?Sized>() -> QuickPerfEvent<L, Box<dyn Counters>, Box<dyn Format>> {
    QuickPerfEvent::new(counters_from_env(), format_from_env())
}

impl<L: Labels + ?Sized, C: Counters, F: Format> QuickPerfEvent<L, C, F> {
    /// Create a `QuickPerfEvent` with custom performance counters and format.
    ///
    /// For constructing a default instance from environment variables, see [from_env].
    pub fn new(counters: C, format: F) -> Self {
        QuickPerfEvent {
            running: false,
            counters,
            error_printed: false,
            format,
            _p: PhantomData,
        }
    }

    /// Measure the execution of a function.
    ///
    /// This is a shorthand for wrapping the function in [`start`](Self::start) and [`stop`](Running::stop) calls.
    pub fn run<R>(&mut self, f: impl FnOnce() -> R) -> Reading<'_, L, R, C, F> {
        let running = self.start();
        let ret = f();
        running.stop().replace_return_value(ret).0
    }

    /// Start a measurement.
    ///
    /// After running your benchmark, call [`stop`](Running::stop) on the returned value to obtain a [`Reading`]
    pub fn start(&mut self) -> Running<'_, L, C, F> {
        let start_time = SystemTime::now();
        if self.running {
            self.counters.disable();
        }
        self.running = true;
        self.counters.reset();
        self.counters.enable();
        Running {
            pe: self,
            start_time,
        }
    }
}

impl<'a, L: Labels + ?Sized, T, C: Counters, F: Format> Reading<'a, L, T, C, F> {
    /// Records the measured result.
    ///
    /// The `scale` argument normalizes counter values (e.g. per iteration count).
    /// The given `labels` instance supplies the labels for this sample.
    pub fn record(self, scale: usize, labels: impl Borrow<L>) -> T {
        if let Err(e) = self.pe.format.push(
            scale,
            self.start_time,
            &mut self.pe.counters,
            &mut |dst| labels.borrow().values(dst),
            L::names(),
        ) {
            if !self.pe.error_printed {
                self.pe.error_printed = true;
                eprintln!("error recording result: {e}");
            }
        }
        self.ret
    }

    /// Replace the associated return value.
    ///
    /// A [`Reading`] contains an associated return value, which is returned from [`record`].
    /// For a [`Reading`] constructed from [`QuickPerfEvent::run`], this is the return value of the passed in function.
    /// This method can be used to separate this value from the reading, or to combine the reading with a return new value.
    pub fn replace_return_value<U>(self, ret: U) -> (Reading<'a, L, U, C, F>, T) {
        (
            Reading {
                pe: self.pe,
                start_time: self.start_time,
                ret,
            },
            self.ret,
        )
    }
}

impl<'a, L: Labels + ?Sized, C: Counters, F: Format> Running<'a, L, C, F> {
    /// Stop the measurement.
    pub fn stop(self) -> Reading<'a, L, (), C, F> {
        self.pe.counters.disable();
        self.pe.running = false;
        Reading {
            pe: self.pe,
            start_time: self.start_time,
            ret: (),
        }
    }
}

impl<L: Labels + ?Sized, C: Counters, F: Format> Drop for QuickPerfEvent<L, C, F> {
    fn drop(&mut self) {
        if let Err(e) = self.format.dump_and_reset(L::names(), &mut self.counters) {
            if !self.error_printed {
                eprintln!("error finnishing report: {e}");
            }
        }
    }
}

fn visit<T: ?Sized>(counters: &[impl AsRef<T>], dst: &mut dyn FnMut(&T)) {
    for name in counters {
        dst(name.as_ref())
    }
}
