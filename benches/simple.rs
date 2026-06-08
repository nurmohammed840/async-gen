mod utils;

use criterion::{criterion_group, criterion_main, Criterion};
use futures::StreamExt;
use std::pin::pin;

const ITER: usize = 10000;

async fn async_gen_sum(iter: usize) -> usize {
    let mut gen = pin!(async_gen::stream! {
        for i in 1..=iter {
            yield i;
        }
    });
    let mut i = 0;
    while let Some(v) = gen.next().await {
        i += v;
    }
    i
}

async fn async_stream_sum(iter: usize) -> usize {
    let mut gen = pin!(async_stream::stream! {
        for i in 1..=iter {
            yield i;
        }
    });
    let mut i = 0;
    while let Some(v) = gen.next().await {
        i += v;
    }
    i
}

pub fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("async_gen", |b| {
        b.iter(|| utils::poll_once(async_gen_sum(ITER)))
    });
    c.bench_function("async_stream", |b| {
        b.iter(|| utils::poll_once(async_stream_sum(ITER)))
    });
}
criterion_group!(benches, simple_benchmark);
criterion_main!(benches);
