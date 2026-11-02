//! Reconciler benchmarks (Principle IV — <8ms for 500-node tree).

use criterion::{criterion_group, criterion_main, Criterion};
use rn::{Component, ComponentTree, Reconciler};

/// Build a balanced tree of approximately `n` nodes.
fn build_tree(n: usize) -> ComponentTree {
    fn build(remaining: &mut usize) -> Component {
        if *remaining == 0 {
            return rn::label("leaf");
        }
        *remaining -= 1;
        let child_count = (*remaining).min(4);
        let mut builder = rn::vstack();
        for _ in 0..child_count {
            builder = builder.child(build(remaining));
        }
        builder.build()
    }
    let mut remaining = n;
    ComponentTree::new(rn::window().child(build(&mut remaining)).build())
}

fn bench_reconcile_500_nodes_initial(c: &mut Criterion) {
    c.bench_function("reconcile_500_initial", |b| {
        b.iter(|| {
            let mut rec = Reconciler::new();
            let _ = rec.reconcile(build_tree(500));
        })
    });
}

fn bench_reconcile_500_nodes_noop(c: &mut Criterion) {
    c.bench_function("reconcile_500_noop", |b| {
        let mut rec = Reconciler::new();
        let _ = rec.reconcile(build_tree(500));
        b.iter(|| {
            let _ = rec.reconcile(build_tree(500));
        });
    });
}

criterion_group!(benches, bench_reconcile_500_nodes_initial, bench_reconcile_500_nodes_noop);
criterion_main!(benches);
