use std::time::Duration;

use crate::counters::Counters;

/// A counter that prints messages at start and stop and waits.
///
/// This is intened to allow you to attach an external profiling tool to the running process for measurements.
/// Naturally, the data captured by the external tool will not be available as a counter value.
///
/// By default, waiting is performed by reading one line from stdin and discarding it.
/// A warning is issued if the line contains anything but whitespace.
/// Waiting may also be performed by sleeping for a fixed duration.
/// To do so, set `QPE_MANUAL_SLEEP` to the desired duration in seconds.
/// In this case, `stdin` is not interacted with.
///
/// To make the start and stop messages reliably detectable, you may specifiy a unique string via `QPE_MANUAL_MARKER`, which will be included in all messages.
pub struct ManualBackend {
    duration: Option<Duration>,
    marker: String,
}

impl ManualBackend {
    pub fn from_env() -> Self {
        let duration = std::env::var("QPE_MANUAL_SLEEP")
            .ok()
            .map(|x| Duration::from_secs_f64(x.parse().expect("failed to parse duration")));
        ManualBackend {
            duration,
            marker: std::env::var("QPE_MANUAL_MARKER").unwrap_or_default(),
        }
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
