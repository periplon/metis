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
        file: None,
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
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
        file: None,
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
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
        file: None,
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
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
        file: None,
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    // Fallback to Lorem
    assert!(value.is_string());
    let text = value.as_str().unwrap();
    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_generate_script() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
            let name = input.name;
            "Hello, " + name + "!"
        "#.to_string()),
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };
    let args = json!({ "name": "Script" });

    let result = handler.generate(&config, Some(&args)).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, "Hello, Script!");
}

#[tokio::test]
async fn test_generate_script_lua() {
    use crate::config::ScriptLang;
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
            return "Hello, " .. input.name .. "!"
        "#.to_string()),
        script_lang: Some(ScriptLang::Lua),
        llm: None,
        database: None,
        faker_schema: None,
    };
    let args = json!({ "name": "Lua" });

    let result = handler.generate(&config, Some(&args)).await;
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, "Hello, Lua!");
}

#[tokio::test]
async fn test_generate_script_js() {
    use crate::config::ScriptLang;
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
            "Hello, " + input.name + "!";
        "#.to_string()),
        script_lang: Some(ScriptLang::Js),
        llm: None,
        database: None,
        faker_schema: None,
    };
    let args = json!({ "name": "JS" });

    let result = handler.generate(&config, Some(&args)).await;
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, "Hello, JS!");
}

#[tokio::test]
async fn test_generate_script_python() {
    use crate::config::ScriptLang;
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
output = "Hello, " + input["name"] + "!"
        "#.to_string()),
        script_lang: Some(ScriptLang::Python),
        llm: None,
        database: None,
        faker_schema: None,
    };
    let args = json!({ "name": "Python" });

    let result = handler.generate(&config, Some(&args)).await;
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, "Hello, Python!");
}

#[tokio::test]
async fn test_generate_pattern_basic() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Pattern,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: Some(r"ID-\d\d\d\d".to_string()),
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    let text = value.as_str().unwrap();
    // Should be "ID-" followed by 4 digits
    assert!(text.starts_with("ID-"));
    assert_eq!(text.len(), 7);
    assert!(text[3..].chars().all(|c| c.is_ascii_digit()));
}

#[tokio::test]
async fn test_generate_pattern_character_class() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Pattern,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: Some(r"[abc][0-9]".to_string()),
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    let text = value.as_str().unwrap();
    assert_eq!(text.len(), 2);
    assert!(['a', 'b', 'c'].contains(&text.chars().next().unwrap()));
    assert!(text.chars().nth(1).unwrap().is_ascii_digit());
}

#[tokio::test]
async fn test_generate_pattern_repetition() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Pattern,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: Some(r"x{5}".to_string()),
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    let text = value.as_str().unwrap();
    assert_eq!(text, "xxxxx");
}

#[tokio::test]
async fn test_generate_pattern_hex() {
    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Pattern,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: Some(r"\x\x\x\x".to_string()),
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    let text = value.as_str().unwrap();
    assert_eq!(text.len(), 4);
    assert!(text.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_generate_file_random() {
    use crate::config::FileConfig;
    use std::io::Write;

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("metis_test_data.json");
    let mut file = std::fs::File::create(&test_file).unwrap();
    writeln!(file, r#"[{{"id": 1}}, {{"id": 2}}, {{"id": 3}}]"#).unwrap();

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::File,
        template: None,
        faker_type: None,
        stateful: None,
        file: Some(FileConfig {
            path: test_file.to_string_lossy().to_string(),
            selection: "random".to_string(),
        }),
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    // Should be one of the objects
    assert!(value.is_object());
    let id = value.get("id").unwrap().as_i64().unwrap();
    assert!((1..=3).contains(&id));

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[tokio::test]
async fn test_generate_file_sequential() {
    use crate::config::FileConfig;
    use std::io::Write;

    // Create a temporary test file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("metis_test_sequential.json");
    let mut file = std::fs::File::create(&test_file).unwrap();
    writeln!(file, r#"[{{"id": 1}}, {{"id": 2}}, {{"id": 3}}]"#).unwrap();

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::File,
        template: None,
        faker_type: None,
        stateful: None,
        file: Some(FileConfig {
            path: test_file.to_string_lossy().to_string(),
            selection: "sequential".to_string(),
        }),
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    // First call should return id: 1
    let result1 = handler.generate(&config, None).await.unwrap();
    assert_eq!(result1.get("id").unwrap().as_i64().unwrap(), 1);

    // Second call should return id: 2
    let result2 = handler.generate(&config, None).await.unwrap();
    assert_eq!(result2.get("id").unwrap().as_i64().unwrap(), 2);

    // Third call should return id: 3
    let result3 = handler.generate(&config, None).await.unwrap();
    assert_eq!(result3.get("id").unwrap().as_i64().unwrap(), 3);

    // Fourth call should wrap around to id: 1
    let result4 = handler.generate(&config, None).await.unwrap();
    assert_eq!(result4.get("id").unwrap().as_i64().unwrap(), 1);

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

#[tokio::test]
async fn test_generate_file_jsonlines() {
    use crate::config::FileConfig;
    use std::io::Write;

    // Create a temporary test file with JSON Lines format
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("metis_test_jsonlines.jsonl");
    let mut file = std::fs::File::create(&test_file).unwrap();
    writeln!(file, r#"{{"name": "Alice"}}"#).unwrap();
    writeln!(file, r#"{{"name": "Bob"}}"#).unwrap();
    writeln!(file, r#"{{"name": "Charlie"}}"#).unwrap();

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::File,
        template: None,
        faker_type: None,
        stateful: None,
        file: Some(FileConfig {
            path: test_file.to_string_lossy().to_string(),
            selection: "first".to_string(),
        }),
        pattern: None,
        script: None,
        script_lang: None,
        llm: None,
        database: None,
        faker_schema: None,
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value.get("name").unwrap().as_str().unwrap(), "Alice");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}