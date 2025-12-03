//! SQL Editor Component
//!
//! A SQL editor with:
//! - Syntax highlighting for SQL keywords
//! - Table name autocompletion
//! - Support for data_lake.schema table naming

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use js_sys;

/// SQL keywords for highlighting
const SQL_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "IS", "NULL", "LIKE",
    "BETWEEN", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "FULL", "CROSS", "ON",
    "AS", "ORDER", "BY", "ASC", "DESC", "GROUP", "HAVING", "LIMIT", "OFFSET",
    "UNION", "ALL", "DISTINCT", "COUNT", "SUM", "AVG", "MIN", "MAX", "CASE",
    "WHEN", "THEN", "ELSE", "END", "CAST", "COALESCE", "NULLIF", "EXISTS",
    "TRUE", "FALSE", "CREATE", "INSERT", "UPDATE", "DELETE", "DROP", "ALTER",
    "TABLE", "INDEX", "VIEW", "WITH", "RECURSIVE", "OVER", "PARTITION", "ROW",
    "ROWS", "RANGE", "UNBOUNDED", "PRECEDING", "FOLLOWING", "CURRENT",
];

/// SQL functions for highlighting
const SQL_FUNCTIONS: &[&str] = &[
    "json_get", "json_get_str", "json_get_int", "json_get_float", "json_get_bool",
    "json_get_json", "json_as_text", "json_contains", "json_length", "json_object_keys",
    "LOWER", "UPPER", "TRIM", "LTRIM", "RTRIM", "LENGTH", "SUBSTRING", "REPLACE",
    "CONCAT", "COALESCE", "NULLIF", "ABS", "CEIL", "FLOOR", "ROUND", "POWER",
    "SQRT", "MOD", "NOW", "DATE", "TIME", "TIMESTAMP", "EXTRACT", "DATE_PART",
];

/// Token types for SQL syntax highlighting
#[derive(Clone, Debug, PartialEq)]
enum SqlToken {
    Keyword(String),
    Function(String),
    String(String),
    Number(String),
    Comment(String),
    Operator(String),
    Identifier(String),
    TableRef(String),      // datalake.schema style references
    Placeholder(String),   // $table
    Whitespace(String),
    Punctuation(String),
}

impl SqlToken {
    fn css_class(&self) -> &'static str {
        match self {
            SqlToken::Keyword(_) => "sql-keyword",
            SqlToken::Function(_) => "sql-function",
            SqlToken::String(_) => "sql-string",
            SqlToken::Number(_) => "sql-number",
            SqlToken::Comment(_) => "sql-comment",
            SqlToken::Operator(_) => "sql-operator",
            SqlToken::Identifier(_) => "sql-identifier",
            SqlToken::TableRef(_) => "sql-table-ref",
            SqlToken::Placeholder(_) => "sql-placeholder",
            SqlToken::Whitespace(_) => "",
            SqlToken::Punctuation(_) => "sql-punctuation",
        }
    }

    fn text(&self) -> &str {
        match self {
            SqlToken::Keyword(s) => s,
            SqlToken::Function(s) => s,
            SqlToken::String(s) => s,
            SqlToken::Number(s) => s,
            SqlToken::Comment(s) => s,
            SqlToken::Operator(s) => s,
            SqlToken::Identifier(s) => s,
            SqlToken::TableRef(s) => s,
            SqlToken::Placeholder(s) => s,
            SqlToken::Whitespace(s) => s,
            SqlToken::Punctuation(s) => s,
        }
    }
}

