use braise_lexer::tokenize;
use braise_parser::Parser;
use braise_runtime::Runtime;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::{collections::HashMap, fs, sync::Arc};

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

fn full_pipeline_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");

    for (name, content) in load_samples() {
        group.throughput(Throughput::Bytes(content.len() as u64));
        group.bench_with_input(BenchmarkId::new("full", name), &content, |b, content| {
            b.iter(|| {
                let tokens = tokenize(content).unwrap();
                let mut parser = Parser::new(&tokens, content, "bench.braise".to_string());
                let ast = parser.parse().unwrap();

                let recipe_name = &ast.recipes[0].value.name.clone();
                let runtime = Runtime::new(ast, Arc::new(content.to_string()))
                    .with_executor(braise_runtime::StringExecutor::new(true))
                    .with_quiet();
                runtime.execute_recipe(recipe_name, HashMap::new()).unwrap()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, full_pipeline_bench);
criterion_main!(benches);
