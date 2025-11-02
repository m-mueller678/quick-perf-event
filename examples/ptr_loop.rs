use quick_perf_event::{PerfEvent, TabledFloat, struct_labels};
use rand::{SeedableRng, rng, rngs::SmallRng, seq::SliceRandom};
use std::mem;

struct_labels! {
    struct Labels{
        size:String,
    }
}

fn walk_ptr_loop(steps: usize, size: usize, qpe: &mut PerfEvent<Labels>) {
    let n = size / mem::size_of::<usize>();
    let mut access_sequence: Vec<usize> = (0..n).collect();
    access_sequence.shuffle(&mut SmallRng::from_rng(&mut rng()));
    let mut cycle = vec![0usize; n];
    for i in 0..n {
        cycle[access_sequence[i]] = access_sequence[(i + 1) % n];
    }
    let mut i = 0;
    let mut sum = 0;
    qpe.run(
        steps,
        Labels {
            size: TabledFloat(size as f64).to_string(),
        },
        || {
            for _ in 0..steps {
                sum += i;
                i = access_sequence[i];
            }
        },
    )
}

fn main() {
    let mut qpe = PerfEvent::new();
    for scale in (12..20).chain((21..30).step_by(2)) {
        walk_ptr_loop(40_000_000, 1 << scale, &mut qpe);
    }
}