/// Tokenize SQL for syntax highlighting
fn tokenize_sql(sql: &str) -> Vec<SqlToken> {
    let mut tokens = Vec::new();
    let mut chars = sql.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // Whitespace
            ' ' | '\t' | '\n' | '\r' => {
                let mut ws = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                        ws.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(SqlToken::Whitespace(ws));
            }
            // Single-quoted strings
            '\'' => {
                let mut s = String::new();
                s.push(chars.next().unwrap());
                let mut escaped = false;
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if escaped {
                        escaped = false;
                    } else if c == '\'' {
                        // Check for escaped quote ('')
                        if chars.peek() == Some(&'\'') {
                            s.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                tokens.push(SqlToken::String(s));
            }
            // Double-quoted identifiers
            '"' => {
                let mut s = String::new();
                s.push(chars.next().unwrap());
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if c == '"' {
                        break;
                    }
                }
                tokens.push(SqlToken::Identifier(s));
            }
            // Comments (-- single line)
            '-' if chars.clone().nth(1) == Some('-') => {
                let mut comment = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    comment.push(chars.next().unwrap());
                }
                tokens.push(SqlToken::Comment(comment));
            }
            // Placeholder ($table)
            '$' => {
                let mut placeholder = String::new();
                placeholder.push(chars.next().unwrap());
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        placeholder.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(SqlToken::Placeholder(placeholder));
            }
            // Numbers
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(SqlToken::Number(num));
            }
            // Operators
            '=' | '<' | '>' | '!' | '+' | '*' | '/' | '%' | '|' | '&' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());
                // Handle multi-char operators like <=, >=, <>, !=, ||, &&
                if let Some(&next) = chars.peek() {
                    if (ch == '<' && (next == '=' || next == '>'))
                        || (ch == '>' && next == '=')
                        || (ch == '!' && next == '=')
                        || (ch == '|' && next == '|')
                        || (ch == '&' && next == '&')
                    {
                        op.push(chars.next().unwrap());
                    }
                }
                tokens.push(SqlToken::Operator(op));
            }
            // Minus (could be operator or negative number)
            '-' => {
                let mut s = String::new();
                s.push(chars.next().unwrap());
                // Check if followed by digit
                if chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() || c == '.' {
                            s.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    tokens.push(SqlToken::Number(s));
                } else {
                    tokens.push(SqlToken::Operator(s));
                }
            }
            // Punctuation
            '(' | ')' | ',' | ';' => {
                tokens.push(SqlToken::Punctuation(chars.next().unwrap().to_string()));
            }
            // Identifiers and keywords (including table.column references)
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(chars.next().unwrap());
                    } else if c == '.' {
                        // This might be a table.column reference
                        ident.push(chars.next().unwrap());
                        // Continue reading after the dot
                        while let Some(&c) = chars.peek() {
                            if c.is_alphanumeric() || c == '_' {
                                ident.push(chars.next().unwrap());
                            } else if c == '.' {
                                // Another dot (datalake.schema.column)
                                ident.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }

                let upper = ident.to_uppercase();

                // Check if it's a table reference (contains a dot)
                if ident.contains('.') {
                    tokens.push(SqlToken::TableRef(ident));
                } else if SQL_KEYWORDS.contains(&upper.as_str()) {
                    tokens.push(SqlToken::Keyword(ident));
                } else if SQL_FUNCTIONS.iter().any(|f| f.eq_ignore_ascii_case(&ident)) {
                    tokens.push(SqlToken::Function(ident));
                } else {
                    tokens.push(SqlToken::Identifier(ident));
                }
            }
            // Any other character
            _ => {
                tokens.push(SqlToken::Punctuation(chars.next().unwrap().to_string()));
            }
        }
    }

    tokens
}

/// Generate HTML with syntax highlighting
fn highlight_sql(sql: &str) -> String {
    let tokens = tokenize_sql(sql);
    let mut html = String::new();

    for token in tokens {
        let text = html_escape(token.text());
        let class = token.css_class();
        if class.is_empty() {
            html.push_str(&text);
        } else {
            html.push_str(&format!("<span class=\"{}\">{}</span>", class, text));
        }
    }

    html
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Table reference for autocompletion
#[derive(Clone, Debug, PartialEq)]
pub struct TableRef {
    pub data_lake: String,
    pub schema: String,
    pub display: String,
}

impl TableRef {
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.data_lake, self.schema)
    }
}

/// Field reference for autocompletion (from JSON schema properties)
#[derive(Clone, Debug, PartialEq)]
pub struct FieldRef {
    pub name: String,
    pub field_type: String,
    pub schema_name: String,
}

/// Extract field names from a JSON Schema
pub fn extract_fields_from_schema(schema: &serde_json::Value, schema_name: &str) -> Vec<FieldRef> {
    let mut fields = Vec::new();

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let field_type = prop.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("any")
                .to_string();
            fields.push(FieldRef {
                name: name.clone(),
                field_type,
                schema_name: schema_name.to_string(),
            });
        }
    }

    // Also handle nested properties for complex schemas
    if let Some(defs) = schema.get("$defs").or(schema.get("definitions")).and_then(|d| d.as_object()) {
        for (def_name, def) in defs {
            if let Some(props) = def.get("properties").and_then(|p| p.as_object()) {
                for (name, prop) in props {
                    let field_type = prop.get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("any")
                        .to_string();
                    fields.push(FieldRef {
                        name: format!("{}.{}", def_name, name),
                        field_type,
                        schema_name: schema_name.to_string(),
                    });
                }
            }
        }
    }

    fields
}

