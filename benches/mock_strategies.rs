use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use metis::adapters::{mock_strategy::MockStrategyHandler, state_manager::StateManager};
use metis::config::{MockConfig, MockStrategyType};
use serde_json::json;
use std::sync::Arc;

fn benchmark_static_strategy(c: &mut Criterion) {
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);
    
    let config = MockConfig {
        strategy: MockStrategyType::Static,
        template: None,
        faker_type: None,
        stateful: None,
        script: None,
        script_lang: None,
        file: None,
        pattern: None,
        llm: None,
        database: None,
    };

    c.bench_function("static_strategy", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                handler.generate(black_box(&config), None).await.unwrap()
            })
        });
    });
}

fn benchmark_template_strategy(c: &mut Criterion) {
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);
    
    let config = MockConfig {
        strategy: MockStrategyType::Template,
        template: Some("Hello, {{ name | default(value='World') }}!".to_string()),
        faker_type: None,
        stateful: None,
        script: None,
        script_lang: None,
        file: None,
        pattern: None,
        llm: None,
        database: None,
    };

    c.bench_function("template_strategy", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let args = json!({"name": "Benchmark"});
                handler.generate(black_box(&config), Some(&args)).await.unwrap()
            })
        });
    });
}

fn benchmark_random_strategy(c: &mut Criterion) {
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);
    
    let config = MockConfig {
        strategy: MockStrategyType::Random,
        template: None,
        faker_type: Some("name".to_string()),
        stateful: None,
        script: None,
        script_lang: None,
        file: None,
        pattern: None,
        llm: None,
        database: None,
    };

    c.bench_function("random_strategy", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                handler.generate(black_box(&config), None).await.unwrap()
            })
        });
    });
}

fn benchmark_pattern_strategy(c: &mut Criterion) {
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);
    
    let config = MockConfig {
        strategy: MockStrategyType::Pattern,
        template: None,
        faker_type: None,
        stateful: None,
        script: None,
        script_lang: None,
        file: None,
        pattern: Some("ID-\\d\\d\\d\\d-\\w\\w\\w\\w".to_string()),
        llm: None,
        database: None,
    };

    c.bench_function("pattern_strategy", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                handler.generate(black_box(&config), None).await.unwrap()
            })
        });
    });
}

fn benchmark_script_strategy(c: &mut Criterion) {
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);
    
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        script: Some("let x = 10; let y = 20; #{ \"sum\": x + y }".to_string()),
        script_lang: None,
        file: None,
        pattern: None,
        llm: None,
        database: None,
    };

    c.bench_function("script_strategy", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                handler.generate(black_box(&config), None).await.unwrap()
            })
        });
    });
}

fn benchmark_all_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_strategies");
    let state_manager = Arc::new(StateManager::new());
    let handler = MockStrategyHandler::new(state_manager);

    let strategies = vec![
        ("static", MockConfig {
            strategy: MockStrategyType::Static,
            template: None,
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
        ("template", MockConfig {
            strategy: MockStrategyType::Template,
            template: Some("Hello {{ name }}".to_string()),
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
        ("random", MockConfig {
            strategy: MockStrategyType::Random,
            template: None,
            faker_type: Some("name".to_string()),
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
        ("pattern", MockConfig {
            strategy: MockStrategyType::Pattern,
            template: None,
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: Some("ID-\\d\\d\\d".to_string()),
            llm: None,
            database: None,
        }),
    ];

    for (name, config) in strategies {
        group.bench_with_input(BenchmarkId::from_parameter(name), &config, |b, cfg| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    handler.generate(black_box(cfg), None).await.unwrap()
                })
            });
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_static_strategy,
    benchmark_template_strategy,
    benchmark_random_strategy,
    benchmark_pattern_strategy,
    benchmark_script_strategy,
    benchmark_all_strategies
);
criterion_main!(benches);
