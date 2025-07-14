use braise_lexer::tokenize;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::fs;

const BENCH_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bench_samples");

fn load_samples() -> Vec<(String, String)> {
    ["small", "medium", "large"]
        .iter()
        .map(|name| {
            let path = format!("{BENCH_DIR}/{name}.braise");
            let content = fs::read_to_string(&path).unwrap();
            (name.to_string(), content)
        })
        .collect()
}

fn tokenize_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    for (name, content) in load_samples() {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("tokenize", name),
            &content,
            |b, content| b.iter(|| tokenize(content).unwrap()),
        );
    }

    group.finish();
}

criterion_group!(benches, tokenize_bench);
criterion_main!(benches);
