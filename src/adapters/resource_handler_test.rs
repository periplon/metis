use super::resource_handler::InMemoryResourceHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, ResourceConfig, Settings, ServerSettings};
use crate::domain::ResourcePort;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test] async fn test_get_resource_static() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ResourceConfig {
        uri: "file:///test.txt".to_string(),
        name: "Test".to_string(),
        description: None,
        mime_type: Some("text/plain".to_string()),
        output_schema: None,
        content: Some("Static Content".to_string()),
        mock: None,
        tags: vec![],
    }];

    let settings = Settings {
            config_path: None,
            version: 0,
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
        resources: config,
        resource_templates: vec![],
        tools: vec![],
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
    };
    let handler = InMemoryResourceHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.get_resource("file:///test.txt").await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value.content, "Static Content");
}

#[tokio::test] async fn test_get_resource_mock() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![ResourceConfig {
        uri: "file:///mock.txt".to_string(),
        name: "Mock".to_string(),
        description: None,
        mime_type: Some("text/plain".to_string()),
        output_schema: None,
        content: None,
        mock: Some(MockConfig {
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
        }),
        tags: vec![],
    }];

    let settings = Settings {
            config_path: None,
            version: 0,
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
        resources: config,
        resource_templates: vec![],
        tools: vec![],
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
    };
    let handler = InMemoryResourceHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.get_resource("file:///mock.txt").await;
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value.content, "Hello, !");
}

#[tokio::test] async fn test_get_resource_not_found() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let settings = Settings {
            config_path: None,
            version: 0,
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
        resources: vec![],
        resource_templates: vec![],
        tools: vec![],
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
    };
    let handler = InMemoryResourceHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.get_resource("file:///unknown.txt").await;
    assert!(result.is_err());
}

#[tokio::test] async fn test_list_resources() {
    let mock_strategy = Arc::new(MockStrategyHandler::new(Arc::new(StateManager::new())));
    let config = vec![
        ResourceConfig {
            uri: "file:///1.txt".to_string(),
            name: "One".to_string(),
            description: None,
            mime_type: Some("text/plain".to_string()),
            output_schema: None,
            content: None,
            mock: None,
            tags: vec![],
        },
        ResourceConfig {
            uri: "file:///2.txt".to_string(),
            name: "Two".to_string(),
            description: None,
            mime_type: Some("text/plain".to_string()),
            output_schema: None,
            content: None,
            mock: None,
            tags: vec![],
        },
    ];

    let settings = Settings {
            config_path: None,
            version: 0,
        server: ServerSettings { host: "127.0.0.1".to_string(), port: 3000 },
        auth: Default::default(),
        resources: config,
        resource_templates: vec![],
        tools: vec![],
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
    };
    let handler = InMemoryResourceHandler::new(Arc::new(RwLock::new(settings)), mock_strategy);

    let result = handler.list_resources().await;
    assert!(result.is_ok());
    let list = result.unwrap();
    assert_eq!(list.len(), 2);
}

#[test]
fn test_extract_template_args_simple() {
    let template = "file://countries/{country_code}/info";
    let uri = "file://countries/us/info";
    let result = InMemoryResourceHandler::extract_template_args(template, uri);
    assert!(result.is_some());
    let args = result.unwrap();
    assert_eq!(args.get("country_code").and_then(|v| v.as_str()), Some("us"));
}

#[test]
fn test_extract_template_args_multiple() {
    let template = "db://tables/{schema}/{table}/row/{id}";
    let uri = "db://tables/public/users/row/123";
    let result = InMemoryResourceHandler::extract_template_args(template, uri);
    assert!(result.is_some());
    let args = result.unwrap();
    assert_eq!(args.get("schema").and_then(|v| v.as_str()), Some("public"));
    assert_eq!(args.get("table").and_then(|v| v.as_str()), Some("users"));
    assert_eq!(args.get("id").and_then(|v| v.as_str()), Some("123"));
}

#[test]
fn test_extract_template_args_no_match() {
    let template = "file://countries/{country_code}/info";
    let uri = "file://cities/ny/info";
    let result = InMemoryResourceHandler::extract_template_args(template, uri);
    assert!(result.is_none());
}

#[test]
fn test_extract_template_args_no_placeholders() {
    let template = "file://static/resource";
    let uri = "file://static/resource";
    let result = InMemoryResourceHandler::extract_template_args(template, uri);
    assert!(result.is_some());
    let args = result.unwrap();
    assert!(args.as_object().unwrap().is_empty());
}
