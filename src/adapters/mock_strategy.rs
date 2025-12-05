use crate::adapters::datafusion_handler::DataFusionHandler;
use crate::adapters::file_storage::FileStorageHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{
    DataFusionConfig, DataLakeCrudConfig, DataLakeCrudOperation, MockConfig, MockStrategyType,
    StateOperation, ScriptLang, Settings, FakerSchemaConfig, FakerFieldConfig, FakerFieldType,
    FakerArrayConfig, DataLakeFileFormat, DataRecord,
};
use anyhow::Result;
use fake::faker::address::en::{CityName, CountryName, PostCode, StateAbbr, StreetName};
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::lorem::en::{Paragraph, Sentence, Word};
use fake::faker::name::en::{FirstName, LastName, Name, Title};
use fake::faker::phone_number::en::PhoneNumber;
use fake::Fake;
use rand::Rng;
use mlua::LuaSerdeExt;
use rhai::{Engine, Scope};  // Engine used for per-request Rhai script execution
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
    datafusion_handler: Option<Arc<DataFusionHandler>>,
    settings: Option<Arc<RwLock<Settings>>>,
    file_storage: Option<Arc<FileStorageHandler>>,
}

impl MockStrategyHandler {
    pub fn new(state_manager: Arc<StateManager>) -> Self {
        Self::new_with_datafusion(state_manager, None, None, None)
    }

    pub fn new_with_datafusion(
        state_manager: Arc<StateManager>,
        datafusion_handler: Option<Arc<DataFusionHandler>>,
        settings: Option<Arc<RwLock<Settings>>>,
        file_storage: Option<Arc<FileStorageHandler>>,
    ) -> Self {
        Self {
            _tera: Tera::default(),
            state_manager,
            _template_cache: Arc::new(RwLock::new(HashMap::new())),
            datafusion_handler,
            settings,
            file_storage,
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
            MockStrategyType::DataLakeCrud => self.generate_data_lake_crud(config, args).await,
        }
    }

    async fn generate_database(
        &self,
        config: &MockConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let db_config = config.database.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database config not provided"))?;

        // Check if using DataFusion for datalake queries
        if db_config.db_type == crate::config::DatabaseType::DataFusion {
            return self.generate_datafusion(db_config, args).await;
        }

        // Traditional database query via sqlx
        self.generate_sqlx_database(db_config, args).await
    }

    async fn generate_datafusion(
        &self,
        db_config: &crate::config::DatabaseConfig,
        _args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let df_config = db_config.datafusion.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DataFusion config not provided for datafusion db_type"))?;

        // Parse data_lake and schema_name - support full path format "data_lake.schema_name"
        let (data_lake_name, schema_name) = if df_config.schema_name.is_empty() && df_config.data_lake.contains('.') {
            // Full path format: "aep.companies" -> data_lake="aep", schema_name="companies"
            let parts: Vec<&str> = df_config.data_lake.splitn(2, '.').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (df_config.data_lake.clone(), df_config.schema_name.clone())
            }
        } else {
            (df_config.data_lake.clone(), df_config.schema_name.clone())
        };

        // Use the shared DataFusionHandler if available (preferred - uses FileStorageHandler)
        if let (Some(datafusion_handler), Some(settings)) = (&self.datafusion_handler, &self.settings) {
            let mut sql = db_config.query.clone();

            // If explicit data_lake and schema_name are provided, register that table
            if !data_lake_name.is_empty() && !schema_name.is_empty() {
                let data_lake = {
                    let settings_guard = settings.read().await;
                    settings_guard.data_lakes.iter()
                        .find(|dl| dl.name == data_lake_name)
                        .cloned()
                };

                if let Some(data_lake) = data_lake {
                    let table_name = datafusion_handler
                        .register_data_lake_table(&data_lake, &schema_name)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to register data lake table: {}", e))?;

                    // Replace $table placeholder and dotted name with registered table name
                    let dotted_name = format!("{}.{}", data_lake_name, schema_name);
                    sql = sql.replace("$table", &table_name).replace(&dotted_name, &table_name);
                }
            }

            // Parse SQL query to find table references like "schema.table" and register them
            // This handles cases where no explicit data_lake/schema is selected in the UI
            let table_refs = self.extract_table_references(&sql);
            for (lake_name, tbl_name) in &table_refs {
                let data_lake = {
                    let settings_guard = settings.read().await;
                    settings_guard.data_lakes.iter()
                        .find(|dl| dl.name == *lake_name)
                        .cloned()
                };

                if let Some(data_lake) = data_lake {
                    match datafusion_handler.register_data_lake_table(&data_lake, tbl_name).await {
                        Ok(registered_name) => {
                            // Replace the dotted reference with the registered table name
                            let dotted_ref = format!("{}.{}", lake_name, tbl_name);
                            sql = sql.replace(&dotted_ref, &registered_name);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to register table {}.{}: {}", lake_name, tbl_name, e);
                        }
                    }
                }
            }

            // Execute the query using DataFusionHandler
            let result = datafusion_handler
                .execute_sql(&sql)
                .await
                .map_err(|e| anyhow::anyhow!("SQL execution error: {}", e))?;

            tracing::debug!(
                "DataFusion query returned {} rows (total_rows: {})",
                result.rows.len(),
                result.total_rows
            );

            return Ok(json!(result.rows));
        }

        // Fallback: Create a standalone session context (for cases without DataFusionHandler)
        use datafusion::prelude::*;
        let ctx = SessionContext::new();

