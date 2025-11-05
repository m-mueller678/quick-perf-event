use quick_perf_event::{from_env, struct_labels};
use std::env;

struct_labels! {
    struct Labels {
        dataset: String,
        operation: &'static str,
    }
}

fn main() {
    let mut perf = from_env::<Labels>();
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
