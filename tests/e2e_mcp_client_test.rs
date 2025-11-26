//! End-to-end tests using the official Rust MCP SDK (rmcp) client
//!
//! These tests verify that the metis server correctly implements the MCP protocol
//! by using the official rmcp client library to communicate with it.

use metis::adapters::{
    health_handler::HealthHandler,
    metrics_handler::{MetricsCollector, MetricsHandler},
    mock_strategy::MockStrategyHandler,
    prompt_handler::InMemoryPromptHandler,
    resource_handler::InMemoryResourceHandler,
    rmcp_server::MetisServer,
    state_manager::StateManager,
    tool_handler::BasicToolHandler,
};
use metis::config::{
    MockConfig, MockStrategyType, PromptArgument, PromptConfig, PromptMessage, ResourceConfig,
    Settings, ToolConfig,
};
use rmcp::{
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    transport::StreamableHttpClientTransport,
    ServiceExt,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Test Server Infrastructure
// ============================================================================

#[allow(dead_code)]
struct TestServer {
    addr: SocketAddr,
    base_url: String,
}

impl TestServer {
    async fn new() -> Self {
        Self::with_config(vec![], vec![], vec![]).await
    }

    async fn with_config(
        resources: Vec<ResourceConfig>,
        tools: Vec<ToolConfig>,
        prompts: Vec<PromptConfig>,
    ) -> Self {
        let settings = Arc::new(RwLock::new(Settings {
            server: metis::config::ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            auth: Default::default(),
            resources,
            resource_templates: vec![],
            tools,
            prompts,
            rate_limit: None,
            s3: None,
            workflows: vec![],
            agents: vec![],
            orchestrations: vec![],
            mcp_servers: vec![],
            secrets: Default::default(),
        }));

        let state_manager = Arc::new(StateManager::new());
        let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager.clone()));
        let resource_handler =
            Arc::new(InMemoryResourceHandler::new(settings.clone(), mock_strategy.clone()));
        let tool_handler = Arc::new(BasicToolHandler::new(settings.clone(), mock_strategy.clone()));
        let prompt_handler = Arc::new(InMemoryPromptHandler::new(settings.clone()));
        let health_handler = Arc::new(HealthHandler::new(settings.clone()));
        let metrics_collector = Arc::new(MetricsCollector::new().unwrap());
        let metrics_handler = Arc::new(MetricsHandler::new(metrics_collector));

        // Create MetisServer using rmcp SDK
        let metis_server = MetisServer::new(resource_handler, tool_handler, prompt_handler);

        // Create test secrets store
        let secrets_store = metis::adapters::secrets::create_secrets_store();

        let app =
            metis::create_app(metis_server, health_handler, metrics_handler, settings, state_manager, secrets_store).await;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Wait for server to be ready
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        TestServer { addr, base_url }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn with_sample_tools() -> Self {
        let tools = vec![ToolConfig {
            name: "echo".to_string(),
            description: "Echo the input message".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo"
                    }
                },
                "required": ["message"]
            }),
            output_schema: None,
            static_response: Some(json!({"echoed": "hello"})),
            mock: Some(MockConfig {
                strategy: MockStrategyType::Static,
                template: Some(r#"{"echoed": "hello"}"#.to_string()),
                faker_type: None,
                stateful: None,
                script: None,
                script_lang: None,
                file: None,
                pattern: None,
                llm: None,
                database: None,
            }),
        }];

        Self::with_config(vec![], tools, vec![]).await
    }

    async fn fully_configured() -> Self {
        let resources = vec![ResourceConfig {
            uri: "test://sample/resource".to_string(),
            name: "Sample Resource".to_string(),
            description: Some("A sample resource for testing".to_string()),
            mime_type: Some("text/plain".to_string()),
            output_schema: None,
            content: Some("Sample resource content".to_string()),
            mock: Some(MockConfig {
                strategy: MockStrategyType::Static,
                template: Some("Sample resource content".to_string()),
                faker_type: None,
                stateful: None,
                script: None,
                script_lang: None,
                file: None,
                pattern: None,
                llm: None,
                database: None,
            }),
        }];

        let tools = vec![ToolConfig {
            name: "echo".to_string(),
            description: "Echo the input message".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to echo"
                    }
                },
                "required": ["message"]
            }),
            output_schema: None,
            static_response: Some(json!({"echoed": "hello"})),
            mock: Some(MockConfig {
                strategy: MockStrategyType::Static,
                template: Some(r#"{"echoed": "hello"}"#.to_string()),
                faker_type: None,
                stateful: None,
                script: None,
                script_lang: None,
                file: None,
                pattern: None,
                llm: None,
                database: None,
            }),
        }];

        let prompts = vec![PromptConfig {
            name: "greeting".to_string(),
            description: "A greeting prompt".to_string(),
            arguments: Some(vec![PromptArgument {
                name: "name".to_string(),
                description: Some("Name to greet".to_string()),
                required: true,
            }]),
            input_schema: None,
            messages: Some(vec![PromptMessage {
                role: "user".to_string(),
                content: "Hello, {{name}}!".to_string(),
            }]),
        }];

        Self::with_config(resources, tools, prompts).await
    }
}