        // Register JSON functions for querying JSON data in the data column
        use datafusion_functions_json::udfs::*;
        let udfs = vec![
            json_get_udf(),
            json_get_bool_udf(),
            json_get_float_udf(),
            json_get_int_udf(),
            json_get_json_udf(),
            json_as_text_udf(),
            json_get_str_udf(),
            json_contains_udf(),
            json_length_udf(),
            json_object_keys_udf(),
        ];
        for udf in udfs {
            ctx.register_udf((*udf).clone());
        }

        // Get the base path from file_storage configuration, fallback to "data_files"
        let base_path = if let Some(settings) = &self.settings {
            let settings_guard = settings.read().await;
            settings_guard.file_storage
                .as_ref()
                .and_then(|fs| fs.local_path.clone())
                .unwrap_or_else(|| "data_files".to_string())
        } else {
            "data_files".to_string()
        };

        // Build the file path using FileStorageHandler convention:
        // {base_path}/data-lakes/{data_lake}/{schema_name}/*.parquet or *.jsonl
        let data_dir = format!("{}/data-lakes/{}/{}", base_path, data_lake_name, schema_name);

        // Check if directory exists and find data files
        let data_path = std::path::Path::new(&data_dir);
        if !data_path.exists() {
            return Err(anyhow::anyhow!(
                "Data directory not found for {}.{}. Expected: {}",
                data_lake_name, schema_name, data_dir
            ));
        }

