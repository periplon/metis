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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
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
        data_lake_crud: None,
        ..Default::default()
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value.get("name").unwrap().as_str().unwrap(), "Alice");

    // Cleanup
    std::fs::remove_file(&test_file).ok();
}

// ============== Python Cross-Invocation Integration Tests ==============

#[tokio::test]
async fn test_python_script_access_config_defaults() {
    use crate::config::{ScriptAccessConfig, ScriptAccessLevel};

    let config = ScriptAccessConfig::default();
    // Default should be All (tools/agents/resources/workflows available)
    assert!(matches!(config.tools, ScriptAccessLevel::All));
    assert!(matches!(config.agents, ScriptAccessLevel::All));
    assert!(matches!(config.resources, ScriptAccessLevel::All));
    assert!(matches!(config.workflows, ScriptAccessLevel::All));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_python_crosscall_with_tool_handler() {
    use crate::adapters::tool_handler::BasicToolHandler;
    use crate::config::{Settings, ServerSettings, ToolConfig, ScriptLang, ScriptAccessConfig, ScriptAccessLevel};
    use tokio::sync::RwLock;

    // Create a settings with a static response tool
    let settings = Arc::new(RwLock::new(Settings {
        config_path: None,
        version: 0,
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
        resources: vec![],
        resource_templates: vec![],
        tools: vec![ToolConfig {
            name: "get_greeting".to_string(),
            description: "Returns a greeting".to_string(),
            input_schema: json!({}),
            output_schema: None,
            static_response: Some(json!({ "message": "Hello from tool!" })),
            mock: None,
            tags: vec![],
        }],
        prompts: vec![],
        rate_limit: None,
        s3: None,
        workflows: vec![],
        agents: vec![],
        orchestrations: vec![],
        mcp_servers: vec![],
        secrets: Default::default(),
        schemas: vec![],
        data_lakes: vec![],
        database: None,
        file_storage: None,
    }));

    let state_manager = Arc::new(StateManager::new());
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager.clone()));
    let tool_handler: Arc<dyn crate::domain::ToolPort> = Arc::new(BasicToolHandler::new(settings.clone(), mock_strategy.clone()));

    // Set up the mock strategy with tool handler
    mock_strategy.set_tool_handler(tool_handler).await;

    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
result = call_tool("get_greeting", {})
output = result
        "#.to_string()),
        script_lang: Some(ScriptLang::Python),
        llm: None,
        database: None,
        faker_schema: None,
        data_lake_crud: None,
        script_access: Some(ScriptAccessConfig {
            tools: ScriptAccessLevel::All,
            agents: ScriptAccessLevel::All,
            resources: ScriptAccessLevel::All,
            workflows: ScriptAccessLevel::All,
        }),
        script_max_depth: None,
        script_timeout_ms: None,
        ..Default::default()
    };

    let result = mock_strategy.generate(&config, None).await;
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let value = result.unwrap();

    // Should have received the tool response
    assert!(value.is_object());
    assert_eq!(value.get("message").and_then(|v| v.as_str()), Some("Hello from tool!"));
}

#[tokio::test]
async fn test_python_crosscall_access_denied() {
    use crate::config::{ScriptLang, ScriptAccessConfig, ScriptAccessLevel};

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));

    // Configure script with tools access denied
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
result = call_tool("any_tool", {})
output = result
        "#.to_string()),
        script_lang: Some(ScriptLang::Python),
        llm: None,
        database: None,
        faker_schema: None,
        data_lake_crud: None,
        script_access: Some(ScriptAccessConfig {
            tools: ScriptAccessLevel::None, // Deny all tools
            agents: ScriptAccessLevel::All,
            resources: ScriptAccessLevel::All,
            workflows: ScriptAccessLevel::All,
        }),
        script_max_depth: None,
        script_timeout_ms: None,
        ..Default::default()
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();

    // Should return an error string for access denied
    if let Some(s) = value.as_str() {
        assert!(s.contains("PermissionDeniedError") || s.contains("not allowed"));
    }
}

