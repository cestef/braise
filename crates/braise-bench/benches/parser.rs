use braise_lexer::tokenize;
use braise_parser::Parser;
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

fn parse_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    for (name, content) in load_samples() {
        let tokens = tokenize(&content).unwrap();
        group.throughput(Throughput::Elements(tokens.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", name),
            &(tokens.as_slice(), content.as_str()),
            |b, (tokens, source)| {
                b.iter(|| {
                    let mut parser = Parser::new(tokens, source, "bench.braise".to_string());
                    parser.parse().unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, parse_bench);
criterion_main!(benches);
