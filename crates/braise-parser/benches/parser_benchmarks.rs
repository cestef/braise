use braise_parser::Parser;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use lexer::tokenize;
use std::fs;

const BENCH_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bench_samples");

fn load_samples() -> Vec<(String, String)> {
    ["simple", "web", "rust", "complex"]
        .iter()
        .map(|name| {
            let path = format!("{BENCH_DIR}/{name}.braise");
            let content =
                fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {path}"));
            (name.to_string(), content)
        })
        .collect()
}

fn gen_scale_sample(count: usize) -> String {
    (0..count)
        .map(|i| {
            format!(
                r#"recipe "task_{i}" {{
    param count: number = {i}
    if count > 10 {{
        run "echo {i}"
    }}
}}

"#
            )
        })
        .collect()
}

fn tokenize_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenize");

    for (name, content) in load_samples() {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("tokenize", &name),
            &content,
            |b, content| b.iter(|| tokenize(content).unwrap()),
        );
    }

    group.finish();
}

fn parse_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for (name, content) in load_samples() {
        let tokens = tokenize(&content).unwrap();
        group.throughput(Throughput::Elements(tokens.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", &name),
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

fn full_pipeline_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");

    for (name, content) in load_samples() {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::new("full", &name), &content, |b, content| {
            b.iter(|| {
                let tokens = tokenize(content).unwrap();
                let mut parser = Parser::new(&tokens, content, "bench.braise".to_string());
                parser.parse().unwrap()
            })
        });
    }

    group.finish();
}

fn scale_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale");

    for &scale in &[50, 100, 200] {
        let content = gen_scale_sample(scale);

        group.throughput(Throughput::Elements(scale as u64));
        group.bench_with_input(
            BenchmarkId::new("recipes", scale),
            &content,
            |b, content| {
                b.iter(|| {
                    let tokens = tokenize(content).unwrap();
                    let mut parser = Parser::new(&tokens, content, "bench.braise".to_string());
                    parser.parse().unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    tokenize_bench,
    parse_bench,
    full_pipeline_bench,
    scale_bench
);
criterion_main!(benches);