/// Autocomplete item with type information
#[derive(Clone, Debug)]
pub struct AutocompleteItem {
    pub text: String,
    pub item_type: AutocompleteType,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AutocompleteType {
    Keyword,
    Function,
    Table,
    Field,
    Column,
    Placeholder,
}

/// SQL Editor component with syntax highlighting and autocompletion
#[component]
pub fn SqlEditor(
    /// The SQL query value (read/write signal)
    value: RwSignal<String>,
    /// Available tables for autocompletion
    #[prop(into)]
    tables: Signal<Vec<TableRef>>,
    /// Available fields from schemas for autocompletion
    #[prop(into, optional)]
    fields: Option<Signal<Vec<FieldRef>>>,
    /// Placeholder text
    #[prop(default = "SELECT * FROM $table WHERE ...")]
    placeholder: &'static str,
    /// Number of rows to display
    #[prop(default = 6)]
    rows: usize,
) -> impl IntoView {
    // Autocompletion state
    let (show_autocomplete, set_show_autocomplete) = signal(false);
    let (autocomplete_items, set_autocomplete_items) = signal(Vec::<AutocompleteItem>::new());
    let (selected_index, set_selected_index) = signal(0usize);
    let (cursor_position, set_cursor_position) = signal(0usize);
    let (autocomplete_trigger_pos, set_autocomplete_trigger_pos) = signal(0usize);
    let (_in_string_context, set_in_string_context) = signal(false);

    // Reference to the textarea
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Standard column names available in all data lake tables
    let standard_columns = vec![
        ("id", "string"),
        ("data", "json"),
        ("data_lake", "string"),
        ("schema_name", "string"),
        ("created_at", "timestamp"),
        ("updated_at", "timestamp"),
        ("created_by", "string"),
        ("metadata", "json"),
    ];

    // Check if cursor is inside a string (for field suggestions)
    let is_in_string = |text: &str, pos: usize| -> (bool, usize) {
        let before = &text[..pos.min(text.len())];
        let mut in_single = false;
        let mut string_start = 0;

        for (i, c) in before.char_indices() {
            if c == '\'' && !in_single {
                in_single = true;
                string_start = i + 1;
            } else if c == '\'' && in_single {
                in_single = false;
            }
        }

        (in_single, string_start)
    };

    // Check if we're in a JSON function context (after 'data,' in a json_get_* call)
    let is_json_field_context = |text: &str, pos: usize| -> bool {
        let before = &text[..pos.min(text.len())].to_lowercase();
        // Look for patterns like "json_get_str(data, '" or "json_get(data, '"
        let json_funcs = ["json_get", "json_get_str", "json_get_int", "json_get_float", "json_get_bool", "json_get_json"];
        for func in json_funcs {
            if let Some(func_pos) = before.rfind(func) {
                let after_func = &before[func_pos..];
                // Check if we have "(data," pattern followed by quote
                if after_func.contains("(data,") || after_func.contains("( data,") {
                    return true;
                }
            }
        }
        false
    };

    // Update autocomplete suggestions based on context
    let update_autocomplete = move |text: &str, pos: usize| {
        let before_cursor = &text[..pos.min(text.len())];

        // Check if we're inside a string (for field name suggestions)
        let (in_string, string_start) = is_in_string(text, pos);
        set_in_string_context.set(in_string);

        if in_string && is_json_field_context(text, pos) {
            // We're inside a string in a JSON function - suggest field names
            let current_field = &before_cursor[string_start..];
            let current_lower = current_field.to_lowercase();

            let mut suggestions: Vec<AutocompleteItem> = Vec::new();

            // Add matching fields from schemas
            if let Some(fields_signal) = fields {
                for field in fields_signal.get().iter() {
                    if field.name.to_lowercase().starts_with(&current_lower) || current_field.is_empty() {
                        suggestions.push(AutocompleteItem {
                            text: field.name.clone(),
                            item_type: AutocompleteType::Field,
                            detail: Some(format!("{} ({})", field.field_type, field.schema_name)),
                        });
                    }
                }
            }

            // Limit and show
            suggestions.truncate(15);
            if !suggestions.is_empty() {
                set_autocomplete_items.set(suggestions);
                set_autocomplete_trigger_pos.set(string_start);
                set_selected_index.set(0);
                set_show_autocomplete.set(true);
            } else {
                set_show_autocomplete.set(false);
            }
            return;
        }

        // Normal autocomplete (outside strings)
        // Find the start of the current word
        let word_start = before_cursor
            .rfind(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')')
            .map(|i| i + 1)
            .unwrap_or(0);

        let current_word = &before_cursor[word_start..];

        // Check if we should show autocomplete
        let should_show = !current_word.is_empty() && current_word.len() >= 1;

        if should_show {
            let current_upper = current_word.to_uppercase();
            let current_lower = current_word.to_lowercase();

            let mut suggestions: Vec<AutocompleteItem> = Vec::new();

            // Add matching tables
            for table in tables.get().iter() {
                let full = table.full_name();
                if full.to_lowercase().starts_with(&current_lower)
                    || table.schema.to_lowercase().starts_with(&current_lower) {
                    suggestions.push(AutocompleteItem {
                        text: full,
                        item_type: AutocompleteType::Table,
                        detail: None,
                    });
                }
            }

            // Add matching standard columns
            for (col, col_type) in &standard_columns {
                if col.starts_with(&current_lower) {
                    suggestions.push(AutocompleteItem {
                        text: col.to_string(),
                        item_type: AutocompleteType::Column,
                        detail: Some(col_type.to_string()),
                    });
                }
            }

            // Add matching SQL keywords
            for kw in SQL_KEYWORDS.iter() {
                if kw.starts_with(&current_upper) {
                    suggestions.push(AutocompleteItem {
                        text: kw.to_string(),
                        item_type: AutocompleteType::Keyword,
                        detail: None,
                    });
                }
            }

            // Add matching functions
            for func in SQL_FUNCTIONS.iter() {
                if func.to_uppercase().starts_with(&current_upper) {
                    suggestions.push(AutocompleteItem {
                        text: func.to_string(),
                        item_type: AutocompleteType::Function,
                        detail: None,
                    });
                }
            }

            // Add $table placeholder
            if "$table".starts_with(&current_lower) || current_word == "$" {
                suggestions.push(AutocompleteItem {
                    text: "$table".to_string(),
                    item_type: AutocompleteType::Placeholder,
                    detail: Some("default table".to_string()),
                });
            }

            // Limit suggestions
            suggestions.truncate(12);

            if !suggestions.is_empty() {
                set_autocomplete_items.set(suggestions);
                set_autocomplete_trigger_pos.set(word_start);
                set_selected_index.set(0);
                set_show_autocomplete.set(true);
            } else {
                set_show_autocomplete.set(false);
            }
        } else {
            set_show_autocomplete.set(false);
        }
    };

    // Handle input changes
    let on_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea = target.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        let text = textarea.value();
        let pos = textarea.selection_start().unwrap_or(Some(0)).unwrap_or(0) as usize;

        value.set(text.clone());
        set_cursor_position.set(pos);
        update_autocomplete(&text, pos);
    };

