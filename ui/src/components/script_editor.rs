//! Script Editor Component
//!
//! A multi-language script editor with:
//! - Syntax highlighting for Rhai, Lua, JavaScript, and Python
//! - Language-specific keyword highlighting
//! - Line numbers
//! - Auto-indentation support

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;

/// Supported script languages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScriptLanguage {
    #[default]
    Rhai,
    Lua,
    JavaScript,
    Python,
}

impl ScriptLanguage {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lua" => Self::Lua,
            "js" | "javascript" => Self::JavaScript,
            "python" | "py" => Self::Python,
            _ => Self::Rhai,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Rhai => "Rhai",
            Self::Lua => "Lua",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
        }
    }

    #[allow(dead_code)]
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Rhai => "rhai",
            Self::Lua => "lua",
            Self::JavaScript => "js",
            Self::Python => "py",
        }
    }

    #[allow(dead_code)]
    pub fn comment_prefix(&self) -> &'static str {
        match self {
            Self::Rhai | Self::JavaScript => "//",
            Self::Lua => "--",
            Self::Python => "#",
        }
    }
}

// Language-specific keywords
const RHAI_KEYWORDS: &[&str] = &[
    "let", "const", "if", "else", "while", "loop", "for", "in", "break", "continue",
    "return", "throw", "try", "catch", "fn", "private", "this", "true", "false",
    "null", "switch", "do", "until", "import", "export", "as", "type_of", "print",
    "debug", "is_def_var", "is_def_fn", "eval",
];

const RHAI_BUILTINS: &[&str] = &[
    "to_string", "to_int", "to_float", "to_bool", "type_of", "len", "is_empty",
    "contains", "index_of", "sub_string", "split", "trim", "to_upper", "to_lower",
    "push", "pop", "shift", "remove", "insert", "clear", "reverse", "sort",
    "keys", "values", "get", "set", "merge", "mixin", "fill_with",
    "map", "filter", "reduce", "for_each", "some", "all", "find", "find_map",
    "parse_json", "to_json", "timestamp", "elapsed",
];

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
    "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then",
    "true", "until", "while",
];

const LUA_BUILTINS: &[&str] = &[
    "assert", "collectgarbage", "dofile", "error", "getmetatable", "ipairs",
    "load", "loadfile", "next", "pairs", "pcall", "print", "rawequal", "rawget",
    "rawlen", "rawset", "require", "select", "setmetatable", "tonumber",
    "tostring", "type", "xpcall", "_G", "_VERSION",
    "string", "table", "math", "io", "os", "coroutine", "package", "debug",
];

const JS_KEYWORDS: &[&str] = &[
    "async", "await", "break", "case", "catch", "class", "const", "continue",
    "debugger", "default", "delete", "do", "else", "export", "extends", "false",
    "finally", "for", "function", "if", "import", "in", "instanceof", "let",
    "new", "null", "return", "static", "super", "switch", "this", "throw",
    "true", "try", "typeof", "undefined", "var", "void", "while", "with", "yield",
];

const JS_BUILTINS: &[&str] = &[
    "Array", "Boolean", "Date", "Error", "Function", "JSON", "Map", "Math",
    "Number", "Object", "Promise", "Proxy", "Reflect", "RegExp", "Set", "String",
    "Symbol", "WeakMap", "WeakSet", "console", "parseInt", "parseFloat",
    "isNaN", "isFinite", "encodeURI", "decodeURI", "encodeURIComponent",
    "decodeURIComponent", "eval", "Infinity", "NaN",
];

const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break",
    "class", "continue", "def", "del", "elif", "else", "except", "finally",
    "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

const PYTHON_BUILTINS: &[&str] = &[
    "abs", "all", "any", "ascii", "bin", "bool", "breakpoint", "bytearray",
    "bytes", "callable", "chr", "classmethod", "compile", "complex", "delattr",
    "dict", "dir", "divmod", "enumerate", "eval", "exec", "filter", "float",
    "format", "frozenset", "getattr", "globals", "hasattr", "hash", "help",
    "hex", "id", "input", "int", "isinstance", "issubclass", "iter", "len",
    "list", "locals", "map", "max", "memoryview", "min", "next", "object",
    "oct", "open", "ord", "pow", "print", "property", "range", "repr",
    "reversed", "round", "set", "setattr", "slice", "sorted", "staticmethod",
    "str", "sum", "super", "tuple", "type", "vars", "zip",
    // Metis-specific cross-call functions
    "call_tool", "call_agent", "get_resource", "call_workflow", "datafusion_query",
    "input", "output",
];

