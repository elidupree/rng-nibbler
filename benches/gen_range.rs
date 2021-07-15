use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::distributions::{Distribution, Uniform};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rand_pcg::Pcg64Mcg;
use rng_nibbler::{BitstreamExt, RngBitstream};

#[doc(hidden)]
pub const TEST_RANGE_SIZES: &'static [u64] = &[
    1,
    2,
    3,
    4,
    5,
    7,
    8,
    9,
    15,
    16,
    17,
    (1 << 31) - 1,
    1 << 31,
    (1 << 31) + 1,
    (1 << 62) + 1,
    (1 << 63) - 1,
];

fn gen_range(c: &mut Criterion) {
    let mut chacha = ChaChaRng::seed_from_u64(0);
    let mut chacha_bitstream = RngBitstream::new(ChaChaRng::seed_from_u64(0));
    let mut pcg = Pcg64Mcg::seed_from_u64(0);
    let mut pcg_bitstream = RngBitstream::new(Pcg64Mcg::seed_from_u64(0));
    let mut group = c.benchmark_group("gen_range");
    for &range_size in TEST_RANGE_SIZES {
        group.bench_with_input(
            BenchmarkId::new("ChaChaRng", range_size),
            &range_size,
            |b, &range_size| b.iter(|| Uniform::new(0, range_size).sample(&mut chacha)),
        );
        group.bench_with_input(
            BenchmarkId::new("Pcg64Mcg", range_size),
            &range_size,
            |b, &range_size| b.iter(|| Uniform::new(0, range_size).sample(&mut pcg)),
        );
    }
    group.finish();
}

criterion_group!(benches, gen_range);
criterion_main!(benches);
