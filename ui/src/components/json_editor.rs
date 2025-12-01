//! Advanced JSON Editor Component
//!
//! A JSON editor with:
//! - Syntax highlighting
//! - Line numbers
//! - Real-time validation
//! - Auto-formatting

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;

/// Tokenize JSON for syntax highlighting
fn tokenize_json(json: &str) -> Vec<(String, &'static str)> {
    let mut tokens = Vec::new();
    let mut chars = json.chars().peekable();

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
                tokens.push((ws, ""));
            }
            // Strings
            '"' => {
                let mut s = String::new();
                s.push(chars.next().unwrap()); // opening quote
                let mut escaped = false;
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        break;
                    }
                }
                // Check if this looks like a key (followed by colon)
                let remaining: String = chars.clone().take_while(|&c| c == ' ' || c == '\t').collect();
                let next_meaningful = chars.clone().skip(remaining.len()).next();
                if next_meaningful == Some(':') {
                    tokens.push((s, "json-key"));
                } else {
                    tokens.push((s, "json-string"));
                }
            }
            // Numbers
            '0'..='9' | '-' => {
                let mut num = String::new();
                if ch == '-' {
                    num.push(chars.next().unwrap());
                }
                // Integer part
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                // Decimal part
                if chars.peek() == Some(&'.') {
                    num.push(chars.next().unwrap());
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                // Exponent
                if chars.peek() == Some(&'e') || chars.peek() == Some(&'E') {
                    num.push(chars.next().unwrap());
                    if chars.peek() == Some(&'+') || chars.peek() == Some(&'-') {
                        num.push(chars.next().unwrap());
                    }
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                tokens.push((num, "json-number"));
            }
            // Booleans and null
            't' | 'f' | 'n' => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphabetic() {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if word == "true" || word == "false" {
                    tokens.push((word, "json-boolean"));
                } else if word == "null" {
                    tokens.push((word, "json-null"));
                } else {
                    tokens.push((word, "json-error"));
                }
            }
            // Structural characters
            '{' | '}' => {
                tokens.push((chars.next().unwrap().to_string(), "json-brace"));
            }
            '[' | ']' => {
                tokens.push((chars.next().unwrap().to_string(), "json-bracket"));
            }
            ':' => {
                tokens.push((chars.next().unwrap().to_string(), "json-colon"));
            }
            ',' => {
                tokens.push((chars.next().unwrap().to_string(), "json-comma"));
            }
            // Unknown characters
            _ => {
                tokens.push((chars.next().unwrap().to_string(), "json-error"));
            }
        }
    }

    tokens
}

/// Generate syntax-highlighted HTML from JSON
fn highlight_json(json: &str) -> String {
    let tokens = tokenize_json(json);
    let mut html = String::new();

    for (text, class) in tokens {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        if class.is_empty() {
            html.push_str(&escaped);
        } else {
            html.push_str(&format!(r#"<span class="{}">{}</span>"#, class, escaped));
        }
    }

    html
}

/// Count lines in text
fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1)
    }
}

/// Format JSON with proper indentation
pub fn format_json(json: &str) -> Result<String, String> {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// Validate JSON and return error if invalid
pub fn validate_json(json: &str) -> Option<String> {
    if json.trim().is_empty() {
        return None;
    }
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(_) => None,
        Err(e) => Some(e.to_string()),
    }
}

