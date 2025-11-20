use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, StateOperation, ScriptLang};
use anyhow::Result;
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::lorem::en::{Paragraph, Sentence, Word};
use fake::faker::name::en::{Name, Title};
use fake::Fake;
use mlua::LuaSerdeExt;
use rhai::{Engine, Scope};
use rustpython_vm::convert::IntoObject;
use rustpython_vm::AsObject;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::RwLock;

pub struct MockStrategyHandler {
    _tera: Tera,
    state_manager: Arc<StateManager>,
    _template_cache: Arc<RwLock<HashMap<String, String>>>,
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
            _tera: Tera::default(),
            state_manager,
            _template_cache: Arc::new(RwLock::new(HashMap::new())),
            rhai_engine: engine,
        }
    }

    pub async fn generate(
        &self,
        config: &MockConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        match config.strategy {
            MockStrategyType::Static => Ok(json!(null)),
            MockStrategyType::Template => self.generate_template(config, args).await,
            MockStrategyType::Random => self.generate_random(config).await,
            MockStrategyType::Stateful => self.generate_stateful(config, args).await,
            MockStrategyType::Script => self.generate_script(config, args),
            MockStrategyType::File => self.generate_file(config).await,
            MockStrategyType::Pattern => self.generate_pattern(config),
            MockStrategyType::LLM => self.generate_llm(config, args).await,
            MockStrategyType::Database => self.generate_database(config, args).await,
        }
    }

    async fn generate_database(
        &self,
        config: &MockConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        use sqlx::any::AnyPoolOptions;
        use sqlx::Row;
        use sqlx::Column;

        // Ensure drivers are installed (safe to call multiple times)
        sqlx::any::install_default_drivers();

        let db_config = config.database.as_ref() 
            .ok_or_else(|| anyhow::anyhow!("Database config not provided"))?;

        // Create connection pool (in a real app, we should cache this)
        // For now, we create a new pool for each request which is not optimal but functional
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(&db_config.url)
            .await
            .map_err(|e| anyhow::anyhow!("Database connection error: {}", e))?;

        let mut query_builder = sqlx::query(&db_config.query);

        // Bind parameters
        if let Some(args_val) = args {
            for param_name in &db_config.params {
                if let Some(val) = args_val.get(param_name) {
                    match val {
                        Value::String(s) => query_builder = query_builder.bind(s),
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                query_builder = query_builder.bind(i);
                            } else if let Some(f) = n.as_f64() {
                                query_builder = query_builder.bind(f);
                            }
                        }
                        Value::Bool(b) => query_builder = query_builder.bind(b),
                        _ => query_builder = query_builder.bind(val.to_string()),
                    }
                } else {
                    // If param missing, bind null or error? Let's bind null for now
                    query_builder = query_builder.bind(Option::<String>::None);
                }
            }
        }

        let rows = query_builder
            .fetch_all(&pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database query error: {}", e))?;

        // Convert rows to JSON
        let mut results = Vec::new();
        for row in rows {
            let mut row_json = serde_json::Map::new();
            for col in row.columns() {
                let col_name = col.name();
                // This is a simplification. Handling all SQL types generically is complex.
                // We try to get as string for now, or handle common types if possible.
                // sqlx::AnyRow is tricky for generic value extraction without knowing types.
                // A robust implementation would need more complex type handling.
                
                // Try to get as string, if fails, try other types or skip
                // Note: sqlx::Any doesn't easily support "get as any" without type info.
                // For this MVP, we'll try to get everything as string.
                let val_str: Option<String> = row.try_get(col_name).ok();
                if let Some(s) = val_str {
                    row_json.insert(col_name.to_string(), Value::String(s));
                } else {
                    // Try integer
                    let val_int: Option<i64> = row.try_get(col_name).ok();
                    if let Some(i) = val_int {
                        row_json.insert(col_name.to_string(), json!(i));
                    } else {
                         // Try bool
                        let val_bool: Option<bool> = row.try_get(col_name).ok();
                        if let Some(b) = val_bool {
                            row_json.insert(col_name.to_string(), json!(b));
                        } else {
                             row_json.insert(col_name.to_string(), Value::Null);
                        }
                    }
                }
            }
            results.push(Value::Object(row_json));
        }

        // If single result expected (implied by usage), return first? 
        // Or always return array? Let's return array for now, user can use template to extract.
        Ok(json!(results))
    }

    async fn generate_llm(
        &self,
        config: &MockConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let llm_config = config.llm.as_ref() 
            .ok_or_else(|| anyhow::anyhow!("LLM config not provided"))?;

        // Get API key from environment variable
        let api_key = if let Some(env_var) = &llm_config.api_key_env {
            std::env::var(env_var) 
                .map_err(|_| anyhow::anyhow!("API key environment variable {} not set", env_var))? 
        } else {
            return Err(anyhow::anyhow!("No API key configuration provided"));
        };

        // Extract prompt from args
        let prompt = if let Some(args) = args {
            args.get("prompt") 
                .and_then(|v| v.as_str())
                .unwrap_or("Hello")
                .to_string()
        } else {
            "Hello".to_string()
        };

        match llm_config.provider {
            crate::config::LLMProvider::OpenAI => {
                let result = self.generate_openai(llm_config, &api_key, &prompt).await?;
                Ok(json!(result))
            }
            crate::config::LLMProvider::Anthropic => {
                let result = self.generate_anthropic(llm_config, &api_key, &prompt).await?;
                Ok(json!(result))
            }
        }
    }

    async fn generate_openai(
        &self,
        config: &crate::config::LLMConfig,
        api_key: &str,
        prompt: &str,
    ) -> Result<String> {
        use async_openai::{Client, config::OpenAIConfig, types::*};

        let openai_config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(openai_config);

        let mut messages = vec![];

        // Add system message if provided
        if let Some(system_prompt) = &config.system_prompt {
            messages.push(ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt.clone())
                    .build()? 
            ));
        }

        // Add user message
        messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt.to_string())
                .build()? 
        ));

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&config.model)
            .messages(messages);

        if let Some(temp) = config.temperature {
            request_builder.temperature(temp);
        }

        if let Some(max_tokens) = config.max_tokens {
            request_builder.max_tokens(max_tokens as u16);
        }

        let request = request_builder.build()?;

        let response = client
            .chat()
            .create(request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI API error: {}", e))?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))?;

        Ok(content)
    }

    async fn generate_anthropic(
        &self,
        config: &crate::config::LLMConfig,
        api_key: &str,
        prompt: &str,
    ) -> Result<String> {
        use serde_json::json;

        let client = reqwest::Client::new();

        let mut request_body = json!({
            "model": config.model,
            "max_tokens": config.max_tokens.unwrap_or(1000),
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        });

        if let Some(system_prompt) = &config.system_prompt {
            request_body["system"] = json!(system_prompt);
        }

        if let Some(temp) = config.temperature {
            request_body["temperature"] = json!(temp);
        }

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Anthropic API error: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error: {}", error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse Anthropic response: {}", e))?;

        let content = response_json
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .ok_or_else(|| anyhow::anyhow!("No response from Anthropic"))?;

        Ok(content.to_string())
    }

    async fn generate_template(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
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

    async fn generate_random(&self, config: &MockConfig) -> Result<Value> {
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
            match config.script_lang.as_ref().unwrap_or(&ScriptLang::Rhai) {
                ScriptLang::Rhai => {
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
                },
                ScriptLang::Lua => self.generate_script_lua(script, args),
                ScriptLang::Js => self.generate_script_js(script, args),
                ScriptLang::Python => self.generate_script_python(script, args),
            }
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_script_lua(&self, script: &str, args: Option<&Value>) -> Result<Value> {
        let lua = mlua::Lua::new();
        if let Some(args_val) = args {
             let lua_val = lua.to_value(args_val)?;
             lua.globals().set("input", lua_val)?;
        }
        let chunk = lua.load(script);
        let result: mlua::Value = chunk.eval()?;
        let json_val = serde_json::to_value(&result)?;
        Ok(json_val)
    }

    fn generate_script_js(&self, script: &str, args: Option<&Value>) -> Result<Value> {
        use boa_engine::{Context, Source};

        let mut context = Context::default();
        
        if let Some(args_val) = args {
             let json_str = serde_json::to_string(args_val)?;
             // Boa doesn't have easy generic "to_value" without interop, but we can parse JSON string in JS
             let setup_script = format!("const input = JSON.parse('{}');", json_str);
             context.eval(Source::from_bytes(setup_script.as_bytes()))
                .map_err(|e| anyhow::anyhow!("JS setup error: {}", e))?;
        }

        let result = context.eval(Source::from_bytes(script.as_bytes()))
            .map_err(|e| anyhow::anyhow!("JS execution error: {}", e))?;

        // Convert result to JSON
        let json_val = result.to_json(&mut context)
            .map_err(|e| anyhow::anyhow!("JS result conversion error: {}", e))?;
            
        Ok(json_val)
    }

    fn generate_script_python(&self, script: &str, args: Option<&Value>) -> Result<Value> {
        use rustpython_vm::Interpreter;
        use rustpython_vm::compiler::Mode;
        
        let interpreter = Interpreter::without_stdlib(Default::default());
        
        interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            
            if let Some(args_val) = args {
                // Plan B: Construct PyObject recursively.
                // This is safer and cleaner.
                let py_val = self.json_to_python(vm, args_val);
                scope.globals.set_item("input", py_val, vm).map_err(|e| self.map_py_err(vm, e))?;
            }
            
            let code_obj = vm.compile(script, Mode::Exec, "<embedded>".to_owned())
                .map_err(|err| self.map_py_err(vm, vm.new_syntax_error(&err, Some(script))))?;
                
            let _ = vm.run_code_obj(code_obj, scope.clone()).map_err(|e| self.map_py_err(vm, e))?;
            
            if let Ok(output) = scope.globals.get_item("output", vm) {
                self.python_to_json(vm, output)
            } else {
                Ok(Value::Null)
            }
        }).map_err(|e| anyhow::anyhow!("Python error: {}", e))
    }

    fn map_py_err(&self, vm: &rustpython_vm::VirtualMachine, err: rustpython_vm::PyRef<rustpython_vm::builtins::PyBaseException>) -> anyhow::Error {
        let mut msg = String::new();
        if let Ok(s) = err.into_object().str(vm) {
            msg.push_str(s.as_str());
        } else {
            msg.push_str("Unknown python error");
        }
        anyhow::anyhow!("{}", msg)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn json_to_python(&self, vm: &rustpython_vm::VirtualMachine, value: &Value) -> rustpython_vm::PyObjectRef {
        use rustpython_vm::convert::ToPyObject;

        match value {
            Value::Null => vm.ctx.none(),
            Value::Bool(b) => b.to_pyobject(vm),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    vm.ctx.new_int(i).into()
                } else if let Some(f) = n.as_f64() {
                    vm.ctx.new_float(f).into()
                } else {
                    vm.ctx.none()
                }
            }
            Value::String(s) => vm.ctx.new_str(s.as_str()).into(),
            Value::Array(arr) => {
                let elements: Vec<_> = arr.iter().map(|v| self.json_to_python(vm, v)).collect();
                vm.ctx.new_list(elements).into()
            }
            Value::Object(obj) => {
                let dict = vm.ctx.new_dict();
                for (k, v) in obj {
                    let py_k = vm.ctx.new_str(k.as_str());
                    let py_v = self.json_to_python(vm, v);
                    let _ = dict.set_item(py_k.as_object(), py_v, vm);
                }
                dict.into()
            }
        }
    }

    fn python_to_json(&self, vm: &rustpython_vm::VirtualMachine, obj: rustpython_vm::PyObjectRef) -> Result<Value> {
        use rustpython_vm::builtins::{PyList, PyStr, PyInt, PyFloat, PyDict};
        // Basic conversion
        if vm.is_none(&obj) {
            Ok(Value::Null)
        } else if let Some(s) = obj.payload::<PyStr>() {
            Ok(Value::String(s.as_str().to_string()))
        } else if obj.class().is(vm.ctx.types.bool_type) {
            Ok(Value::Bool(obj.try_to_bool(vm).unwrap_or(false)))
        } else if let Some(i) = obj.payload::<PyInt>() {
            match i.try_to_primitive::<i64>(vm) {
                Ok(val) => Ok(json!(val)),
                Err(_) => Ok(Value::Null) 
            }
        } else if let Some(f) = obj.payload::<PyFloat>() {
            Ok(json!(f.to_f64()))
        } else if let Some(l) = obj.payload::<PyList>() {
            let borrowed = l.borrow_vec();
            let mut arr = Vec::new();
            for item in borrowed.iter() {
                arr.push(self.python_to_json(vm, item.clone())?);
            }
            Ok(Value::Array(arr))
        } else if let Some(d) = obj.payload::<PyDict>() {
            let mut map = serde_json::Map::new();
            for (k, v) in d {
                let k_str = if let Some(s) = k.payload::<PyStr>() {
                    s.as_str().to_string()
                } else {
                    continue;
                };
                let v_json = self.python_to_json(vm, v)?;
                map.insert(k_str, v_json);
            }
            Ok(Value::Object(map))
        } else {
            let s = obj.str(vm).map_err(|e| self.map_py_err(vm, e))?;
            Ok(Value::String(s.as_str().to_string()))
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