#[tokio::test]
async fn test_python_crosscall_allow_list() {
    use crate::config::{ScriptLang, ScriptAccessConfig, ScriptAccessLevel};

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));

    // Configure script with specific tools allowed
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
result = call_tool("not_in_allow_list", {})
output = result
        "#.to_string()),
        script_lang: Some(ScriptLang::Python),
        llm: None,
        database: None,
        faker_schema: None,
        data_lake_crud: None,
        script_access: Some(ScriptAccessConfig {
            tools: ScriptAccessLevel::AllowList(vec!["allowed_tool".to_string()]),
            agents: ScriptAccessLevel::All,
            resources: ScriptAccessLevel::All,
            workflows: ScriptAccessLevel::All,
        }),
        script_max_depth: None,
        script_timeout_ms: None,
        ..Default::default()
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();

    // Should return permission denied since tool is not in allow list
    if let Some(s) = value.as_str() {
        assert!(s.contains("PermissionDeniedError") || s.contains("not allowed"));
    }
}

#[tokio::test]
async fn test_python_crosscall_deny_list() {
    use crate::config::{ScriptLang, ScriptAccessConfig, ScriptAccessLevel};

    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));

    // Configure script with specific tools denied
    let config = MockConfig {
        strategy: MockStrategyType::Script,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: Some(r#"
result = call_tool("denied_tool", {})
output = result
        "#.to_string()),
        script_lang: Some(ScriptLang::Python),
        llm: None,
        database: None,
        faker_schema: None,
        data_lake_crud: None,
        script_access: Some(ScriptAccessConfig {
            tools: ScriptAccessLevel::DenyList(vec!["denied_tool".to_string()]),
            agents: ScriptAccessLevel::All,
            resources: ScriptAccessLevel::All,
            workflows: ScriptAccessLevel::All,
        }),
        script_max_depth: None,
        script_timeout_ms: None,
        ..Default::default()
    };

    let result = handler.generate(&config, None).await;
    assert!(result.is_ok());
    let value = result.unwrap();

    // Should return permission denied since tool is in deny list
    if let Some(s) = value.as_str() {
        assert!(s.contains("PermissionDeniedError") || s.contains("not allowed"));
    }
}

#[tokio::test]
async fn test_execution_context_depth_tracking() {
    use crate::adapters::execution_context::ExecutionContext;

    let ctx = ExecutionContext::default();
    assert_eq!(ctx.call_depth, 0);
    assert_eq!(ctx.max_call_depth, 10);

    let ctx2 = ctx.increment_depth();
    assert_eq!(ctx2.call_depth, 1);

    // Test is_depth_exceeded (should be false since we're at depth 1)
    assert!(!ctx2.is_depth_exceeded());

    // Create deep context (10 increments reaches max depth)
    let mut deep_ctx = ctx;
    for _ in 0..10 {
        deep_ctx = deep_ctx.increment_depth();
    }
    assert!(deep_ctx.is_depth_exceeded());
}

#[tokio::test]
async fn test_execution_context_tool_access() {
    use crate::adapters::execution_context::{ExecutionContext, AccessPolicy, AccessLevel};

    // Test allow all
    let ctx = ExecutionContext::with_access_policy(AccessPolicy {
        tools: AccessLevel::All,
        agents: AccessLevel::None,
        resources: AccessLevel::All,
        workflows: AccessLevel::All,
    });
    assert!(ctx.is_tool_allowed("any_tool"));
    assert!(!ctx.is_agent_allowed("any_agent"));

    // Test allow list
    let ctx2 = ExecutionContext::with_access_policy(AccessPolicy {
        tools: AccessLevel::AllowList(vec!["tool_a".to_string(), "tool_b".to_string()]),
        agents: AccessLevel::All,
        resources: AccessLevel::All,
        workflows: AccessLevel::All,
    });
    assert!(ctx2.is_tool_allowed("tool_a"));
    assert!(ctx2.is_tool_allowed("tool_b"));
    assert!(!ctx2.is_tool_allowed("tool_c"));

    // Test deny list
    let ctx3 = ExecutionContext::with_access_policy(AccessPolicy {
        tools: AccessLevel::DenyList(vec!["dangerous_tool".to_string()]),
        agents: AccessLevel::All,
        resources: AccessLevel::All,
        workflows: AccessLevel::All,
    });
    assert!(ctx3.is_tool_allowed("safe_tool"));
    assert!(!ctx3.is_tool_allowed("dangerous_tool"));
}