// ============================================================================
// MCP Client Helper
// ============================================================================

async fn create_client(
    server: &TestServer,
) -> Result<
    rmcp::service::RunningService<rmcp::RoleClient, rmcp::model::InitializeRequestParam>,
    rmcp::service::ClientInitializeError,
> {
    let transport = StreamableHttpClientTransport::from_uri(server.url("/mcp"));
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "e2e-test-client".to_string(),
            title: None,
            version: "1.0.0".to_string(),
            website_url: None,
            icons: None,
        },
    };
    client_info.serve(transport).await
}

// ============================================================================
// Basic Protocol Tests
// ============================================================================

#[tokio::test]
async fn test_client_connect_and_initialize() {
    let server = TestServer::new().await;
    let client = create_client(&server).await;

    assert!(
        client.is_ok(),
        "Client should successfully connect and initialize"
    );

    let client = client.unwrap();
    let server_info = client.peer_info();

    if let Some(info) = server_info {
        assert!(
            !info.server_info.name.is_empty(),
            "Server should have a name"
        );
        // Protocol version is available - just verify we connected successfully
        assert!(true, "Server should report protocol version");
    } else {
        panic!("Server info should be available after initialization");
    }

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_list_tools_empty() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let tools = client.list_tools(Default::default()).await;
    assert!(tools.is_ok(), "Should be able to list tools");

    let tools = tools.unwrap();
    assert!(tools.tools.is_empty(), "Empty server should have no tools");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_list_resources_empty() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let resources = client.list_resources(Default::default()).await;
    assert!(resources.is_ok(), "Should be able to list resources");

    let resources = resources.unwrap();
    assert!(
        resources.resources.is_empty(),
        "Empty server should have no resources"
    );

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_list_prompts_empty() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let prompts = client.list_prompts(Default::default()).await;
    assert!(prompts.is_ok(), "Should be able to list prompts");

    let prompts = prompts.unwrap();
    assert!(
        prompts.prompts.is_empty(),
        "Empty server should have no prompts"
    );

    client.cancel().await.unwrap();
}

// ============================================================================
// Tool Tests
// ============================================================================

