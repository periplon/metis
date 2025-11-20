use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, StateOperation};
use anyhow::Result;
use fake::faker::internet::en::{FreeEmail, SafeEmail, Username};
use fake::faker::lorem::en::{Paragraph, Sentence, Word};
use fake::faker::name::en::{Name, Title};
use fake::Fake;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::RwLock;

pub struct MockStrategyHandler {
    tera: Tera,
    state_manager: Arc<StateManager>,
    template_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl MockStrategyHandler {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self {
            tera: Tera::default(),
            state_manager,
            template_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn generate(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        match config.strategy {
            MockStrategyType::Static => Ok(json!(null)), // Handled by static content/response
            MockStrategyType::Template => self.generate_template(config, args),
            MockStrategyType::Random => self.generate_random(config),
            MockStrategyType::Stateful => self.generate_stateful(config, args).await,
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
}