/// JSON Editor Component with syntax highlighting and line numbers
#[component]
pub fn JsonEditor(
    /// Current value as a signal
    value: RwSignal<String>,
    /// Placeholder text
    #[prop(default = r#"{"key": "value"}"#.to_string())]
    placeholder: String,
    /// Number of visible rows
    #[prop(default = 10)]
    rows: u32,
    /// Whether the editor is read-only
    #[prop(default = false)]
    readonly: bool,
    /// Label for the editor
    #[prop(default = "JSON".to_string())]
    label: String,
    /// Optional help text
    #[prop(optional)]
    help_text: Option<String>,
    /// Show format button
    #[prop(default = true)]
    show_format_button: bool,
) -> impl IntoView {
    // Validation error signal
    let (validation_error, set_validation_error) = signal(Option::<String>::None);

    // Handle input changes
    let handle_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
        let new_value = textarea.value();

        // Validate JSON
        set_validation_error.set(validate_json(&new_value));

        // Update signal
        value.set(new_value);
    };

    // Handle format button click
    let on_format = move |_| {
        let current = value.get();
        if let Ok(formatted) = format_json(&current) {
            value.set(formatted);
            set_validation_error.set(None);
        }
    };

    // Handle tab key for indentation
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Tab" {
            ev.prevent_default();
            let target = ev.target().unwrap();
            let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();

            let start = textarea.selection_start().unwrap().unwrap_or(0) as usize;
            let end = textarea.selection_end().unwrap().unwrap_or(0) as usize;
            let current = textarea.value();

            // Insert two spaces at cursor position
            let new_value = format!(
                "{}  {}",
                &current[..start],
                &current[end..]
            );

            textarea.set_value(&new_value);
            let new_pos = (start + 2) as u32;
            let _ = textarea.set_selection_start(Some(new_pos));
            let _ = textarea.set_selection_end(Some(new_pos));

            // Trigger change
            value.set(new_value);
        }
    };

    // Sync scroll between textarea and highlighted overlay
    let handle_scroll = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();

        // Find the highlight overlay sibling and sync scroll
        if let Some(parent) = textarea.parent_element() {
            if let Some(highlight) = parent.query_selector(".json-highlight").ok().flatten() {
                let highlight_el: web_sys::HtmlElement = highlight.dyn_into().unwrap();
                highlight_el.set_scroll_top(textarea.scroll_top());
                highlight_el.set_scroll_left(textarea.scroll_left());
            }
        }
    };

    view! {
        <div class="json-editor-container">
            // Header with label and format button
            <div class="flex justify-between items-center mb-1">
                <label class="block text-sm font-medium text-gray-700">{label}</label>
                {show_format_button.then(|| view! {
                    <button
                        type="button"
                        class="text-xs px-2 py-1 bg-gray-100 hover:bg-gray-200 text-gray-600 rounded transition-colors"
                        on:click=on_format
                        disabled=move || readonly || validation_error.get().is_some()
                    >
                        "Format"
                    </button>
                })}
            </div>

            // Editor container with line numbers
            <div class="json-editor-wrapper">
                // Line numbers
                <div class="json-line-numbers" aria-hidden="true">
                    {move || {
                        let lines = count_lines(&value.get());
                        (1..=lines.max(rows as usize))
                            .map(|n| view! { <div class="json-line-number">{n}</div> })
                            .collect_view()
                    }}
                </div>

                // Editor area
                <div class="json-editor-area">
                    // Syntax highlighted overlay (visual only)
                    <pre
                        class="json-highlight"
                        aria-hidden="true"
                        inner_html=move || highlight_json(&value.get())
                    />

                    // Actual textarea for input
                    <textarea
                        class="json-textarea"
                        rows=rows
                        placeholder=placeholder
                        readonly=readonly
                        spellcheck="false"
                        autocomplete="off"
                        prop:value=move || value.get()
                        on:input=handle_input
                        on:keydown=handle_keydown
                        on:scroll=handle_scroll
                    />
                </div>
            </div>

            // Validation error or help text
            {move || {
                if let Some(error) = validation_error.get() {
                    view! {
                        <p class="mt-1 text-xs text-red-500 flex items-center gap-1">
                            <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd"/>
                            </svg>
                            {error}
                        </p>
                    }.into_any()
                } else if let Some(ref help) = help_text {
                    view! {
                        <p class="mt-1 text-xs text-gray-500">{help.clone()}</p>
                    }.into_any()
                } else {
                    view! { <span /> }.into_any()
                }
            }}
        </div>
    }
}
