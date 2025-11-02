# quick_perf_event

## Quick Perf Event

This crate provides a lightweight framework for measuring and reporting performance
counters across labeled workloads or benchmarks.
The output format can be controlled using the `QPE_FORMAT` environment variable:

- **`QPE_FORMAT=live`** (default) - Outputs a **live table**, designed for development and
  debugging. Each result is printed as soon as it is available, with compact,
  fixed-width, line-wrapped cells to fit many columns in narrow terminals.
  If the requested columns still do not fit, table rows are line wrapped as well.
- **`QPE_FORMAT=md`** - Generates a **Markdown table** after all runs have completed,
  choosing column widths automatically for clean, publication-ready output.
- **`QPE_FORMAT=csv`** - Streams results as **CSV** records to stdout, suitable for
  further processing.

### Example
This benchmark measures computing the sum of an iterator.
```rust
use quick_perf_event::QuickPerfEvent;

fn main() {
    let mut perf = QuickPerfEvent::<str>::new();
    let result = perf.run(|| {
        // Code to benchmark
        (0..1_000_000_000).map(std::hint::black_box).sum::<u64>();
    });
    result.record(1_000_000_000, "sum");
}
```
The results show that it takes about 1 CPU cycle to process each number.
Practically no branch or cache misses are encountered.
In total, the benchmark took around 0.2 seconds to run.
```
failed to create counter "kcycle": Permission denied (os error 13)
┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
│  label  │  time   │  scale  │  cycle  │ l1-miss │llc-miss │ br-miss │
├─────────┼─────────┼─────────┼─────────┼─────────┼─────────┼─────────┤
│   sum   │   0.194 │   1.0 G │   1.007 │ 364.0 n │ 166.0 n │ 345.0 n │
└─────────┴─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘
```

## System Configuration
Note that the example above was unable to record the number of CPU cycles spent in the kernel (`kcycle`).
If you run into similar issues, you probably need to configure `perf_event_paranoid`.
You can change this value until the next reboot using `sudo sysctl -w kernel.perf_event_paranoid=0` or permanently by adding `kernel.perf_event_paranoid = 0` to `/etc/sysctl.conf`.
Lower values mean more permissive handling.
See [`man 2 perf_event_open`](https://www.man7.org/linux/man-pages/man2/perf_event_open.2.html) for what the different restriction levels mean.

### Usage
To start benchmarking, you first need a [`QuickPerfEvent`] object.
[`QuickPerfEvent`] manages both recording and reporting of benchmarks.
You may configure the set of performance counters using either the environment variable `QPE_EVENTS` or [`with_counters`](QuickPerfEvent::with_counters).
For basic usage, you should prefer `QPE_EVENTS`.
For example, to count CPU cycles and branch misses, set it to `cycles,br-miss`.
For an up-to-date list of supported values see the implementation of [`with_counter_names`](PerfCounters::with_counter_names).
If your program is multi-threaded, construct [`QuickPerfEvent`] **before spawning threads** to ensure counts include other threads.

Now that you have a [`QuickPerfEvent`] object, you may start taking measurements using its [`run`](QuickPerfEvent::run) method.
After each run, you **must** call [`record`](PerfReading::record) on the returned value to log the measurement.
The [`record`](PerfReading::record) method takes two parameters:

- **`scale`** - a normalization factor (e.g. number of iterations).
  All performance counters are divided by this value, producing results
  such as *branch misses per operation* or *cycles per iteration*.
  Note that the time column is not normalized.
  It reports the absolute amount of time elapsed over the measurement.
  Dividing this by scale would be misleading when multiple threads are involved.
  If you want a measure of time spent per operation, consider using the task clock counter `t-clock`.

- **`labels`** - metadata describing the measurement.
  This can be:
  - the unit type `()` (no labels),
  - a string `&str` (single label),
  - or a user-defined struct implementing [`Labels`].

## Environment Variables
Quick Perf Event can be configured using various environment variables.
- **`QPE_FORMAT`** - set the output format, see above.
- **`QPE_EVENTS`** - set the counters recorded by a default [PerfCounters] instance.
- **`QPE_LINE_LEN`** - override the line length used for line wrapping live tables. If not set, terminal size is detected automatically.

## Acknowledgements
This crate is heavily inspired by [the C++ header only library](https://github.com/viktorleis/perfevent).

License: MIT OR Apache-2.0
