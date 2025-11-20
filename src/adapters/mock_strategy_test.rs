use super::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType};
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_generate_static() {
    let _handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    // Static is not handled by MockStrategyHandler directly in the current implementation 
    // (it's handled by the caller), but if we extended it, we'd test it here.
    // For now, let's test what MockStrategyHandler does: Template and Random.
}

#[tokio::test]
async fn test_generate_template() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Template,
        template: Some("Hello, {{ name }}!".to_string()),
        faker_type: None,
        stateful: None,
    };
    let args = json!({ "name": "World" });

    let result = handler.generate(&config, Some(&args)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, "Hello, World!");
}

#[tokio::test]
async fn test_generate_template_missing_args() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Template,
        template: Some("Hello, {{ name | default(value=\"\") }}!".to_string()),
        faker_type: None,
        stateful: None,
    };
    
    // Tera renders missing variables as empty string by default or errors depending on config. 
    // In our implementation: context.insert(k, v).
    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    // "Hello, !" because name is missing
    assert_eq!(value, "Hello, !");
}

#[tokio::test]
async fn test_generate_random() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Random,
        template: None,
        faker_type: Some("name".to_string()),
        stateful: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.is_string());
    let text = value.as_str().unwrap();
    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_generate_random_unknown_type() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Random,
        template: None,
        faker_type: Some("unknown_type".to_string()),
        stateful: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    // Fallback to Lorem
    assert!(value.is_string());
    let text = value.as_str().unwrap();
    assert!(!text.is_empty());
}
