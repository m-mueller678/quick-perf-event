use std::time::Duration;

use crate::counters::Counters;

/// A counter that prints messages at start and stop and waits.
///
/// This is intened to allow you to attach an external profiling tool to the running process for measurements.
/// Naturally, the data captured by the external tool will not be available as a counter value.
///
/// How waiting is performed, is configured by the environment variable `QPE_MANUAL`.
/// If it is set to `read`, waiting is performed by reading one line from stdin and discarding it.
/// A warning is issued if the line contains anything but whitespace.
/// Otherwise, the value is parsed as a floating point number.
/// Waiting is performed by sleeping for the given number of seconds.
///
/// To make the start and stop messages reliably detectable, you may specifiy a unique string via `QPE_MANUAL_MARKER`, which will be included in all messages.
pub struct ManualBackend {
    duration: Option<Duration>,
    marker: String,
}

impl ManualBackend {
    pub fn from_env() -> Option<Self> {
        let var = std::env::var("QPE_MANUAL").ok()?;
        let duration = if var == "read" {
            None
        } else {
            Some(Duration::from_secs_f64(
                var.parse().expect("failed to parse duration"),
            ))
        };
        Some(ManualBackend {
            duration,
            marker: std::env::var("QPE_MANUAL_MARKER").unwrap_or_default(),
        })
    }

    fn print_and_wait(&self, msg: &str) {
        println!("\x1b[1;97;105mQPE {msg} \x1b[0m{}", self.marker);
        if let Some(d) = self.duration {
            std::thread::sleep(d);
        } else {
            let mut discard = String::new();
            std::io::stdin().read_line(&mut discard).ok();
            if !discard.trim().is_empty() {
                println!("⚠️ wait line was non-empty")
            }
        }
    }
}

impl Counters for ManualBackend {
    fn enable(&mut self) {
        self.print_and_wait("start");
    }

    fn disable(&mut self) {
        self.print_and_wait("stop ");
    }

    fn reset(&mut self) {}

    fn read(&mut self, _dst: &mut Vec<super::CounterReading>) {}

    fn names(&self, _dst: &mut dyn FnMut(&str)) {}
}
