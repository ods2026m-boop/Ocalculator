//! Matrix benchmarks (Phase 3): mul / determinant / inverse.
//!
//! Run with `cargo bench -p ocalc-matrix`.

use std::time::Instant;

use ocalc_matrix::Matrix;

fn bench(name: &str, iters: usize, mut f: impl FnMut()) {
    for _ in 0..10 {
        f();
    }
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    let el = t.elapsed();
    println!(
        "matrix/{name}: {iters} iters in {el:?} ({:.1} ns/op)",
        el.as_nanos() as f64 / iters as f64
    );
}

fn main() {
    println!("== ocalc matrix benchmarks ==");
    let a = Matrix::from_rows(&[
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
        vec![7.0, 8.0, 10.0],
    ])
    .unwrap();
    let b = Matrix::identity(3);

    bench("mul3x3", 20_000, || {
        let _ = a.mul(&b).unwrap();
    });
    bench("det3x3", 20_000, || {
        let _ = a.determinant();
    });
    bench("inverse3x3", 5_000, || {
        let _ = a.inverse().unwrap();
    });
    bench("transpose3x3", 50_000, || {
        let _ = a.transpose();
    });

    let m10 = Matrix::identity(10);
    bench("mul10x10", 2_000, || {
        let _ = m10.mul(&m10).unwrap();
    });
    bench("det10x10", 500, || {
        let _ = m10.determinant();
    });
}