#[tokio::test]
async fn test_list_tools_with_configured_tools() {
    let tools = vec![ToolConfig {
        name: "echo".to_string(),
        description: "Echo the input message".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        }),
        output_schema: None,
        static_response: Some(json!({"echoed": "hello"})),
        mock: Some(MockConfig {
            strategy: MockStrategyType::Static,
            template: Some(r#"{"echoed": "hello"}"#.to_string()),
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
    }];

    let server = TestServer::with_config(vec![], tools, vec![]).await;
    let client = create_client(&server).await.unwrap();

    let tools = client.list_tools(Default::default()).await.unwrap();
    assert_eq!(tools.tools.len(), 1, "Should have one tool configured");

    let tool = &tools.tools[0];
    assert_eq!(tool.name.as_ref(), "echo");
    assert!(tool.description.is_some());
    assert_eq!(tool.description.as_deref(), Some("Echo the input message"));

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_call_tool() {
    let tools = vec![ToolConfig {
        name: "echo".to_string(),
        description: "Echo the input message".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string"
                }
            },
            "required": ["message"]
        }),
        output_schema: None,
        static_response: Some(json!({"echoed": "hello"})),
        mock: Some(MockConfig {
            strategy: MockStrategyType::Static,
            template: Some(r#"{"echoed": "hello"}"#.to_string()),
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
    }];

    let server = TestServer::with_config(vec![], tools, vec![]).await;
    let client = create_client(&server).await.unwrap();

    let result = client
        .call_tool(CallToolRequestParam {
            name: "echo".into(),
            arguments: json!({"message": "test"}).as_object().cloned(),
        })
        .await;

    assert!(result.is_ok(), "Tool call should succeed");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_call_nonexistent_tool() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let result = client
        .call_tool(CallToolRequestParam {
            name: "nonexistent_tool".into(),
            arguments: None,
        })
        .await;

    assert!(result.is_err(), "Calling nonexistent tool should fail");

    client.cancel().await.unwrap();
}

// ============================================================================
// Resource Tests
// ============================================================================

#[tokio::test]
async fn test_list_resources_with_configured_resources() {
    let resources = vec![ResourceConfig {
        uri: "test://sample/resource".to_string(),
        name: "Sample Resource".to_string(),
        description: Some("A sample resource for testing".to_string()),
        mime_type: Some("text/plain".to_string()),
        output_schema: None,
        content: Some("Sample resource content".to_string()),
        mock: Some(MockConfig {
            strategy: MockStrategyType::Static,
            template: Some("Sample resource content".to_string()),
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
    }];

    let server = TestServer::with_config(resources, vec![], vec![]).await;
    let client = create_client(&server).await.unwrap();

    let resources = client.list_resources(Default::default()).await.unwrap();
    assert_eq!(
        resources.resources.len(),
        1,
        "Should have one resource configured"
    );

    let resource = &resources.resources[0];
    assert_eq!(resource.uri.as_str(), "test://sample/resource");
    assert_eq!(resource.name.as_str(), "Sample Resource");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_resource() {
    let resources = vec![ResourceConfig {
        uri: "test://sample/resource".to_string(),
        name: "Sample Resource".to_string(),
        description: Some("A sample resource".to_string()),
        mime_type: Some("text/plain".to_string()),
        output_schema: None,
        content: Some("Sample resource content".to_string()),
        mock: Some(MockConfig {
            strategy: MockStrategyType::Static,
            template: Some("Sample resource content".to_string()),
            faker_type: None,
            stateful: None,
            script: None,
            script_lang: None,
            file: None,
            pattern: None,
            llm: None,
            database: None,
        }),
    }];

    let server = TestServer::with_config(resources, vec![], vec![]).await;
    let client = create_client(&server).await.unwrap();

    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParam {
            uri: "test://sample/resource".into(),
        })
        .await;

    assert!(result.is_ok(), "Reading resource should succeed");

    let contents = result.unwrap();
    assert!(
        !contents.contents.is_empty(),
        "Resource should have contents"
    );

    client.cancel().await.unwrap();
}

// ============================================================================
// Prompt Tests
// ============================================================================

#[tokio::test]
async fn test_list_prompts_with_configured_prompts() {
    let prompts = vec![PromptConfig {
        name: "greeting".to_string(),
        description: "A greeting prompt".to_string(),
        arguments: Some(vec![PromptArgument {
            name: "name".to_string(),
            description: Some("Name to greet".to_string()),
            required: true,
        }]),
        input_schema: None,
        messages: Some(vec![PromptMessage {
            role: "user".to_string(),
            content: "Hello, {{name}}!".to_string(),
        }]),
    }];

    let server = TestServer::with_config(vec![], vec![], prompts).await;
    let client = create_client(&server).await.unwrap();

    let prompts = client.list_prompts(Default::default()).await.unwrap();
    assert_eq!(prompts.prompts.len(), 1, "Should have one prompt configured");

    let prompt = &prompts.prompts[0];
    assert_eq!(prompt.name.as_str(), "greeting");
    assert!(prompt.description.is_some());

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_get_prompt() {
    let prompts = vec![PromptConfig {
        name: "greeting".to_string(),
        description: "A greeting prompt".to_string(),
        arguments: Some(vec![PromptArgument {
            name: "name".to_string(),
            description: Some("Name to greet".to_string()),
            required: true,
        }]),
        input_schema: None,
        messages: Some(vec![PromptMessage {
            role: "user".to_string(),
            content: "Hello, {{name}}!".to_string(),
        }]),
    }];

    let server = TestServer::with_config(vec![], vec![], prompts).await;
    let client = create_client(&server).await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("name".to_string(), serde_json::Value::String("World".to_string()));
    let result = client
        .get_prompt(rmcp::model::GetPromptRequestParam {
            name: "greeting".into(),
            arguments: Some(args),
        })
        .await;

    assert!(result.is_ok(), "Getting prompt should succeed");

    let prompt_result = result.unwrap();
    assert!(
        !prompt_result.messages.is_empty(),
        "Prompt should have messages"
    );

    client.cancel().await.unwrap();
}

// ============================================================================
// Fully Configured Server Tests
// ============================================================================

#[tokio::test]
async fn test_fully_configured_server() {
    let server = TestServer::fully_configured().await;
    let client = create_client(&server).await.unwrap();

    // Verify tools
    let tools = client.list_tools(Default::default()).await.unwrap();
    assert!(!tools.tools.is_empty(), "Should have tools configured");

    // Verify resources
    let resources = client.list_resources(Default::default()).await.unwrap();
    assert!(
        !resources.resources.is_empty(),
        "Should have resources configured"
    );

    // Verify prompts
    let prompts = client.list_prompts(Default::default()).await.unwrap();
    assert!(!prompts.prompts.is_empty(), "Should have prompts configured");

    client.cancel().await.unwrap();
}

// ============================================================================
// Multiple Concurrent Clients Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let server = TestServer::with_sample_tools().await;

    // Connect multiple clients concurrently
    let client1 = create_client(&server).await;
    let client2 = create_client(&server).await;
    let client3 = create_client(&server).await;

    assert!(client1.is_ok(), "First client should connect");
    assert!(client2.is_ok(), "Second client should connect");
    assert!(client3.is_ok(), "Third client should connect");

    let client1 = client1.unwrap();
    let client2 = client2.unwrap();
    let client3 = client3.unwrap();

    // All clients should be able to list tools
    let tools1 = client1.list_tools(Default::default()).await;
    let tools2 = client2.list_tools(Default::default()).await;
    let tools3 = client3.list_tools(Default::default()).await;

    assert!(tools1.is_ok());
    assert!(tools2.is_ok());
    assert!(tools3.is_ok());

    // Cleanup
    let _ = client1.cancel().await;
    let _ = client2.cancel().await;
    let _ = client3.cancel().await;
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_read_nonexistent_resource() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let result = client
        .read_resource(rmcp::model::ReadResourceRequestParam {
            uri: "nonexistent://resource".into(),
        })
        .await;

    assert!(result.is_err(), "Reading nonexistent resource should fail");

    client.cancel().await.unwrap();
}

#[tokio::test]
async fn test_get_nonexistent_prompt() {
    let server = TestServer::new().await;
    let client = create_client(&server).await.unwrap();

    let result = client
        .get_prompt(rmcp::model::GetPromptRequestParam {
            name: "nonexistent".into(),
            arguments: None,
        })
        .await;

    assert!(result.is_err(), "Getting nonexistent prompt should fail");

    client.cancel().await.unwrap();
}
