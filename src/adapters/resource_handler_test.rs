use super::resource_handler::InMemoryResourceHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, ResourceConfig};
use crate::domain::ResourcePort;
use std::sync::Arc;

#[tokio::test]
async fn test_get_resource_static() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ResourceConfig {
        uri: "file:///test.txt".to_string(),
        name: "Test".to_string(),
        description: None,
        mime_type: Some("text/plain".to_string()),
        content: Some("Static Content".to_string()),
        mock: None,
    }];
    let handler = InMemoryResourceHandler::new(config, mock_strategy);

    let result = handler.get_resource("file:///test.txt").await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["text"], "Static Content");
}

#[tokio::test]
async fn test_get_resource_mock() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ResourceConfig {
        uri: "file:///mock.txt".to_string(),
        name: "Mock".to_string(),
        description: None,
        mime_type: Some("text/plain".to_string()),
        content: None,
        mock: Some(MockConfig {
            strategy: MockStrategyType::Template,
            template: Some("Hello, {{ name | default(value=\"\") }}!".to_string()),
            faker_type: None,
            stateful: None,
        }),
    }];
    let handler = InMemoryResourceHandler::new(config, mock_strategy);

    // Note: ResourceHandler currently doesn't pass args to mock strategy for resources,
    // so it will render with empty context.
    let result = handler.get_resource("file:///mock.txt").await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value["text"], "Hello, !");
}

#[tokio::test]
async fn test_get_resource_not_found() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let handler = InMemoryResourceHandler::new(vec![], mock_strategy);

    let result = handler.get_resource("file:///unknown.txt").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_resources() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![
        ResourceConfig {
            uri: "file:///1.txt".to_string(),
            name: "One".to_string(),
            description: None,
            mime_type: Some("text/plain".to_string()),
            content: None,
            mock: None,
        },
        ResourceConfig {
            uri: "file:///2.txt".to_string(),
            name: "Two".to_string(),
            description: None,
            mime_type: Some("text/plain".to_string()),
            content: None,
            mock: None,
        },
    ];
    let handler = InMemoryResourceHandler::new(config, mock_strategy);

    let result = handler.list_resources().await;
    assert!(result.is_ok());
    let list = result.unwrap();
    assert_eq!(list.len(), 2);
}
