use braise_lexer::tokenize;
use braise_parser::Parser;
use braise_runtime::{Runtime, StringExecutor};
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

fn execute_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime");

    for (name, content) in load_samples() {
        let tokens = tokenize(&content).unwrap();
        let mut parser = Parser::new(&tokens, &content, "bench.braise".to_string());
        let ast = parser.parse().unwrap();
        let recipe_name = ast.recipes[0].value.name.clone();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("execute", name),
            &(ast, recipe_name),
            |b, (recipes, recipe_name)| {
                b.iter(|| {
                    let runtime = Runtime::new(recipes.clone(), content.clone().into())
                        .with_executor(StringExecutor::new(true))
                        .with_quiet();
                    runtime
                        .execute_recipe(recipe_name, Default::default())
                        .unwrap();
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, execute_bench);
criterion_main!(benches);
