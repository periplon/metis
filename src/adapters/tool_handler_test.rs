use super::tool_handler::BasicToolHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, ToolConfig, Settings, ServerSettings};
use crate::domain::ToolPort;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_execute_tool_static() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ToolConfig {
        name: "test_tool".to_string(),
        description: "Test Tool".to_string(),
        input_schema: json!({}),
        static_response: Some(json!({ "result": "success" })),
        mock: None,
    }];
    
    let settings = Settings {
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
            resources: vec![],
        tools: config,
        prompts: vec![],
    };
    let handler = BasicToolHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.execute_tool("test_tool", json!({})).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["result"], "success");
}

#[tokio::test]
async fn test_execute_tool_mock() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ToolConfig {
        name: "mock_tool".to_string(),
        description: "Mock Tool".to_string(),
        input_schema: json!({}),
        static_response: None,
        mock: Some(MockConfig {
            strategy: MockStrategyType::Template,
            template: Some("{\"result\": \"{{ name }}\"}".to_string()),
            faker_type: None,
            stateful: None,
            file: None,
            pattern: None,            script: None,
        }),
    }];
    
    let settings = Settings {
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
            resources: vec![],
        tools: config,
        prompts: vec![],
    };
    let handler = BasicToolHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.execute_tool("mock_tool", json!({ "name": "Mocked" })).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["result"], "Mocked");
}

#[tokio::test]
async fn test_execute_tool_echo_fallback() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ToolConfig {
        name: "echo".to_string(),
        description: "Echo".to_string(),
        input_schema: json!({}),
        static_response: None,
        mock: None,
    }];
    
    let settings = Settings {
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
            resources: vec![],
        tools: config,
        prompts: vec![],
    };
    let handler = BasicToolHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.execute_tool("echo", json!({ "msg": "hello" })).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, serde_json::Value::Null);
}

#[tokio::test]
async fn test_execute_tool_not_found() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let settings = Settings {
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
            resources: vec![],
        tools: vec![],
        prompts: vec![],
    };
    let handler = BasicToolHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.execute_tool("unknown", json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_tools() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![
        ToolConfig {
            name: "t1".to_string(),
            description: "d1".to_string(),
            input_schema: json!({}),
            static_response: None,
            mock: None,
        },
        ToolConfig {
            name: "t2".to_string(),
            description: "d2".to_string(),
            input_schema: json!({}),
            static_response: None,
            mock: None,
        },
    ];
    
    let settings = Settings {
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
            resources: vec![],
        tools: config,
        prompts: vec![],
    };
    let handler = BasicToolHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.list_tools().await;
    assert!(result.is_ok());
    let list = result.unwrap();
    assert_eq!(list.len(), 2);
}
