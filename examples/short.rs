use quick_perf_event::from_env;

fn main() {
    let mut perf = from_env::<str>();
    let result = perf.run(|| {
        // Code to benchmark
        (0..1_000_000_000).map(std::hint::black_box).sum::<u64>();
    });
    result.record(1_000_000_000, "sum");
}
