# Reconciler Benchmarks

Measured on this host (macOS, Apple Silicon). Each sample reconciles a
~500-node tree built with `rn::vstack` + `rn::label` leaves.

| Benchmark | mean | lower | upper |
|-----------|------|-------|-------|
| `reconcile_500_initial` | 229.65 µs | 228.06 µs | 231.45 µs |
| `reconcile_500_noop`    | 267.80 µs | 265.09 µs | 270.49 µs |

Both are well below the 8 ms ceiling set by Principle IV
(`specs/001-core-framework/plan.md`). The no-op path is slightly slower than
the initial render because the diff walk visits every node to confirm
nothing changed, but the resulting changeset is empty so no `apply` work
runs on the backend side.

Reproduce:

```bash
cargo bench -p rn
```

Criterion stores detailed reports under `target/criterion/`.
