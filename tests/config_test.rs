use metis::config::Settings;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_load_external_configs() -> anyhow::Result<()> {
    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create config directories
    fs::create_dir_all(root.join("config/tools"))?;
    fs::create_dir_all(root.join("config/resources"))?;
    fs::create_dir_all(root.join("config/prompts"))?;

    // Create metis.toml
    let metis_toml = r#"
[server]
host = "127.0.0.1"
port = 3000
"#;
    fs::write(root.join("metis.toml"), metis_toml)?;

    // Create a tool in JSON
    let tool_json = r#"
{
    "name": "json_tool",
    "description": "A tool defined in JSON",
    "input_schema": {},
    "mock": {
        "strategy": "static"
    }
}
"#;
    fs::write(root.join("config/tools/tool1.json"), tool_json)?;

    // Create a tool in YAML
    let tool_yaml = r#"
name: yaml_tool
description: A tool defined in YAML
input_schema: {}
mock:
  strategy: static
"#;
    fs::write(root.join("config/tools/tool2.yaml"), tool_yaml)?;

    // Create a resource in JSON
    let resource_json = r#"
{
    "uri": "file:///test.txt",
    "name": "test_resource",
    "description": "A resource defined in JSON",
    "mock": {
        "strategy": "static"
    }
}
"#;
    fs::write(root.join("config/resources/resource1.json"), resource_json)?;

    // Create a prompt in YAML
    let prompt_yaml = r#"
name: yaml_prompt
description: A prompt defined in YAML
messages:
  - role: user
    content: Hello
"#;
    fs::write(root.join("config/prompts/prompt1.yaml"), prompt_yaml)?;

    // Load settings
    let settings = Settings::from_root(root.to_str().unwrap())?;

    // Verify tools
    assert_eq!(settings.tools.len(), 2);
    assert!(settings.tools.iter().any(|t| t.name == "json_tool"));
    assert!(settings.tools.iter().any(|t| t.name == "yaml_tool"));

    // Verify resources
    assert_eq!(settings.resources.len(), 1);
    assert_eq!(settings.resources[0].name, "test_resource");

    // Verify prompts
    assert_eq!(settings.prompts.len(), 1);
    assert_eq!(settings.prompts[0].name, "yaml_prompt");

    Ok(())
}
