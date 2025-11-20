use super::tool_handler::BasicToolHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, ToolConfig};
use crate::domain::ToolPort;
use serde_json::json;
use std::sync::Arc;

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
    let handler = BasicToolHandler::new(config, mock_strategy);

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
        }),
    }];
    let handler = BasicToolHandler::new(config, mock_strategy);

    let result = handler.execute_tool("mock_tool", json!({ "name": "Mocked" })).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    // MockStrategy returns the rendered content. ToolHandler returns it as is.
    // Wait, MockStrategy returns Value. If template renders JSON, it returns JSON Value.
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
    let handler = BasicToolHandler::new(config, mock_strategy);

    let result = handler.execute_tool("echo", json!({ "msg": "hello" })).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["result"]["msg"], "hello");
}

#[tokio::test]
async fn test_execute_tool_not_found() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let handler = BasicToolHandler::new(vec![], mock_strategy);

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
    let handler = BasicToolHandler::new(config, mock_strategy);

    let result = handler.list_tools().await;
    assert!(result.is_ok());
    let list = result.unwrap();
    assert_eq!(list.len(), 2);
}