/// Token types for syntax highlighting
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Keyword(String),
    Builtin(String),
    String(String),
    Number(String),
    Comment(String),
    Operator(String),
    Identifier(String),
    Punctuation(String),
    Whitespace(String),
}

impl Token {
    fn css_class(&self) -> &'static str {
        match self {
            Token::Keyword(_) => "script-keyword",
            Token::Builtin(_) => "script-builtin",
            Token::String(_) => "script-string",
            Token::Number(_) => "script-number",
            Token::Comment(_) => "script-comment",
            Token::Operator(_) => "script-operator",
            Token::Identifier(_) => "script-identifier",
            Token::Punctuation(_) => "script-punctuation",
            Token::Whitespace(_) => "",
        }
    }

    fn text(&self) -> &str {
        match self {
            Token::Keyword(s) => s,
            Token::Builtin(s) => s,
            Token::String(s) => s,
            Token::Number(s) => s,
            Token::Comment(s) => s,
            Token::Operator(s) => s,
            Token::Identifier(s) => s,
            Token::Punctuation(s) => s,
            Token::Whitespace(s) => s,
        }
    }
}

/// Tokenize code for syntax highlighting
fn tokenize(code: &str, lang: ScriptLanguage) -> Vec<Token> {
    let (keywords, builtins, _comment_single, _comment_multi_start, _comment_multi_end) = match lang {
        ScriptLanguage::Rhai => (RHAI_KEYWORDS, RHAI_BUILTINS, "//", Some("/*"), Some("*/")),
        ScriptLanguage::Lua => (LUA_KEYWORDS, LUA_BUILTINS, "--", Some("--[["), Some("]]")),
        ScriptLanguage::JavaScript => (JS_KEYWORDS, JS_BUILTINS, "//", Some("/*"), Some("*/")),
        ScriptLanguage::Python => (PYTHON_KEYWORDS, PYTHON_BUILTINS, "#", None, None),
    };

    let mut tokens = Vec::new();
    let mut chars = code.chars().peekable();

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
                tokens.push(Token::Whitespace(ws));
            }

            // String literals
            '"' | '\'' => {
                let quote = ch;
                let mut s = String::new();
                s.push(chars.next().unwrap());
                let mut escaped = false;

                // Check for triple quotes (Python)
                if lang == ScriptLanguage::Python && chars.peek() == Some(&quote) {
                    s.push(chars.next().unwrap());
                    if chars.peek() == Some(&quote) {
                        s.push(chars.next().unwrap());
                        // Triple quoted string
                        while let Some(c) = chars.next() {
                            s.push(c);
                            if c == quote {
                                if chars.peek() == Some(&quote) {
                                    s.push(chars.next().unwrap());
                                    if chars.peek() == Some(&quote) {
                                        s.push(chars.next().unwrap());
                                        break;
                                    }
                                }
                            }
                        }
                        tokens.push(Token::String(s));
                        continue;
                    }
                }

                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == quote {
                        break;
                    }
                }
                tokens.push(Token::String(s));
            }

            // Backtick strings (JavaScript template literals)
            '`' if lang == ScriptLanguage::JavaScript => {
                let mut s = String::new();
                s.push(chars.next().unwrap());
                let mut escaped = false;
                while let Some(&c) = chars.peek() {
                    s.push(chars.next().unwrap());
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '`' {
                        break;
                    }
                }
                tokens.push(Token::String(s));
            }

            // Multi-line comments and single-line comments
            '/' if lang == ScriptLanguage::Rhai || lang == ScriptLanguage::JavaScript => {
                if chars.clone().nth(1) == Some('/') {
                    // Single-line comment
                    let mut comment = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        comment.push(chars.next().unwrap());
                    }
                    tokens.push(Token::Comment(comment));
                } else if chars.clone().nth(1) == Some('*') {
                    // Multi-line comment
                    let mut comment = String::new();
                    comment.push(chars.next().unwrap()); // /
                    comment.push(chars.next().unwrap()); // *
                    while let Some(c) = chars.next() {
                        comment.push(c);
                        if c == '*' && chars.peek() == Some(&'/') {
                            comment.push(chars.next().unwrap());
                            break;
                        }
                    }
                    tokens.push(Token::Comment(comment));
                } else {
                    tokens.push(Token::Operator(chars.next().unwrap().to_string()));
                }
            }

            // Lua comments
            '-' if lang == ScriptLanguage::Lua && chars.clone().nth(1) == Some('-') => {
                let mut comment = String::new();
                comment.push(chars.next().unwrap());
                comment.push(chars.next().unwrap());
                // Check for multi-line --[[
                if chars.peek() == Some(&'[') {
                    let peek2: String = chars.clone().take(2).collect();
                    if peek2 == "[[" {
                        comment.push(chars.next().unwrap());
                        comment.push(chars.next().unwrap());
                        // Read until ]]
                        while let Some(c) = chars.next() {
                            comment.push(c);
                            if c == ']' && chars.peek() == Some(&']') {
                                comment.push(chars.next().unwrap());
                                break;
                            }
                        }
                        tokens.push(Token::Comment(comment));
                        continue;
                    }
                }
                // Single-line comment
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    comment.push(chars.next().unwrap());
                }
                tokens.push(Token::Comment(comment));
            }

            // Python comments
            '#' if lang == ScriptLanguage::Python => {
                let mut comment = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    comment.push(chars.next().unwrap());
                }
                tokens.push(Token::Comment(comment));
            }

            // Numbers
            '0'..='9' => {
                let mut num = String::new();
                // Check for hex/octal/binary
                if ch == '0' {
                    num.push(chars.next().unwrap());
                    if let Some(&next) = chars.peek() {
                        if next == 'x' || next == 'X' || next == 'o' || next == 'O' || next == 'b' || next == 'B' {
                            num.push(chars.next().unwrap());
                        }
                    }
                }
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_hexdigit() || c == '.' || c == '_' || c == 'e' || c == 'E' || c == '+' || c == '-' {
                        // Handle scientific notation carefully
                        if (c == '+' || c == '-') && !num.ends_with('e') && !num.ends_with('E') {
                            break;
                        }
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num));
            }

            // Operators
            '+' | '*' | '%' | '&' | '|' | '^' | '~' | '<' | '>' | '=' | '!' | ':' | '?' => {
                let mut op = String::new();
                op.push(chars.next().unwrap());
                // Handle multi-char operators
                if let Some(&next) = chars.peek() {
                    let combo = format!("{}{}", ch, next);
                    if ["==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+=", "-=", "*=", "/=",
                        "=>", "->", "::", "??", "**", "//", "++", "--", ".."].contains(&combo.as_str()) {
                        op.push(chars.next().unwrap());
                        // Check for ===, !==, >>>, etc.
                        if let Some(&third) = chars.peek() {
                            let triple = format!("{}{}", combo, third);
                            if ["===", "!==", ">>>", "..."].contains(&triple.as_str()) {
                                op.push(chars.next().unwrap());
                            }
                        }
                    }
                }
                tokens.push(Token::Operator(op));
            }

            // Minus (separate for Lua comments)
            '-' => {
                if lang != ScriptLanguage::Lua || chars.clone().nth(1) != Some('-') {
                    let mut op = String::new();
                    op.push(chars.next().unwrap());
                    if let Some(&next) = chars.peek() {
                        if next == '=' || next == '>' || next == '-' {
                            op.push(chars.next().unwrap());
                        }
                    }
                    tokens.push(Token::Operator(op));
                } else {
                    // Will be handled by Lua comment branch
                    continue;
                }
            }

            // Punctuation
            '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | '.' | '@' => {
                tokens.push(Token::Punctuation(chars.next().unwrap().to_string()));
            }

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if keywords.contains(&ident.as_str()) {
                    tokens.push(Token::Keyword(ident));
                } else if builtins.contains(&ident.as_str()) {
                    tokens.push(Token::Builtin(ident));
                } else {
                    tokens.push(Token::Identifier(ident));
                }
            }

            // Any other character
            _ => {
                tokens.push(Token::Punctuation(chars.next().unwrap().to_string()));
            }
        }
    }

    tokens
}