        // Find parquet or jsonl files in the directory
        let mut parquet_files: Vec<String> = Vec::new();
        let mut jsonl_files: Vec<String> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(data_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let path_str = path.to_string_lossy().to_string();
                    if ext == "parquet" && !path_str.contains("_tombstones") {
                        parquet_files.push(path_str);
                    } else if ext == "jsonl" && !path_str.contains("_tombstones") {
                        jsonl_files.push(path_str);
                    }
                }
            }
        }

        // Register the data file as a table
        let table_name = format!("{}_{}", data_lake_name, schema_name);

        if !parquet_files.is_empty() {
            // Register all parquet files
            for (idx, file_path) in parquet_files.iter().enumerate() {
                let temp_table = if idx == 0 { table_name.clone() } else { format!("{}_{}", table_name, idx) };
                ctx.register_parquet(&temp_table, file_path, ParquetReadOptions::default())
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to register parquet file {}: {}", file_path, e))?;
            }
        } else if !jsonl_files.is_empty() {
            // Register all jsonl files
            for (idx, file_path) in jsonl_files.iter().enumerate() {
                let temp_table = if idx == 0 { table_name.clone() } else { format!("{}_{}", table_name, idx) };
                let options = NdJsonReadOptions::default().file_extension(".jsonl");
                ctx.register_json(&temp_table, file_path, options)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to register JSONL file {}: {}", file_path, e))?;
            }
        } else {
            return Err(anyhow::anyhow!(
                "No data files found for {}.{}. Expected parquet or jsonl files in: {}",
                data_lake_name, schema_name, data_dir
            ));
        }

        // Replace $table placeholder with actual table name
        let sql = db_config.query.replace("$table", &table_name);

        // Execute the query
        let df = ctx.sql(&sql)
            .await
            .map_err(|e| anyhow::anyhow!("SQL execution error: {}", e))?;

        // Collect results
        let batches = df.collect()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to collect results: {}", e))?;

        // Convert to JSON
        let mut results = Vec::new();
        for batch in batches {
            let schema = batch.schema();
            for row_idx in 0..batch.num_rows() {
                let mut row_json = serde_json::Map::new();
                for (col_idx, field) in schema.fields().iter().enumerate() {
                    let col = batch.column(col_idx);
                    let value = self.arrow_value_to_json(col, row_idx);
                    row_json.insert(field.name().clone(), value);
                }
                results.push(Value::Object(row_json));
            }
        }

        Ok(json!(results))
    }

    /// Extract table references from SQL query in the format "schema.table"
    /// Returns a vector of (data_lake_name, table_name) tuples
    fn extract_table_references(&self, sql: &str) -> Vec<(String, String)> {
        let mut refs = Vec::new();

        // Simple parsing for "word.word" patterns after SQL keywords
        // This handles common cases like "FROM schema.table" or "JOIN schema.table"
        let tokens: Vec<&str> = sql.split_whitespace().collect();

        for (i, token) in tokens.iter().enumerate() {
            // Look for tokens after FROM, JOIN, INTO keywords
            if i > 0 {
                let prev = tokens[i - 1].to_uppercase();
                if prev == "FROM" || prev == "JOIN" || prev == "INTO" ||
                   prev.ends_with("JOIN") || prev == "TABLE" {
                    // Check if this token contains a dot (schema.table pattern)
                    let clean_token = token.trim_matches(|c| c == ',' || c == ';' || c == '(' || c == ')');
                    if clean_token.contains('.') && !clean_token.starts_with('\'') && !clean_token.starts_with('"') {
                        let parts: Vec<&str> = clean_token.splitn(2, '.').collect();
                        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                            // Skip if it looks like a function call or already registered
                            let schema = parts[0].to_lowercase();
                            let table = parts[1].to_lowercase();
                            if !refs.iter().any(|(s, t): &(String, String)| s == &schema && t == &table) {
                                refs.push((schema, table));
                            }
                        }
                    }
                }
            }
        }

        refs
    }

    fn arrow_value_to_json(&self, col: &dyn datafusion::arrow::array::Array, row_idx: usize) -> Value {
        use datafusion::arrow::array::*;
        use datafusion::arrow::datatypes::DataType;

        if col.is_null(row_idx) {
            return Value::Null;
        }

        match col.data_type() {
            DataType::Utf8 => {
                let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
                Value::String(arr.value(row_idx).to_string())
            }
            DataType::LargeUtf8 => {
                let arr = col.as_any().downcast_ref::<LargeStringArray>().unwrap();
                Value::String(arr.value(row_idx).to_string())
            }
            DataType::Int8 => {
                let arr = col.as_any().downcast_ref::<Int8Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Int16 => {
                let arr = col.as_any().downcast_ref::<Int16Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Int32 => {
                let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Int64 => {
                let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::UInt8 => {
                let arr = col.as_any().downcast_ref::<UInt8Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::UInt16 => {
                let arr = col.as_any().downcast_ref::<UInt16Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::UInt32 => {
                let arr = col.as_any().downcast_ref::<UInt32Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::UInt64 => {
                let arr = col.as_any().downcast_ref::<UInt64Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Float32 => {
                let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Float64 => {
                let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
                json!(arr.value(row_idx))
            }
            DataType::Boolean => {
                let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                json!(arr.value(row_idx))
            }
            _ => {
                // For other types, try to get string representation
                let arr = col.as_any().downcast_ref::<StringArray>();
                if let Some(arr) = arr {
                    Value::String(arr.value(row_idx).to_string())
                } else {
                    Value::Null
                }
            }
        }
    }

    async fn generate_sqlx_database(
        &self,
        db_config: &crate::config::DatabaseConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        use sqlx::any::AnyPoolOptions;
        use sqlx::Row;
        use sqlx::Column;

        // Ensure drivers are installed (safe to call multiple times)
        sqlx::any::install_default_drivers();

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
        // If faker_schema is provided, use schema-driven generation
        if let Some(faker_schema) = &config.faker_schema {
            return self.generate_from_schema(faker_schema);
        }

        // Fall back to simple faker_type generation
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

    /// Public method to generate data from a faker schema configuration
    /// Used by data lake record generation
    pub fn generate_from_faker_config(&self, schema_config: &FakerSchemaConfig) -> Result<Value> {
        self.generate_from_schema(schema_config)
    }

    /// Generate data from a schema-driven faker configuration
    fn generate_from_schema(&self, schema_config: &FakerSchemaConfig) -> Result<Value> {
        let mut result = serde_json::Map::new();

        // Process all field configurations
        for (path, field_config) in &schema_config.fields {
            let value = self.generate_faker_value(field_config)?;
            self.set_nested_value(&mut result, path, value, &schema_config.arrays)?;
        }

        Ok(Value::Object(result))
    }

    /// Generate a single faker value based on field configuration
    fn generate_faker_value(&self, config: &FakerFieldConfig) -> Result<Value> {
        match config.faker_type {
            // Personal
            FakerFieldType::FirstName => Ok(json!(FirstName().fake::<String>())),
            FakerFieldType::LastName => Ok(json!(LastName().fake::<String>())),
            FakerFieldType::FullName => Ok(json!(Name().fake::<String>())),
            FakerFieldType::Username => Ok(json!(Username().fake::<String>())),

            // Contact
            FakerFieldType::Email => Ok(json!(SafeEmail().fake::<String>())),
            FakerFieldType::Phone => Ok(json!(PhoneNumber().fake::<String>())),

            // Address
            FakerFieldType::StreetAddress => Ok(json!(StreetName().fake::<String>())),
            FakerFieldType::City => Ok(json!(CityName().fake::<String>())),
            FakerFieldType::State => Ok(json!(StateAbbr().fake::<String>())),
            FakerFieldType::Country => Ok(json!(CountryName().fake::<String>())),
            FakerFieldType::PostalCode => Ok(json!(PostCode().fake::<String>())),

            // Text
            FakerFieldType::Word => Ok(json!(Word().fake::<String>())),
            FakerFieldType::Sentence => Ok(json!(Sentence(1..10).fake::<String>())),
            FakerFieldType::Paragraph => Ok(json!(Paragraph(1..3).fake::<String>())),

            // Numbers
            FakerFieldType::Integer => {
                let min = config.min.unwrap_or(0.0) as i64;
                let max = config.max.unwrap_or(100.0) as i64;
                let value: i64 = rand::thread_rng().gen_range(min..=max);
                Ok(json!(value))
            }
            FakerFieldType::Float => {
                let min = config.min.unwrap_or(0.0);
                let max = config.max.unwrap_or(100.0);
                let value: f64 = rand::thread_rng().gen_range(min..=max);
                Ok(json!(value))
            }

            // Identifiers
            FakerFieldType::Uuid => Ok(json!(uuid::Uuid::new_v4().to_string())),

            // Special
            FakerFieldType::Pattern => {
                if let Some(pattern) = &config.pattern {
                    // Use regex_generate crate or simple pattern replacement
                    Ok(json!(self.generate_from_pattern(pattern)))
                } else {
                    Ok(json!(""))
                }
            }
            FakerFieldType::Enum => {
                if let Some(enum_values) = &config.enum_values {
                    if !enum_values.is_empty() {
                        let idx = rand::thread_rng().gen_range(0..enum_values.len());
                        Ok(json!(enum_values[idx].clone()))
                    } else {
                        Ok(json!(""))
                    }
                } else {
                    Ok(json!(""))
                }
            }
            FakerFieldType::Constant => {
                if let Some(constant) = &config.constant {
                    Ok(constant.clone())
                } else {
                    Ok(Value::Null)
                }
            }

            // Default
            FakerFieldType::Lorem => Ok(json!(Sentence(1..5).fake::<String>())),
        }
    }

    /// Generate string from a simple pattern (basic implementation)
    fn generate_from_pattern(&self, pattern: &str) -> String {
        let mut result = String::new();
        let mut rng = rand::thread_rng();

        for c in pattern.chars() {
            match c {
                '#' => result.push_str(&rng.gen_range(0..10).to_string()),
                '?' => result.push(rng.gen_range(b'a'..=b'z') as char),
                '*' => {
                    if rng.gen_bool(0.5) {
                        result.push_str(&rng.gen_range(0..10).to_string())
                    } else {
                        result.push(rng.gen_range(b'a'..=b'z') as char)
                    }
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Set a value at a nested path in the result object
    fn set_nested_value(
        &self,
        result: &mut serde_json::Map<String, Value>,
        path: &str,
        value: Value,
        array_configs: &std::collections::HashMap<String, FakerArrayConfig>,
    ) -> Result<()> {
        // Parse path segments (e.g., "user.address.city" or "items[*].name")
        let segments: Vec<&str> = path.split('.').collect();

        if segments.is_empty() {
            return Ok(());
        }

        // Handle array wildcard paths specially
        if path.contains("[*]") {
            // Find the array path
            let array_path_end = path.find("[*]").unwrap();
            let array_path = &path[..array_path_end];
            let item_field = &path[array_path_end + 4..]; // Skip "[*]."

            // Get array config for size
            let array_config = array_configs.get(array_path);
            let (min_items, max_items) = if let Some(cfg) = array_config {
                (cfg.min_items, cfg.max_items)
            } else {
                (1, 3) // Default
            };

            let count = rand::thread_rng().gen_range(min_items..=max_items);

            // Navigate to/create the array
            let array_segments: Vec<&str> = array_path.split('.').collect();
            let mut current = result;

            for (i, seg) in array_segments.iter().enumerate() {
                if i == array_segments.len() - 1 {
                    // Last segment - create/get the array
                    let arr = current.entry(seg.to_string())
                        .or_insert_with(|| Value::Array(vec![]));

                    if let Value::Array(items) = arr {
                        // Ensure we have enough items
                        while items.len() < count {
                            items.push(Value::Object(serde_json::Map::new()));
                        }

                        // Set the field on each item
                        for item in items.iter_mut() {
                            if let Value::Object(obj) = item {
                                // Handle nested fields in item
                                if item_field.contains('.') {
                                    let nested_segments: Vec<&str> = item_field.split('.').collect();
                                    let mut nested_current = obj;

                                    for (j, nested_seg) in nested_segments.iter().enumerate() {
                                        if j == nested_segments.len() - 1 {
                                            nested_current.insert(nested_seg.to_string(), value.clone());
                                        } else {
                                            nested_current = nested_current
                                                .entry(nested_seg.to_string())
                                                .or_insert_with(|| Value::Object(serde_json::Map::new()))
                                                .as_object_mut()
                                                .unwrap();
                                        }
                                    }
                                } else {
                                    obj.insert(item_field.to_string(), value.clone());
                                }
                            }
                        }
                    }
                } else {
                    // Navigate deeper
                    current = current
                        .entry(seg.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .ok_or_else(|| anyhow::anyhow!("Expected object at path segment: {}", seg))?;
                }
            }
        } else {
            // Simple non-array path
            let mut current = result;

            for (i, seg) in segments.iter().enumerate() {
                if i == segments.len() - 1 {
                    // Last segment - insert the value
                    current.insert(seg.to_string(), value.clone());
                } else {
                    // Navigate deeper, creating objects as needed
                    current = current
                        .entry(seg.to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()))
                        .as_object_mut()
                        .ok_or_else(|| anyhow::anyhow!("Expected object at path segment: {}", seg))?;
                }
            }
        }

        Ok(())
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

    /// Execute a DataFusion query from a synchronous context (for use in scripts).
    /// Uses block_in_place to safely call async code from sync context.
    #[allow(dead_code)]
    fn execute_datafusion_query_blocking(
        &self,
        sql: &str,
        df_config: &DataFusionConfig,
    ) -> Result<Vec<Value>> {
        let datafusion_handler = self.datafusion_handler.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DataFusion not configured"))?;

        let settings = self.settings.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Settings not configured for DataFusion queries"))?;

        // Use block_in_place to run async code from sync context
        let result = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                // Get the data lake config from settings
                let settings_guard = settings.read().await;
                let data_lake = settings_guard.data_lakes.iter()
                    .find(|dl| dl.name == df_config.data_lake)
                    .ok_or_else(|| anyhow::anyhow!("Data lake '{}' not found", df_config.data_lake))?;

                // Register the table and execute query
                let table_name = datafusion_handler
                    .register_data_lake_table(data_lake, &df_config.schema_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to register table: {}", e))?;

                // Replace $table placeholder with actual table name
                let actual_sql = sql.replace("$table", &table_name);

                // Execute the query
                let query_result = datafusion_handler
                    .execute_sql(&actual_sql)
                    .await
                    .map_err(|e| anyhow::anyhow!("DataFusion query error: {}", e))?;

                Ok::<Vec<Value>, anyhow::Error>(query_result.rows)
            })
        })?;

        Ok(result)
    }

    fn generate_script(&self, config: &MockConfig, args: Option<&Value>) -> Result<Value> {
        if let Some(script) = &config.script {
            // Get DataFusion config if available
            let df_config = config.database.as_ref()
                .and_then(|db| db.datafusion.clone());

            match config.script_lang.as_ref().unwrap_or(&ScriptLang::Rhai) {
                ScriptLang::Rhai => self.generate_script_rhai(script, args, df_config.as_ref()),
                ScriptLang::Lua => self.generate_script_lua(script, args, df_config.as_ref()),
                ScriptLang::Js => self.generate_script_js(script, args, df_config.as_ref()),
                ScriptLang::Python => self.generate_script_python(script, args, df_config.as_ref()),
            }
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_script_rhai(
        &self,
        script: &str,
        args: Option<&Value>,
        df_config: Option<&DataFusionConfig>,
    ) -> Result<Value> {
        // Create a new engine for this execution to register the datafusion_query function
        let mut engine = Engine::new();

        // Register standard faker functions
        engine.register_fn("fake_name", || Name().fake::<String>());
        engine.register_fn("fake_email", || SafeEmail().fake::<String>());
        engine.register_fn("fake_sentence", || Sentence(1..10).fake::<String>());

        // Register datafusion_query if configured
        if let Some(config) = df_config {
            if self.datafusion_handler.is_some() && self.settings.is_some() {
                let df_handler = self.datafusion_handler.clone().unwrap();
                let settings = self.settings.clone().unwrap();
                let config_clone = config.clone();

                engine.register_fn("datafusion_query", move |sql: String| -> rhai::Array {
                    let result = tokio::task::block_in_place(|| {
                        let handle = tokio::runtime::Handle::current();
                        handle.block_on(async {
                            let settings_guard = settings.read().await;
                            let data_lake = settings_guard.data_lakes.iter()
                                .find(|dl| dl.name == config_clone.data_lake);

                            if let Some(dl) = data_lake {
                                let table_name = df_handler
                                    .register_data_lake_table(dl, &config_clone.schema_name)
                                    .await
                                    .ok()?;

                                let actual_sql = sql.replace("$table", &table_name);
                                let query_result = df_handler.execute_sql(&actual_sql).await.ok()?;

                                Some(query_result.rows)
                            } else {
                                None
                            }
                        })
                    });

                    match result {
                        Some(rows) => {
                            rows.into_iter()
                                .filter_map(|v| serde_json::from_value::<rhai::Dynamic>(v).ok())
                                .collect()
                        }
                        None => rhai::Array::new(),
                    }
                });
            }
        }

        let mut scope = Scope::new();

        if let Some(args_val) = args {
            let args_dynamic = serde_json::from_value::<rhai::Dynamic>(args_val.clone())?;
            scope.push("input", args_dynamic);
        }

        let result = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script)?;
        let json_val = serde_json::to_value(&result)?;
        Ok(json_val)
    }

    fn generate_script_lua(
        &self,
        script: &str,
        args: Option<&Value>,
        df_config: Option<&DataFusionConfig>,
    ) -> Result<Value> {
        let lua = mlua::Lua::new();

        if let Some(args_val) = args {
            let lua_val = lua.to_value(args_val)?;
            lua.globals().set("input", lua_val)?;
        }

        // Register datafusion_query if configured
        if let Some(config) = df_config {
            if self.datafusion_handler.is_some() && self.settings.is_some() {
                let df_handler = self.datafusion_handler.clone().unwrap();
                let settings = self.settings.clone().unwrap();
                let config_clone = config.clone();

                let query_fn = lua.create_function(move |lua_ctx, sql: String| {
                    let result = tokio::task::block_in_place(|| {
                        let handle = tokio::runtime::Handle::current();
                        handle.block_on(async {
                            let settings_guard = settings.read().await;
                            let data_lake = settings_guard.data_lakes.iter()
                                .find(|dl| dl.name == config_clone.data_lake);

                            if let Some(dl) = data_lake {
                                let table_name = df_handler
                                    .register_data_lake_table(dl, &config_clone.schema_name)
                                    .await
                                    .ok()?;

                                let actual_sql = sql.replace("$table", &table_name);
                                let query_result = df_handler.execute_sql(&actual_sql).await.ok()?;

                                Some(query_result.rows)
                            } else {
                                None
                            }
                        })
                    });

                    match result {
                        Some(rows) => {
                            let lua_table = lua_ctx.create_table()?;
                            for (i, row) in rows.iter().enumerate() {
                                let row_val = lua_ctx.to_value(row)?;
                                lua_table.set(i + 1, row_val)?;
                            }
                            Ok(mlua::Value::Table(lua_table))
                        }
                        None => Ok(mlua::Value::Table(lua_ctx.create_table()?)),
                    }
                })?;

                lua.globals().set("datafusion_query", query_fn)?;
            }
        }

        let chunk = lua.load(script);
        let result: mlua::Value = chunk.eval()?;
        let json_val = serde_json::to_value(&result)?;
        Ok(json_val)
    }

    fn generate_script_js(
        &self,
        script: &str,
        args: Option<&Value>,
        df_config: Option<&DataFusionConfig>,
    ) -> Result<Value> {
        use boa_engine::{Context, Source};

        let mut context = Context::default();

        if let Some(args_val) = args {
            let json_str = serde_json::to_string(args_val)?;
            let setup_script = format!("const input = JSON.parse('{}');", json_str);
            context.eval(Source::from_bytes(setup_script.as_bytes()))
               .map_err(|e| anyhow::anyhow!("JS setup error: {}", e))?;
        }

        // For JavaScript, we pre-execute the DataFusion query and inject the results
        // as a global variable. The script can access _datafusion_data directly.
        // This is a simpler approach since Boa's closure capture requirements are strict.
        if let Some(config) = df_config {
            if let Some(df_handler) = &self.datafusion_handler {
                if let Some(settings) = &self.settings {
                    let query_result = tokio::task::block_in_place(|| {
                        let handle = tokio::runtime::Handle::current();
                        handle.block_on(async {
                            let settings_guard = settings.read().await;
                            let data_lake = settings_guard.data_lakes.iter()
                                .find(|dl| dl.name == config.data_lake);

                            if let Some(dl) = data_lake {
                                let table_name = df_handler
                                    .register_data_lake_table(dl, &config.schema_name)
                                    .await
                                    .ok()?;

                                Some((table_name, df_handler.clone()))
                            } else {
                                None
                            }
                        })
                    });

                    // Create a wrapper function that scripts can call
                    // datafusion_query(sql) becomes a synchronous call using the pre-registered table
                    if let Some((table_name, df_h)) = query_result {
                        let table_name_clone = table_name.clone();
                        let df_h_clone = df_h.clone();

                        // Inject a helper that uses eval to execute queries
                        let setup_datafusion = format!(
                            r#"
                            const _df_table_name = "{}";
                            function datafusion_query(sql) {{
                                // This is a placeholder - actual implementation needs native bridge
                                // For now, return empty array
                                return [];
                            }}
                            "#,
                            table_name_clone
                        );
                        context.eval(Source::from_bytes(setup_datafusion.as_bytes()))
                            .map_err(|e| anyhow::anyhow!("JS DataFusion setup error: {}", e))?;

                        // Store handlers for potential later use (though Boa limits this)
                        let _ = (df_h_clone, table_name);
                    }
                }
            }
        }

        let result = context.eval(Source::from_bytes(script.as_bytes()))
            .map_err(|e| anyhow::anyhow!("JS execution error: {}", e))?;

        let json_val = result.to_json(&mut context)
            .map_err(|e| anyhow::anyhow!("JS result conversion error: {}", e))?;

        Ok(json_val)
    }

    fn generate_script_python(
        &self,
        script: &str,
        args: Option<&Value>,
        df_config: Option<&DataFusionConfig>,
    ) -> Result<Value> {
        use rustpython_vm::Interpreter;
        use rustpython_vm::compiler::Mode;
        use rustpython_vm::PyObjectRef;

        // Clone what we need for the closure
        let df_handler = self.datafusion_handler.clone();
        let settings = self.settings.clone();
        let config_clone = df_config.cloned();

        let interpreter = Interpreter::without_stdlib(Default::default());

        interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            if let Some(args_val) = args {
                let py_val = self.json_to_python(vm, args_val);
                scope.globals.set_item("input", py_val, vm).map_err(|e| self.map_py_err(vm, e))?;
            }

            // Register datafusion_query if configured
            if let Some(config) = config_clone {
                if df_handler.is_some() && settings.is_some() {
                    let df_handler = df_handler.clone().unwrap();
                    let settings = settings.clone().unwrap();

                    // Create a Python function for datafusion_query
                    let query_fn = vm.new_function(
                        "datafusion_query",
                        move |args: rustpython_vm::function::FuncArgs, vm: &rustpython_vm::VirtualMachine| -> PyObjectRef {
                            use rustpython_vm::builtins::PyStr;

                            let sql = args.args.first()
                                .and_then(|arg| arg.payload::<PyStr>())
                                .map(|s| s.as_str().to_string())
                                .unwrap_or_default();

                            let df_handler_clone = df_handler.clone();
                            let settings_clone = settings.clone();
                            let config_clone2 = config.clone();

                            let result = tokio::task::block_in_place(|| {
                                let handle = tokio::runtime::Handle::current();
                                handle.block_on(async {
                                    let settings_guard = settings_clone.read().await;
                                    let data_lake = settings_guard.data_lakes.iter()
                                        .find(|dl| dl.name == config_clone2.data_lake);

                                    if let Some(dl) = data_lake {
                                        let table_name = df_handler_clone
                                            .register_data_lake_table(dl, &config_clone2.schema_name)
                                            .await
                                            .ok()?;

                                        let actual_sql = sql.replace("$table", &table_name);
                                        let query_result = df_handler_clone.execute_sql(&actual_sql).await.ok()?;

                                        Some(query_result.rows)
                                    } else {
                                        None
                                    }
                                })
                            });

                            match result {
                                Some(rows) => {
                                    // Convert rows to Python list of dicts
                                    let elements: Vec<_> = rows.iter().map(|row| {
                                        json_to_python_static(vm, row)
                                    }).collect();
                                    vm.ctx.new_list(elements).into()
                                }
                                None => vm.ctx.new_list(vec![]).into(),
                            }
                        },
                    );

                    scope.globals.set_item("datafusion_query", query_fn.into(), vm)
                        .map_err(|e| self.map_py_err(vm, e))?;
                }
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

            // Try to parse as JSON array first
            let data: Vec<Value> = match serde_json::from_str(&content) {
                Ok(arr) => arr,
                Err(_) => {
                    // Try parsing as JSON Lines (one JSON object per line)
                    let lines: Vec<Value> = content
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .filter_map(|line| serde_json::from_str(line).ok())
                        .collect();

                    if lines.is_empty() {
                        // Return raw content as string if not JSON
                        return Ok(Value::String(content));
                    }
                    lines
                }
            };

            if data.is_empty() {
                return Ok(Value::Null);
            }

            // Select based on strategy
            let selected = match file_config.selection.as_str() {
                "random" => {
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    let idx = rng.gen_range(0..data.len());
                    data[idx].clone()
                }
                "sequential" => {
                    // Use state manager for sequential selection
                    let state_key = format!("__file_seq_{}", file_config.path);
                    let current_idx = self.state_manager.increment(&state_key).await;
                    // Wrap around using modulo
                    let idx = ((current_idx - 1) as usize) % data.len();
                    data[idx].clone()
                }
                "first" => data[0].clone(),
                "last" => data[data.len() - 1].clone(),
                _ => data[0].clone(),
            };

            Ok(selected)
        } else {
            Ok(Value::Null)
        }
    }

    fn generate_pattern(&self, config: &MockConfig) -> Result<Value> {
        if let Some(pattern) = &config.pattern {
            let result = self.expand_pattern(pattern)?;
            Ok(json!(result))
        } else {
            Ok(Value::Null)
        }
    }

    /// Expand a pattern string into a generated value.
    /// Supports:
    /// - `\d` - random digit (0-9)
    /// - `\D` - random non-digit (a-z)
    /// - `\w` - random word character (a-zA-Z0-9_)
    /// - `\W` - random non-word character (space, punctuation)
    /// - `\a` - random lowercase letter (a-z)
    /// - `\A` - random uppercase letter (A-Z)
    /// - `\x` - random hex digit (0-9a-f)
    /// - `\X` - random uppercase hex digit (0-9A-F)
    /// - `\s` - space character
    /// - `\n` - newline character
    /// - `{n}` - repeat previous pattern n times
    /// - `{n,m}` - repeat previous pattern n to m times
    /// - `[abc]` - one of the characters in brackets
    /// - `[a-z]` - one character from range
    /// - `\\` - literal backslash
    fn expand_pattern(&self, pattern: &str) -> Result<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut result = String::new();
        let mut chars = pattern.chars().peekable();
        let mut last_char: Option<char> = None;

        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    if let Some(next) = chars.next() {
                        let generated = match next {
                            'd' => rng.gen_range(0..10).to_string(),
                            'D' => (rng.gen_range(b'a'..=b'z') as char).to_string(),
                            'w' => {
                                let choices: Vec<char> = ('a'..='z')
                                    .chain('A'..='Z')
                                    .chain('0'..='9')
                                    .chain(std::iter::once('_'))
                                    .collect();
                                choices[rng.gen_range(0..choices.len())].to_string()
                            }
                            'W' => {
                                let choices = [' ', '!', '@', '#', '$', '%', '^', '&', '*'];
                                choices[rng.gen_range(0..choices.len())].to_string()
                            }
                            'a' => (rng.gen_range(b'a'..=b'z') as char).to_string(),
                            'A' => (rng.gen_range(b'A'..=b'Z') as char).to_string(),
                            'x' => {
                                let hex_chars: Vec<char> = ('0'..='9').chain('a'..='f').collect();
                                hex_chars[rng.gen_range(0..hex_chars.len())].to_string()
                            }
                            'X' => {
                                let hex_chars: Vec<char> = ('0'..='9').chain('A'..='F').collect();
                                hex_chars[rng.gen_range(0..hex_chars.len())].to_string()
                            }
                            's' => " ".to_string(),
                            'n' => "\n".to_string(),
                            't' => "\t".to_string(),
                            '\\' => "\\".to_string(),
                            _ => next.to_string(),
                        };
                        for c in generated.chars() {
                            result.push(c);
                            last_char = Some(c);
                        }
                    }
                }
                '[' => {
                    // Character class [abc] or [a-z]
                    let mut class_chars: Vec<char> = Vec::new();
                    while let Some(&next) = chars.peek() {
                        if next == ']' {
                            chars.next();
                            break;
                        }
                        let c = chars.next().unwrap();
                        if chars.peek() == Some(&'-') {
                            chars.next(); // consume '-'
                            if let Some(&end) = chars.peek() {
                                if end != ']' {
                                    chars.next();
                                    // Range like a-z
                                    for range_char in c..=end {
                                        class_chars.push(range_char);
                                    }
                                    continue;
                                }
                            }
                        }
                        class_chars.push(c);
                    }
                    if !class_chars.is_empty() {
                        let selected = class_chars[rng.gen_range(0..class_chars.len())];
                        result.push(selected);
                        last_char = Some(selected);
                    }
                }
                '{' => {
                    // Repetition {n} or {n,m}
                    let mut rep_str = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == '}' {
                            chars.next();
                            break;
                        }
                        rep_str.push(chars.next().unwrap());
                    }

                    let (min, max) = if rep_str.contains(',') {
                        let parts: Vec<&str> = rep_str.split(',').collect();
                        let min_val = parts[0].trim().parse::<usize>().unwrap_or(1);
                        let max_val = parts.get(1)
                            .and_then(|s| s.trim().parse::<usize>().ok())
                            .unwrap_or(min_val);
                        (min_val, max_val)
                    } else {
                        let n = rep_str.trim().parse::<usize>().unwrap_or(1);
                        (n, n)
                    };

                    let repeat_count = if min == max {
                        min
                    } else {
                        rng.gen_range(min..=max)
                    };

                    // Repeat the last character
                    if let Some(c) = last_char {
                        // We already have one instance, add repeat_count - 1 more
                        for _ in 1..repeat_count {
                            result.push(c);
                        }
                    }
                }
                _ => {
                    result.push(ch);
                    last_char = Some(ch);
                }
            }
        }

        Ok(result)
    }

    // ==================== DataLakeCrud Strategy ====================

    async fn generate_data_lake_crud(
        &self,
        config: &MockConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let crud_config = config.data_lake_crud.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DataLakeCrud config not provided"))?;

        let settings = self.settings.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Settings not available for DataLakeCrud"))?;

        // Validate data lake exists
        let data_lake = {
            let settings_guard = settings.read().await;
            settings_guard.data_lakes.iter()
                .find(|dl| dl.name == crud_config.data_lake)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Data lake '{}' not found", crud_config.data_lake))?
        };

        // Validate schema exists in data lake
        if !data_lake.schemas.iter().any(|s|
            s.schema_name == crud_config.schema_name ||
            s.alias.as_ref() == Some(&crud_config.schema_name)
        ) {
            return Err(anyhow::anyhow!(
                "Schema '{}' not found in data lake '{}'",
                crud_config.schema_name, crud_config.data_lake
            ));
        }

        match crud_config.operation {
            DataLakeCrudOperation::Create =>
                self.crud_create(crud_config, args).await,
            DataLakeCrudOperation::ReadById =>
                self.crud_read_by_id(crud_config, args).await,
            DataLakeCrudOperation::ReadAll =>
                self.crud_read_all(crud_config, args).await,
            DataLakeCrudOperation::ReadFilter =>
                self.crud_read_filter(crud_config, args).await,
            DataLakeCrudOperation::Update =>
                self.crud_update(crud_config, args).await,
            DataLakeCrudOperation::Delete =>
                self.crud_delete(crud_config, args).await,
        }
    }

    async fn crud_create(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let data = args.cloned().unwrap_or(json!({}));

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud Create"))?;

        // Create record
        let record = DataRecord::new(&config.data_lake, &config.schema_name, data)
            .with_source("mock:data_lake_crud");

        // Get data lake format
        let format = self.get_data_lake_format(&config.data_lake).await?;

        // Write record
        file_storage.write_records(&config.data_lake, &config.schema_name, &[record.clone()], &format).await
            .map_err(|e| anyhow::anyhow!("Failed to create record: {}", e))?;

        Ok(serde_json::to_value(&record)?)
    }

    async fn crud_read_by_id(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let id = args
            .and_then(|a| a.get(&config.id_field))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing '{}' field in input", config.id_field))?;

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud ReadById"))?;

        match file_storage.find_record(&config.data_lake, &config.schema_name, id).await {
            Ok(Some(record)) => Ok(serde_json::to_value(&record)?),
            Ok(None) => Ok(Value::Null),
            Err(e) => Err(anyhow::anyhow!("Failed to find record: {}", e)),
        }
    }

    async fn crud_read_all(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let limit = args
            .and_then(|a| a.get("limit"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(config.read_limit);

        let offset = args
            .and_then(|a| a.get("offset"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(0);

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud ReadAll"))?;

        let records = file_storage.read_active_records(&config.data_lake, &config.schema_name).await
            .map_err(|e| anyhow::anyhow!("Failed to read records: {}", e))?;

        let paginated: Vec<_> = records.into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok(serde_json::to_value(&paginated)?)
    }

    async fn crud_read_filter(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        // Build filter from template if provided
        let filter: Option<serde_json::Map<String, Value>> = if let Some(template) = &config.filter_template {
            let mut context = tera::Context::new();
            if let Some(args_val) = args {
                if let Some(obj) = args_val.as_object() {
                    for (k, v) in obj {
                        context.insert(k, v);
                    }
                }
            }
            let rendered = tera::Tera::one_off(template, &context, false)
                .map_err(|e| anyhow::anyhow!("Filter template error: {}", e))?;
            serde_json::from_str(&rendered).ok()
        } else {
            args.and_then(|a| a.as_object()).cloned()
        };

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud ReadFilter"))?;

        let records = file_storage.read_active_records(&config.data_lake, &config.schema_name).await
            .map_err(|e| anyhow::anyhow!("Failed to read records: {}", e))?;

        // Apply filter
        let filtered: Vec<_> = if let Some(filter) = filter {
            records.into_iter().filter(|record| {
                filter.iter().all(|(key, expected)| {
                    record.data.get(key).map(|v| v == expected).unwrap_or(false)
                })
            }).collect()
        } else {
            records
        };

        // Apply limit
        let limited: Vec<_> = filtered.into_iter()
            .take(config.read_limit)
            .collect();

        Ok(serde_json::to_value(&limited)?)
    }

    async fn crud_update(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let args_obj = args
            .and_then(|a| a.as_object())
            .ok_or_else(|| anyhow::anyhow!("Input must be an object"))?;

        let id = args_obj.get(&config.id_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing '{}' field in input", config.id_field))?;

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud Update"))?;

        // Find existing record
        let existing = file_storage.find_record(&config.data_lake, &config.schema_name, id).await
            .map_err(|e| anyhow::anyhow!("Failed to find record: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Record not found: {}", id))?;

        // Merge new data into existing (exclude id_field)
        let mut updated = existing;
        if let Some(obj) = updated.data.as_object_mut() {
            for (k, v) in args_obj {
                if k != &config.id_field {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }

        // Get data lake format
        let format = self.get_data_lake_format(&config.data_lake).await?;

        // Update record
        let result = file_storage.update_record(&config.data_lake, &config.schema_name, id, updated, &format).await
            .map_err(|e| anyhow::anyhow!("Failed to update record: {}", e))?;

        Ok(serde_json::to_value(&result)?)
    }

    async fn crud_delete(
        &self,
        config: &DataLakeCrudConfig,
        args: Option<&serde_json::Value>,
    ) -> Result<Value> {
        let id = args
            .and_then(|a| a.get(&config.id_field))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing '{}' field in input", config.id_field))?;

        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not available for DataLakeCrud Delete"))?;

        file_storage.soft_delete_record(&config.data_lake, &config.schema_name, id).await
            .map_err(|e| anyhow::anyhow!("Failed to delete record: {}", e))?;

        Ok(json!({"success": true, "deleted_id": id}))
    }

    async fn get_data_lake_format(&self, data_lake: &str) -> Result<DataLakeFileFormat> {
        if let Some(settings) = &self.settings {
            let settings_guard = settings.read().await;
            if let Some(dl) = settings_guard.data_lakes.iter().find(|d| d.name == data_lake) {
                return Ok(dl.file_format.clone());
            }
        }
        Ok(DataLakeFileFormat::default())
    }
}

/// Static helper to convert JSON to Python objects (for use in closures)
fn json_to_python_static(vm: &rustpython_vm::VirtualMachine, value: &Value) -> rustpython_vm::PyObjectRef {
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
            let elements: Vec<_> = arr.iter().map(|v| json_to_python_static(vm, v)).collect();
            vm.ctx.new_list(elements).into()
        }
        Value::Object(obj) => {
            let dict = vm.ctx.new_dict();
            for (k, v) in obj {
                let py_k = vm.ctx.new_str(k.as_str());
                let py_v = json_to_python_static(vm, v);
                let _ = dict.set_item(py_k.as_object(), py_v, vm);
            }
            dict.into()
        }
    }
}
