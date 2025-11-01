use quick_perf_event::{PerfEvent, perf_reading_labels};

perf_reading_labels! {
    struct Labels{
        operation:&'static str,
    }
}

fn main() {
    let mut perf = PerfEvent::new();
    perf.run(
        100_000,
        Labels {
            operation: "black_box_i",
        },
        || {
            for i in 0..100_000 {
                std::hint::black_box(i);
            }
        },
    );
    perf.run(
        100_000,
        Labels {
            operation: "black_box",
        },
        || {
            for _ in 0..100_000 {
                std::hint::black_box(());
            }
        },
    );
}