/// Generate HTML with syntax highlighting
fn highlight_code(code: &str, lang: ScriptLanguage) -> String {
    let tokens = tokenize(code, lang);
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

/// Count lines in text
fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1)
    }
}

/// Script Editor component with syntax highlighting
#[component]
pub fn ScriptEditor(
    /// The script value (read/write signal)
    value: RwSignal<String>,
    /// The script language
    #[prop(into)]
    language: Signal<ScriptLanguage>,
    /// Placeholder text
    #[prop(default = "// Write your script here...")]
    placeholder: &'static str,
    /// Minimum number of rows
    #[prop(default = 10)]
    min_rows: usize,
    /// Maximum number of rows (for auto-resize)
    #[prop(default = 30)]
    max_rows: usize,
    /// Whether to show line numbers
    #[prop(default = true)]
    show_line_numbers: bool,
) -> impl IntoView {
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Calculate rows based on content
    let computed_rows = Memo::new(move |_| {
        let content = value.get();
        let lines = count_lines(&content);
        lines.clamp(min_rows, max_rows)
    });

    // Generate line numbers HTML
    let line_numbers_html = Memo::new(move |_| {
        let content = value.get();
        let lines = count_lines(&content);
        (1..=lines.max(min_rows))
            .map(|n| format!("<div class=\"script-line-number\">{}</div>", n))
            .collect::<Vec<_>>()
            .join("")
    });

    // Handle input changes
    let on_input = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea = target.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        value.set(textarea.value());
    };

    // Handle tab key for indentation
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Tab" {
            ev.prevent_default();

            if let Some(textarea) = textarea_ref.get() {
                let start = textarea.selection_start().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let end = textarea.selection_end().unwrap_or(Some(0)).unwrap_or(0) as usize;
                let text = value.get();

                let indent = "    "; // 4 spaces

                if ev.shift_key() {
                    // Unindent
                    let before = &text[..start];
                    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let line = &text[line_start..];

                    if line.starts_with(indent) {
                        let new_text = format!(
                            "{}{}",
                            &text[..line_start],
                            &text[line_start + indent.len()..]
                        );
                        value.set(new_text);

                        let new_pos = (start.saturating_sub(indent.len())) as u32;
                        let _ = textarea.set_selection_start(Some(new_pos));
                        let _ = textarea.set_selection_end(Some(new_pos));
                    }
                } else {
                    // Indent
                    let new_text = format!(
                        "{}{}{}",
                        &text[..start],
                        indent,
                        &text[end..]
                    );
                    value.set(new_text);

                    let new_pos = (start + indent.len()) as u32;
                    let _ = textarea.set_selection_start(Some(new_pos));
                    let _ = textarea.set_selection_end(Some(new_pos));
                }
            }
        }
    };

    view! {
        <div class="script-editor-container">
            <div class="script-editor-wrapper">
                // Line numbers
                <Show when=move || show_line_numbers>
                    <div
                        class="script-line-numbers"
                        inner_html=move || line_numbers_html.get()
                    />
                </Show>

                // Code area
                <div class="script-code-area">
                    // Syntax highlighted display
                    <pre
                        class="script-editor-highlight"
                        aria-hidden="true"
                        inner_html=move || {
                            let text = value.get();
                            if text.is_empty() {
                                String::new()
                            } else {
                                highlight_code(&text, language.get())
                            }
                        }
                    />

                    // Actual textarea
                    <textarea
                        node_ref=textarea_ref
                        class="script-editor-textarea"
                        rows=move || computed_rows.get()
                        placeholder=placeholder
                        prop:value=move || value.get()
                        on:input=on_input
                        on:keydown=on_keydown
                        spellcheck="false"
                        autocomplete="off"
                        autocapitalize="off"
                    />
                </div>
            </div>

            // Language indicator and help
            <div class="script-editor-footer">
                <div class="script-language-badge">
                    {move || language.get().display_name()}
                </div>
                <div class="script-editor-hints">
                    <span class="script-hint">"Tab: indent"</span>
                    <span class="script-hint">"Shift+Tab: unindent"</span>
                    {move || {
                        match language.get() {
                            ScriptLanguage::Python => view! {
                                <span class="script-hint">"Access: input, output, call_tool(), call_agent()"</span>
                            }.into_any(),
                            ScriptLanguage::Rhai => view! {
                                <span class="script-hint">"Return: last expression or to_json(result)"</span>
                            }.into_any(),
                            ScriptLanguage::Lua => view! {
                                <span class="script-hint">"Return: return value"</span>
                            }.into_any(),
                            ScriptLanguage::JavaScript => view! {
                                <span class="script-hint">"Return: last expression"</span>
                            }.into_any(),
                        }
                    }}
                </div>
            </div>
        </div>

        // CSS styles
        <style>
            r#"
            .script-editor-container {
                border: 1px solid #d1d5db;
                border-radius: 0.5rem;
                overflow: hidden;
                background: #1e1e1e;
            }

            .script-editor-wrapper {
                display: flex;
                position: relative;
                min-height: 200px;
            }

            .script-line-numbers {
                flex-shrink: 0;
                padding: 0.75rem 0.5rem;
                background: #252526;
                color: #858585;
                font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
                font-size: 0.875rem;
                line-height: 1.5rem;
                text-align: right;
                user-select: none;
                border-right: 1px solid #3c3c3c;
            }

            .script-line-number {
                min-width: 2rem;
                padding-right: 0.5rem;
            }

            .script-code-area {
                flex: 1;
                position: relative;
                overflow: auto;
            }

            .script-editor-highlight {
                position: absolute;
                top: 0;
                left: 0;
                right: 0;
                margin: 0;
                padding: 0.75rem;
                font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
                font-size: 0.875rem;
                line-height: 1.5rem;
                white-space: pre-wrap;
                word-wrap: break-word;
                pointer-events: none;
                color: #d4d4d4;
                background: transparent;
                overflow: hidden;
            }

            .script-editor-textarea {
                width: 100%;
                min-height: 100%;
                margin: 0;
                padding: 0.75rem;
                font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
                font-size: 0.875rem;
                line-height: 1.5rem;
                color: transparent;
                caret-color: #fff;
                background: transparent;
                border: none;
                outline: none;
                resize: none;
                overflow: hidden;
            }

            .script-editor-textarea::placeholder {
                color: #6b7280;
            }

            .script-editor-footer {
                display: flex;
                justify-content: space-between;
                align-items: center;
                padding: 0.5rem 0.75rem;
                background: #252526;
                border-top: 1px solid #3c3c3c;
                font-size: 0.75rem;
            }

            .script-language-badge {
                padding: 0.125rem 0.5rem;
                background: #0e639c;
                color: #fff;
                border-radius: 0.25rem;
                font-weight: 500;
            }

            .script-editor-hints {
                display: flex;
                gap: 1rem;
                color: #858585;
            }

            .script-hint {
                white-space: nowrap;
            }

            /* Syntax highlighting colors (VS Code Dark+ theme inspired) */
            .script-keyword {
                color: #569cd6;
                font-weight: 500;
            }

            .script-builtin {
                color: #dcdcaa;
            }

            .script-string {
                color: #ce9178;
            }

            .script-number {
                color: #b5cea8;
            }

            .script-comment {
                color: #6a9955;
                font-style: italic;
            }

            .script-operator {
                color: #d4d4d4;
            }

            .script-identifier {
                color: #9cdcfe;
            }

            .script-punctuation {
                color: #d4d4d4;
            }
            "#
        </style>
    }
}

/// Compact language selector for the script editor
#[component]
pub fn ScriptLanguageSelector(
    /// The selected language
    selected: RwSignal<String>,
) -> impl IntoView {
    view! {
        <select
            class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 bg-white"
            prop:value=move || selected.get()
            on:change=move |ev| {
                let target = ev.target().unwrap();
                let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                selected.set(select.value());
            }
        >
            <option value="rhai">"Rhai (Rust-like)"</option>
            <option value="lua">"Lua"</option>
            <option value="js">"JavaScript"</option>
            <option value="python">"Python"</option>
        </select>
    }
}
