//! This example implements a simple pointer chasing benchmark for measuring cache latency.
//! An array of indexes forms a random loop of indirection of a specified size.
//! As the position of each array access is determined by the index loaded in the previous access,
//! the load can only be started after the previous load completes.
//! Therefore, each loop iteration takes roughtly as long as the access latency.

use quick_perf_event::{QuickPerfEvent, TabledFloat, struct_labels};
use rand::{SeedableRng, rng, rngs::SmallRng, seq::SliceRandom};
use std::mem;

struct_labels! {
    struct Labels{
        size:String,
    }
}

fn walk_ptr_loop(steps: usize, size: usize, qpe: &mut QuickPerfEvent<Labels>) {
    let n = size / mem::size_of::<usize>();
    let mut access_sequence: Vec<usize> = (0..n).collect();
    access_sequence.shuffle(&mut SmallRng::from_rng(&mut rng()));
    let mut cycle = vec![0usize; n];
    for i in 0..n {
        cycle[access_sequence[i]] = access_sequence[(i + 1) % n];
    }
    let mut i = 0;
    let mut sum = 0;
    qpe.run(|| {
        for _ in 0..steps {
            sum += i;
            i = access_sequence[i];
        }
    })
    .record(
        steps,
        Labels {
            size: TabledFloat(size as f64).to_string(),
        },
    )
}

fn main() {
    let mut qpe = QuickPerfEvent::new();
    for scale in (12..20).chain((21..30).step_by(2)) {
        walk_ptr_loop(40_000_000, 1 << scale, &mut qpe);
    }
}
