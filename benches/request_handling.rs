use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use metis::adapters::{
    mcp_protocol_handler::{McpProtocolHandler, JsonRpcRequest},
    health_handler::HealthHandler,
    metrics_handler::{MetricsCollector, MetricsHandler},
    resource_handler::InMemoryResourceHandler,
    tool_handler::BasicToolHandler,
    prompt_handler::InMemoryPromptHandler,
    logging_handler::LoggingHandler,
    mock_strategy::MockStrategyHandler,
    state_manager::StateManager,
};
use metis::config::Settings;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_app() -> (Arc<McpProtocolHandler>, Arc<HealthHandler>, Arc<MetricsHandler>) {
    let settings = Arc::new(RwLock::new(Settings {
        server: metis::config::ServerSettings {
            host: "127.0.0.1".to_string(),
            port: 3000,
        },
        auth: Default::default(),
        resources: vec![],
        tools: vec![],
        prompts: vec![],
    }));

    let state_manager = Arc::new(StateManager::new());
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));
    let resource_handler = Arc::new(InMemoryResourceHandler::new(settings.clone(), mock_strategy.clone()));
    let tool_handler = Arc::new(BasicToolHandler::new(settings.clone(), mock_strategy.clone()));
    let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
    let logging_handler = Arc::new(LoggingHandler::new());
    let health_handler = Arc::new(HealthHandler::new(settings.clone()));
    let metrics_collector = Arc::new(MetricsCollector::new().unwrap());
    let metrics_handler = Arc::new(MetricsHandler::new(metrics_collector));

    let protocol_handler = Arc::new(McpProtocolHandler::new(
        resource_handler,
        tool_handler,
        prompt_handler,
        logging_handler,
    ));

    (protocol_handler, health_handler, metrics_handler)
}

fn benchmark_mcp_initialize(c: &mut Criterion) {
    let (protocol_handler, _, _) = create_test_app();
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "benchmark",
                "version": "1.0.0"
            }
        })),
        id: Some(json!(1)),
    };

    c.bench_function("mcp_initialize", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                protocol_handler.handle_request(black_box(request.clone())).await
            })
        });
    });
}

fn benchmark_mcp_ping(c: &mut Criterion) {
    let (protocol_handler, _, _) = create_test_app();
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    c.bench_function("mcp_ping", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                protocol_handler.handle_request(black_box(request.clone())).await
            })
        });
    });
}

fn benchmark_mcp_resources_list(c: &mut Criterion) {
    let (protocol_handler, _, _) = create_test_app();
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "resources/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    c.bench_function("mcp_resources_list", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                protocol_handler.handle_request(black_box(request.clone())).await
            })
        });
    });
}

fn benchmark_request_throughput(c: &mut Criterion) {
    let (protocol_handler, _, _) = create_test_app();
    
    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(1));
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    group.bench_function("requests_per_second", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                protocol_handler.handle_request(black_box(request.clone())).await
            })
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_mcp_initialize,
    benchmark_mcp_ping,
    benchmark_mcp_resources_list,
    benchmark_request_throughput
);
criterion_main!(benches);
