use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, StateOperation};
use anyhow::Result;
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::lorem::en::{Paragraph, Sentence, Word};
use fake::faker::name::en::{Name, Title};
use fake::Fake;
use rhai::{Engine, Scope};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::RwLock;

pub struct MockStrategyHandler {
    tera: Tera,
    state_manager: Arc<StateManager>,
    template_cache: Arc<RwLock<HashMap<String, String>>>,
    rhai_engine: Engine,
}

impl MockStrategyHandler {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        let mut engine = Engine::new();
        
        // Register custom functions for Rhai
        engine.register_fn("fake_name", || Name().fake::<String>());
        engine.register_fn("fake_email", || SafeEmail().fake::<String>());
        engine.register_fn("fake_sentence", || Sentence(1..10).fake::<String>());
        
        Self {
            tera: Tera::default(),
            state_manager,
            template_cache: Arc::new(RwLock::new(HashMap::new())),
            rhai_engine: engine,
        }
    }

    pub async fn generate(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        match config.strategy {
            MockStrategyType::Static => Ok(json!(null)), // Handled by static content/response
            MockStrategyType::Template => self.generate_template(config, args),
            MockStrategyType::Random => self.generate_random(config),
            MockStrategyType::Stateful => self.generate_stateful(config, args).await,
            MockStrategyType::Script => self.generate_script(config, args),
            MockStrategyType::File => self.generate_file(config).await,
            MockStrategyType::Pattern => self.generate_pattern(config),
        }
    }

    fn generate_template(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        if let Some(template_str) = &config.template {
            let mut context = Context::new();
            if let Some(args_val) = args {
                if let Some(obj) = args_val.as_object() {
                    for (k, v) in obj {
                        context.insert(k, v);
                    }
                }
            }
            
            // One-off rendering for now. For performance, we should pre-compile templates.
            let rendered = Tera::one_off(template_str, &context, false)?;
            
            // Try to parse as JSON, otherwise return as string
            if let Ok(json_val) = serde_json::from_str::<Value>(&rendered) {
                Ok(json_val)
            } else {
                Ok(Value::String(rendered))
            }
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_random(&self, config: &MockConfig) -> Result<Value> {
        if let Some(faker_type) = &config.faker_type {
            match faker_type.as_str() {
                "name" => Ok(json!(Name().fake::<String>())),
                "title" => Ok(json!(Title().fake::<String>())),
                "email" => Ok(json!(SafeEmail().fake::<String>())),
                "username" => Ok(json!(Username().fake::<String>())),
                "word" => Ok(json!(Word().fake::<String>())),
                "sentence" => Ok(json!(Sentence(1..10).fake::<String>())),
                "paragraph" => Ok(json!(Paragraph(1..3).fake::<String>())),
                _ => Ok(json!(format!("Unknown faker type: {}", faker_type))),
            }
        } else {
            Ok(Value::Null)
        }
    }

    async fn generate_stateful(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        if let Some(stateful_config) = &config.stateful {
            match stateful_config.operation {
                StateOperation::Get => {
                    let value = self.state_manager.get(&stateful_config.state_key).await
                        .unwrap_or(Value::Null);
                    Ok(value)
                }
                StateOperation::Set => {
                    if let Some(args_val) = args {
                        self.state_manager.set(stateful_config.state_key.clone(), args_val.clone()).await;
                        Ok(args_val.clone())
                    } else {
                        Ok(Value::Null)
                    }
                }
                StateOperation::Increment => {
                    let new_value = self.state_manager.increment(&stateful_config.state_key).await;
                    
                    // If template is provided, render it with the new value
                    if let Some(template_str) = &stateful_config.template {
                        let mut context = Context::new();
                        context.insert("value", &new_value);
                        let rendered = Tera::one_off(template_str, &context, false)?;
                        if let Ok(json_val) = serde_json::from_str::<Value>(&rendered) {
                            Ok(json_val)
                        } else {
                            Ok(Value::String(rendered))
                        }
                    } else {
                        Ok(json!(new_value))
                    }
                }
            }
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_script(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        if let Some(script) = &config.script {
            let mut scope = Scope::new();
            
            if let Some(args_val) = args {
                // Convert serde_json::Value to Rhai Dynamic
                let args_dynamic = serde_json::from_value::<rhai::Dynamic>(args_val.clone())?;
                scope.push("input", args_dynamic);
            }

            let result = self.rhai_engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script)?;
            
            // Convert Rhai Dynamic back to serde_json::Value
            let json_val = serde_json::to_value(&result)?;
            Ok(json_val)
        } else {
            Ok(Value::Null)
        }
    }

    async fn generate_file(&self, config: &MockConfig) -> Result<Value> {
        if let Some(file_config) = &config.file {
            // Read file content
            let content = tokio::fs::read_to_string(&file_config.path).await?;
            
            // Parse as JSON array
            let data: Vec<Value> = serde_json::from_str(&content)?;
            
            if data.is_empty() {
                return Ok(Value::Null);
            }

            // Select based on strategy
            let selected = match file_config.selection.as_str() {
                "random" => {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let idx = rng.gen_range(0..data.len());
                    &data[idx]
                }
                "sequential" => {
                    // TODO: Implement sequential selection with state
                    &data[0]
                }
                _ => &data[0],
            };

            Ok(selected.clone())
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_pattern(&self, config: &MockConfig) -> Result<Value> {
        if let Some(pattern) = &config.pattern {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            
            // Simple pattern generation - expand character classes
            let mut result = String::new();
            let mut chars = pattern.chars().peekable();
            
            while let Some(ch) = chars.next() {
                match ch {
                    '\\' => {
                        if let Some(next) = chars.next() {
                            match next {
                                'd' => result.push_str(&rng.gen_range(0..10).to_string()),
                                'w' => {
                                    let c = if rng.gen_bool(0.5) {
                                        rng.gen_range(b'a'..=b'z') as char
                                    } else {
                                        rng.gen_range(b'A'..=b'Z') as char
                                    };
                                    result.push(c);
                                }
                                _ => result.push(next),
                            }
                        }
                    }
                    _ => result.push(ch),
                }
            }
            
            Ok(json!(result))
        } else {
            Ok(Value::Null)
        }
    }
}
