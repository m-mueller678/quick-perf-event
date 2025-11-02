use quick_perf_event::{QuickPerfEvent, struct_labels};
use std::env;

struct_labels! {
    struct Labels {
        dataset: String,
        operation: &'static str,
    }
}

fn main() {
    let mut perf = QuickPerfEvent::<Labels>::new();
    let result = perf.run(|| {
        // benchmarked code
    });
    result.record(
        1,
        &Labels {
            operation: "my_op",
            dataset: env::var("DATA").unwrap_or_default(),
        },
    );
}