    // Handle key navigation in autocomplete
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if show_autocomplete.get() {
            let items = autocomplete_items.get();
            let current = selected_index.get();

            match ev.key().as_str() {
                "ArrowDown" => {
                    ev.prevent_default();
                    set_selected_index.set((current + 1) % items.len().max(1));
                }
                "ArrowUp" => {
                    ev.prevent_default();
                    set_selected_index.set(if current == 0 { items.len().saturating_sub(1) } else { current - 1 });
                }
                "Tab" | "Enter" => {
                    if !items.is_empty() {
                        ev.prevent_default();
                        // Insert the selected suggestion
                        let suggestion = &items[current].text;
                        let text = value.get();
                        let trigger_pos = autocomplete_trigger_pos.get();
                        let cursor_pos = cursor_position.get();

                        let new_text = format!(
                            "{}{}{}",
                            &text[..trigger_pos],
                            suggestion,
                            &text[cursor_pos.min(text.len())..]
                        );

                        let new_cursor_pos = trigger_pos + suggestion.len();
                        value.set(new_text.clone());
                        set_cursor_position.set(new_cursor_pos);
                        set_show_autocomplete.set(false);

                        // Update cursor position after the DOM updates
                        let textarea_ref_clone = textarea_ref.clone();
                        request_animation_frame(move || {
                            if let Some(textarea) = textarea_ref_clone.get() {
                                let pos = new_cursor_pos as u32;
                                let _ = textarea.set_selection_start(Some(pos));
                                let _ = textarea.set_selection_end(Some(pos));
                                let _ = textarea.focus();
                            }
                        });
                    }
                }
                "Escape" => {
                    ev.prevent_default();
                    set_show_autocomplete.set(false);
                }
                _ => {}
            }
        }
    };

    // Handle clicking on autocomplete item
    let select_autocomplete = move |item: AutocompleteItem| {
        let text = value.get();
        let trigger_pos = autocomplete_trigger_pos.get();
        let cursor_pos = cursor_position.get();
        let item_len = item.text.len();

        let new_text = format!(
            "{}{}{}",
            &text[..trigger_pos],
            item.text,
            &text[cursor_pos.min(text.len())..]
        );

        let new_cursor_pos = trigger_pos + item_len;
        value.set(new_text);
        set_cursor_position.set(new_cursor_pos);
        set_show_autocomplete.set(false);

        // Focus back on textarea and set cursor position after DOM updates
        let textarea_ref_clone = textarea_ref.clone();
        request_animation_frame(move || {
            if let Some(textarea) = textarea_ref_clone.get() {
                let pos = new_cursor_pos as u32;
                let _ = textarea.set_selection_start(Some(pos));
                let _ = textarea.set_selection_end(Some(pos));
                let _ = textarea.focus();
            }
        });
    };

    // Close autocomplete when clicking outside
    let on_blur = move |_| {
        // Delay to allow click on autocomplete item
        set_timeout(
            move || {
                set_show_autocomplete.set(false);
            },
            std::time::Duration::from_millis(200),
        );
    };

    view! {
        <div class="sql-editor-container relative">
            // Editor wrapper with highlighting overlay
            <div class="sql-editor-wrapper relative">
                // Syntax highlighted display (behind textarea)
                <pre
                    class="sql-editor-highlight absolute inset-0 overflow-auto pointer-events-none m-0 p-3 font-mono text-sm leading-relaxed whitespace-pre-wrap break-words"
                    aria-hidden="true"
                    inner_html=move || {
                        let text = value.get();
                        if text.is_empty() {
                            String::new()
                        } else {
                            highlight_sql(&text)
                        }
                    }
                />

                // Actual textarea (transparent text, but handles input)
                <textarea
                    node_ref=textarea_ref
                    class="sql-editor-textarea w-full font-mono text-sm leading-relaxed resize-none border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500 p-3"
                    rows=rows
                    placeholder=placeholder
                    prop:value=move || value.get()
                    on:input=on_input
                    on:keydown=on_keydown
                    on:blur=on_blur
                    spellcheck="false"
                    autocomplete="off"
                    autocapitalize="off"
                />
            </div>

            // Autocompletion dropdown
            <Show when=move || show_autocomplete.get() && !autocomplete_items.get().is_empty()>
                <div class="sql-autocomplete absolute z-50 mt-1 w-72 bg-white border border-gray-300 rounded-lg shadow-lg max-h-60 overflow-auto">
                    {move || {
                        let items = autocomplete_items.get();
                        let selected = selected_index.get();
                        items.into_iter().enumerate().map(|(i, item)| {
                            let item_clone = item.clone();
                            let is_selected = i == selected;

                            let (text_class, type_label, icon) = match &item.item_type {
                                AutocompleteType::Keyword => ("text-blue-600 font-medium", "keyword", "K"),
                                AutocompleteType::Function => ("text-purple-600", "function", "f"),
                                AutocompleteType::Table => ("text-cyan-700 font-semibold", "table", "T"),
                                AutocompleteType::Field => ("text-green-600", "field", "F"),
                                AutocompleteType::Column => ("text-amber-600", "column", "C"),
                                AutocompleteType::Placeholder => ("text-orange-600", "placeholder", "$"),
                            };

                            let detail = item.detail.clone();
                            let display_text = item.text.clone();

                            view! {
                                <div
                                    class=move || format!(
                                        "px-3 py-1.5 cursor-pointer flex items-center gap-2 hover:bg-gray-100 {}",
                                        if is_selected { "bg-cyan-50" } else { "" }
                                    )
                                    on:mousedown=move |_| select_autocomplete(item_clone.clone())
                                >
                                    <span class="w-5 h-5 flex items-center justify-center text-xs font-mono bg-gray-100 rounded text-gray-500">
                                        {icon}
                                    </span>
                                    <div class="flex-1 min-w-0">
                                        <span class=text_class>{display_text}</span>
                                        {detail.map(|d| view! {
                                            <span class="text-xs text-gray-400 ml-2">{d}</span>
                                        })}
                                    </div>
                                    <span class="text-xs text-gray-400">{type_label}</span>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    }}
                </div>
            </Show>
        </div>

        // CSS styles for syntax highlighting
        <style>
            r#"
            .sql-editor-wrapper {
                position: relative;
            }

            .sql-editor-highlight {
                background: transparent;
                color: transparent;
                border: 1px solid transparent;
                box-sizing: border-box;
            }

            .sql-editor-textarea {
                background: rgba(255, 255, 255, 0.95);
                color: transparent;
                caret-color: #1f2937;
            }

            .sql-editor-textarea::placeholder {
                color: #9ca3af;
            }

            /* Syntax highlighting colors */
            .sql-keyword {
                color: #2563eb;
                font-weight: 600;
            }

            .sql-function {
                color: #7c3aed;
            }

            .sql-string {
                color: #059669;
            }

            .sql-number {
                color: #d97706;
            }

            .sql-comment {
                color: #6b7280;
                font-style: italic;
            }

            .sql-operator {
                color: #dc2626;
            }

            .sql-identifier {
                color: #1f2937;
            }

            .sql-table-ref {
                color: #0891b2;
                font-weight: 500;
            }

            .sql-placeholder {
                color: #ea580c;
                font-weight: 600;
                background: #fef3c7;
                padding: 0 2px;
                border-radius: 2px;
            }

            .sql-punctuation {
                color: #6b7280;
            }
            "#
        </style>
    }
}

/// Table selector component for choosing default table and showing available tables
#[component]
pub fn TableSelector(
    /// All available data lakes with their schemas
    #[prop(into)]
    data_lakes: Signal<Vec<crate::types::DataLake>>,
    /// Selected default table (data_lake.schema)
    selected: RwSignal<String>,
    /// Current data lake name (for filtering/highlighting)
    #[prop(optional)]
    current_data_lake: Option<String>,
) -> impl IntoView {
    // Build flat list of table references
    let table_refs = Memo::new(move |_| {
        let mut refs = Vec::new();
        for lake in data_lakes.get() {
            for schema in &lake.schemas {
                refs.push(TableRef {
                    data_lake: lake.name.clone(),
                    schema: schema.schema_name.clone(),
                    display: format!("{}.{}", lake.name, schema.schema_name),
                });
            }
        }
        refs
    });

    view! {
        <div class="space-y-2">
            <label class="block text-sm font-medium text-gray-700">
                "Default Table "
                <span class="text-gray-400 font-normal">"(replaces $table)"</span>
            </label>
            <select
                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-cyan-500 focus:border-cyan-500"
                on:change=move |ev| {
                    selected.set(event_target_value(&ev));
                }
                prop:value=move || selected.get()
            >
                <option value="">"Select a table..."</option>
                {move || {
                    let current_dl = current_data_lake.clone();
                    table_refs.get().into_iter().map(|table| {
                        let is_current = current_dl.as_ref().map(|dl| dl == &table.data_lake).unwrap_or(false);
                        let full_name = table.full_name();
                        let display_name = if is_current {
                            format!("{} (current)", &full_name)
                        } else {
                            full_name.clone()
                        };
                        view! {
                            <option
                                value=full_name
                                class=move || if is_current { "font-semibold" } else { "" }
                            >
                                {display_name}
                            </option>
                        }
                    }).collect::<Vec<_>>()
                }}
            </select>

            // Show available tables for reference
            <details class="text-sm">
                <summary class="cursor-pointer text-gray-500 hover:text-gray-700">
                    "Available tables for JOINs"
                </summary>
                <div class="mt-2 p-3 bg-gray-50 rounded-lg max-h-40 overflow-y-auto">
                    <div class="grid grid-cols-2 gap-1 text-xs font-mono">
                        {move || {
                            table_refs.get().into_iter().map(|table| {
                                let full_name = table.full_name();
                                view! {
                                    <div
                                        class="px-2 py-1 bg-white rounded border border-gray-200 text-cyan-700 cursor-pointer hover:bg-cyan-50"
                                        title="Click to copy"
                                        on:click=move |_| {
                                            // Copy to clipboard using JS
                                            let _ = js_sys::eval(&format!(
                                                "navigator.clipboard.writeText('{}')",
                                                full_name.replace('\'', "\\'")
                                            ));
                                        }
                                    >
                                        {table.full_name()}
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                </div>
            </details>
        </div>
    }
}
