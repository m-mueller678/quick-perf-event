use quick_perf_event::{QuickPerfEvent, struct_labels};

struct_labels! {
    struct Labels{
        operation:&'static str,
    }
}

fn main() {
    let mut perf = QuickPerfEvent::<Labels>::from_env();
    perf.run(|| {
        for i in 0..100_000 {
            std::hint::black_box(i);
        }
    })
    .record(
        100_000,
        Labels {
            operation: "black_box_i",
        },
    );
    perf.run(|| {
        for _ in 0..100_000 {
            std::hint::black_box(());
        }
    })
    .record(
        100_000,
        Labels {
            operation: "black_box",
        },
    );
}
