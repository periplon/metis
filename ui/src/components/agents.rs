//! AI Agents management component
//!
//! Provides CRUD operations for AI agents with support for different
//! agent types (SingleTurn, MultiTurn, ReAct) and LLM providers.

use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use crate::api;
use crate::api::LlmModelInfo;
use crate::types::{Agent, AgentType, AgentLlmConfig, AgentLlmProvider, MemoryConfig, MemoryBackend, MemoryStrategy};
use crate::components::schema_editor::{
    JsonSchemaEditor, SchemaPreview, SchemaProperty,
    properties_to_schema, schema_to_properties,
};
use crate::components::artifact_selector::{
    ArtifactSelector, ArtifactItem, SelectionMode,
};

/// Convert provider enum to API string
fn provider_to_string(provider: &AgentLlmProvider) -> &'static str {
    match provider {
        AgentLlmProvider::OpenAI => "openai",
        AgentLlmProvider::Anthropic => "anthropic",
        AgentLlmProvider::Gemini => "gemini",
        AgentLlmProvider::Ollama => "ollama",
        AgentLlmProvider::AzureOpenAI => "azureopenai",
    }
}

/// Get default model for a provider (used as fallback)
fn get_default_model(provider: &AgentLlmProvider) -> &'static str {
    match provider {
        AgentLlmProvider::OpenAI => "gpt-4o",
        AgentLlmProvider::Anthropic => "claude-sonnet-4-20250514",
        AgentLlmProvider::Gemini => "gemini-2.0-flash",
        AgentLlmProvider::Ollama => "llama3.2:3b",
        AgentLlmProvider::AzureOpenAI => "gpt-4o",
    }
}

/// Get default API key environment variable for a provider
fn get_default_api_key_env(provider: &AgentLlmProvider) -> &'static str {
    match provider {
        AgentLlmProvider::OpenAI => "OPENAI_API_KEY",
        AgentLlmProvider::Anthropic => "ANTHROPIC_API_KEY",
        AgentLlmProvider::Gemini => "GEMINI_API_KEY",
        AgentLlmProvider::Ollama => "",
        AgentLlmProvider::AzureOpenAI => "AZURE_OPENAI_API_KEY",
    }
}

/// Get default base URL for a provider (if applicable)
fn get_default_base_url(provider: &AgentLlmProvider) -> &'static str {
    match provider {
        AgentLlmProvider::Ollama => "http://localhost:11434",
        _ => "",
    }
}

/// Simple markdown to HTML renderer for chat display
/// Handles: headers, bold, italic, code blocks, inline code, links, lists, tables
fn render_markdown(text: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut in_list = false;
    let mut list_type = "ul"; // "ul" or "ol"
    let mut in_blockquote = false;
    let mut blockquote_content = String::new();
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_alignments: Vec<&str> = Vec::new();
    let mut has_header = false;

    // Wrap in a container with proper typography
    html.push_str("<div class=\"markdown-content space-y-2\">");

    for line in text.lines() {
        // Handle code blocks
        if line.starts_with("```") {
            // Close any open list before code block
            if in_list {
                html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
                in_list = false;
            }
            // Close any open blockquote
            if in_blockquote {
                html.push_str(&format!("<blockquote class=\"border-l-4 border-blue-400 dark:border-blue-500 pl-4 py-1 my-2 text-gray-600 dark:text-gray-300 italic bg-blue-50/50 dark:bg-blue-900/20 rounded-r\">{}</blockquote>", process_inline_markdown(&blockquote_content)));
                blockquote_content.clear();
                in_blockquote = false;
            }

            if in_code_block {
                // End code block with language badge
                let lang_badge = if !code_block_lang.is_empty() {
                    format!("<div class=\"flex items-center justify-between px-3 py-1.5 bg-gray-200 dark:bg-gray-700 rounded-t-lg border-b border-gray-300 dark:border-gray-600\"><span class=\"text-xs font-medium text-gray-600 dark:text-gray-300 uppercase tracking-wide\">{}</span><button onclick=\"navigator.clipboard.writeText(this.closest('.code-block').querySelector('code').textContent)\" class=\"text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 flex items-center gap-1\"><svg class=\"w-3.5 h-3.5\" fill=\"none\" stroke=\"currentColor\" viewBox=\"0 0 24 24\"><path stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"2\" d=\"M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z\"/></svg></button></div>", html_escape(&code_block_lang))
                } else {
                    String::new()
                };
                let rounded_class = if code_block_lang.is_empty() { "rounded-lg" } else { "rounded-b-lg" };
                html.push_str(&format!(
                    "<div class=\"code-block my-3 shadow-sm\">{}<pre class=\"bg-gray-100 dark:bg-gray-800 p-3 {} text-xs font-mono overflow-x-auto leading-relaxed\"><code class=\"text-gray-800 dark:text-gray-200\">{}</code></pre></div>",
                    lang_badge,
                    rounded_class,
                    html_escape(&code_block_content)
                ));
                code_block_content.clear();
                code_block_lang.clear();
                in_code_block = false;
            } else {
                // Start code block - extract language
                code_block_lang = line[3..].trim().to_string();
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            if !code_block_content.is_empty() {
                code_block_content.push('\n');
            }
            code_block_content.push_str(line);
            continue;
        }

        let trimmed = line.trim();

        // Handle tables
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            // Close any open list before table
            if in_list {
                html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
                in_list = false;
            }
            // Close any open blockquote
            if in_blockquote {
                html.push_str(&format!("<blockquote class=\"border-l-4 border-blue-400 dark:border-blue-500 pl-4 py-1 my-2 text-gray-600 dark:text-gray-300 italic bg-blue-50/50 dark:bg-blue-900/20 rounded-r\">{}</blockquote>", process_inline_markdown(&blockquote_content)));
                blockquote_content.clear();
                in_blockquote = false;
            }

            // Parse the table row
            let cells: Vec<String> = trimmed[1..trimmed.len()-1]
                .split('|')
                .map(|s| s.trim().to_string())
                .collect();

            // Check if this is a separator row (|---|---|)
            let is_separator = cells.iter().all(|cell| {
                let c = cell.trim();
                c.chars().all(|ch| ch == '-' || ch == ':') && c.contains('-')
            });

            if is_separator {
                // Parse alignments from separator
                table_alignments = cells.iter().map(|cell| {
                    let c = cell.trim();
                    if c.starts_with(':') && c.ends_with(':') {
                        "center"
                    } else if c.ends_with(':') {
                        "right"
                    } else {
                        "left"
                    }
                }).collect();
                has_header = !table_rows.is_empty();
            } else {
                if !in_table {
                    in_table = true;
                }
                table_rows.push(cells);
            }
            continue;
        } else if in_table {
            // End of table - render it
            html.push_str(&render_table(&table_rows, &table_alignments, has_header));
            table_rows.clear();
            table_alignments.clear();
            has_header = false;
            in_table = false;
        }

        // Handle horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            if in_list {
                html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
                in_list = false;
            }
            if in_blockquote {
                html.push_str(&format!("<blockquote class=\"border-l-4 border-blue-400 dark:border-blue-500 pl-4 py-1 my-2 text-gray-600 dark:text-gray-300 italic bg-blue-50/50 dark:bg-blue-900/20 rounded-r\">{}</blockquote>", process_inline_markdown(&blockquote_content)));
                blockquote_content.clear();
                in_blockquote = false;
            }
            html.push_str("<hr class=\"my-4 border-gray-300 dark:border-gray-600\"/>");
            continue;
        }

        // Handle blockquotes
        if trimmed.starts_with("> ") {
            if in_list {
                html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
                in_list = false;
            }
            if !in_blockquote {
                in_blockquote = true;
            } else {
                blockquote_content.push_str("<br/>");
            }
            blockquote_content.push_str(&trimmed[2..]);
            continue;
        } else if in_blockquote {
            html.push_str(&format!("<blockquote class=\"border-l-4 border-blue-400 dark:border-blue-500 pl-4 py-1 my-2 text-gray-600 dark:text-gray-300 italic bg-blue-50/50 dark:bg-blue-900/20 rounded-r\">{}</blockquote>", process_inline_markdown(&blockquote_content)));
            blockquote_content.clear();
            in_blockquote = false;
        }

        // Handle empty lines
        if trimmed.is_empty() {
            if in_list {
                html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
                in_list = false;
            }
            continue;
        }

        // Process regular line
        let processed = process_markdown_line(line, &mut in_list, &mut list_type, &mut html);
        if !processed.is_empty() {
            html.push_str(&processed);
        }
    }

    // Close any unclosed elements
    if in_code_block && !code_block_content.is_empty() {
        let lang_badge = if !code_block_lang.is_empty() {
            format!("<div class=\"flex items-center justify-between px-3 py-1.5 bg-gray-200 dark:bg-gray-700 rounded-t-lg border-b border-gray-300 dark:border-gray-600\"><span class=\"text-xs font-medium text-gray-600 dark:text-gray-300 uppercase tracking-wide\">{}</span></div>", html_escape(&code_block_lang))
        } else {
            String::new()
        };
        let rounded_class = if code_block_lang.is_empty() { "rounded-lg" } else { "rounded-b-lg" };
        html.push_str(&format!(
            "<div class=\"code-block my-3 shadow-sm\">{}<pre class=\"bg-gray-100 dark:bg-gray-800 p-3 {} text-xs font-mono overflow-x-auto leading-relaxed\"><code class=\"text-gray-800 dark:text-gray-200\">{}</code></pre></div>",
            lang_badge,
            rounded_class,
            html_escape(&code_block_content)
        ));
    }
    if in_list {
        html.push_str(if list_type == "ol" { "</ol>" } else { "</ul>" });
    }
    if in_blockquote {
        html.push_str(&format!("<blockquote class=\"border-l-4 border-blue-400 dark:border-blue-500 pl-4 py-1 my-2 text-gray-600 dark:text-gray-300 italic bg-blue-50/50 dark:bg-blue-900/20 rounded-r\">{}</blockquote>", process_inline_markdown(&blockquote_content)));
    }
    if in_table && !table_rows.is_empty() {
        html.push_str(&render_table(&table_rows, &table_alignments, has_header));
    }

    html.push_str("</div>");
    html
}

/// Render a markdown table to HTML
fn render_table(rows: &[Vec<String>], alignments: &[&str], has_header: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let mut html = String::from("<div class=\"my-3 overflow-x-auto\"><table class=\"min-w-full border-collapse text-sm\">");

    for (row_idx, row) in rows.iter().enumerate() {
        let is_header = has_header && row_idx == 0;

        if is_header {
            html.push_str("<thead class=\"bg-gray-100 dark:bg-gray-700\">");
        } else if row_idx == 0 || (has_header && row_idx == 1) {
            html.push_str("<tbody class=\"divide-y divide-gray-200 dark:divide-gray-700\">");
        }

        html.push_str("<tr>");

        for (col_idx, cell) in row.iter().enumerate() {
            let align = alignments.get(col_idx).copied().unwrap_or("left");
            let align_class = match align {
                "center" => "text-center",
                "right" => "text-right",
                _ => "text-left",
            };

            if is_header {
                html.push_str(&format!(
                    "<th class=\"px-3 py-2 font-semibold text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 {}\">{}</th>",
                    align_class,
                    process_inline_markdown(cell)
                ));
            } else {
                html.push_str(&format!(
                    "<td class=\"px-3 py-2 text-gray-600 dark:text-gray-300 border border-gray-300 dark:border-gray-600 {}\">{}</td>",
                    align_class,
                    process_inline_markdown(cell)
                ));
            }
        }

        html.push_str("</tr>");

        if is_header {
            html.push_str("</thead>");
        }
    }

    if !has_header || rows.len() > 1 {
        html.push_str("</tbody>");
    }

    html.push_str("</table></div>");
    html
}

/// Process a single markdown line (headers, bold, italic, inline code, links)
fn process_markdown_line(line: &str, in_list: &mut bool, list_type: &mut &str, html: &mut String) -> String {
    let trimmed = line.trim();

    // Headers - close any open list first
    if trimmed.starts_with('#') {
        if *in_list {
            html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
            *in_list = false;
        }
    }

    // H4
    if trimmed.starts_with("#### ") {
        return format!("<h4 class=\"text-sm font-semibold mt-3 mb-1 text-gray-700 dark:text-gray-300\">{}</h4>", process_inline_markdown(&trimmed[5..]));
    }
    // H3
    if trimmed.starts_with("### ") {
        return format!("<h3 class=\"text-base font-semibold mt-4 mb-2 text-gray-800 dark:text-gray-200 border-b border-gray-200 dark:border-gray-700 pb-1\">{}</h3>", process_inline_markdown(&trimmed[4..]));
    }
    // H2
    if trimmed.starts_with("## ") {
        return format!("<h2 class=\"text-lg font-bold mt-4 mb-2 text-gray-900 dark:text-gray-100\">{}</h2>", process_inline_markdown(&trimmed[3..]));
    }
    // H1
    if trimmed.starts_with("# ") {
        return format!("<h1 class=\"text-xl font-bold mt-4 mb-2 text-gray-900 dark:text-gray-100 border-b-2 border-gray-300 dark:border-gray-600 pb-2\">{}</h1>", process_inline_markdown(&trimmed[2..]));
    }

    // Task list items (checkboxes)
    if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
        if !*in_list || *list_type != "ul" {
            if *in_list {
                html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
            }
            html.push_str("<ul class=\"my-2 space-y-1\">");
            *in_list = true;
            *list_type = "ul";
        }
        return format!("<li class=\"flex items-start gap-2 ml-1\"><span class=\"flex-shrink-0 w-4 h-4 mt-0.5 rounded border border-green-500 bg-green-500 flex items-center justify-center\"><svg class=\"w-3 h-3 text-white\" fill=\"none\" stroke=\"currentColor\" viewBox=\"0 0 24 24\"><path stroke-linecap=\"round\" stroke-linejoin=\"round\" stroke-width=\"3\" d=\"M5 13l4 4L19 7\"/></svg></span><span class=\"text-gray-600 dark:text-gray-400 line-through\">{}</span></li>", process_inline_markdown(&trimmed[6..]));
    }
    if trimmed.starts_with("- [ ] ") {
        if !*in_list || *list_type != "ul" {
            if *in_list {
                html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
            }
            html.push_str("<ul class=\"my-2 space-y-1\">");
            *in_list = true;
            *list_type = "ul";
        }
        return format!("<li class=\"flex items-start gap-2 ml-1\"><span class=\"flex-shrink-0 w-4 h-4 mt-0.5 rounded border-2 border-gray-300 dark:border-gray-500\"></span><span>{}</span></li>", process_inline_markdown(&trimmed[6..]));
    }

    // Unordered list items
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        if !*in_list || *list_type != "ul" {
            if *in_list {
                html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
            }
            html.push_str("<ul class=\"my-2 ml-4 space-y-1\">");
            *in_list = true;
            *list_type = "ul";
        }
        return format!("<li class=\"flex items-start\"><span class=\"text-blue-500 dark:text-blue-400 mr-2 font-bold\">•</span><span>{}</span></li>", process_inline_markdown(&trimmed[2..]));
    }

    // Ordered list items
    if let Some((num, rest)) = parse_ordered_list_item(trimmed) {
        if !*in_list || *list_type != "ol" {
            if *in_list {
                html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
            }
            html.push_str("<ol class=\"my-2 ml-4 space-y-1 list-decimal list-inside\">");
            *in_list = true;
            *list_type = "ol";
        }
        return format!("<li class=\"flex items-start\"><span class=\"text-blue-500 dark:text-blue-400 mr-2 font-semibold min-w-[1.5rem]\">{}</span><span>{}</span></li>", num, process_inline_markdown(rest));
    }

    // Regular paragraph - close list if open
    if *in_list {
        html.push_str(if *list_type == "ol" { "</ol>" } else { "</ul>" });
        *in_list = false;
    }

    format!("<p class=\"leading-relaxed\">{}</p>", process_inline_markdown(trimmed))
}

/// Parse an ordered list item, returning (number, rest) if successful
fn parse_ordered_list_item(line: &str) -> Option<(&str, &str)> {
    let mut i = 0;
    let chars: Vec<char> = line.chars().collect();

    // Find digits
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // Need at least one digit followed by ". "
    if i == 0 || i >= chars.len() - 1 {
        return None;
    }

    if chars[i] == '.' && chars.get(i + 1) == Some(&' ') {
        let num_end = line.char_indices().nth(i).map(|(idx, _)| idx)?;
        let rest_start = line.char_indices().nth(i + 2).map(|(idx, _)| idx)?;
        return Some((&line[..num_end], &line[rest_start..]));
    }

    None
}

/// Process inline markdown (bold, italic, code, links, strikethrough)
fn process_inline_markdown(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code `code` - enhanced styling
        if chars[i] == '`' {
            if let Some(end) = chars[i+1..].iter().position(|&c| c == '`') {
                let code: String = chars[i+1..i+1+end].iter().collect();
                result.push_str(&format!(
                    "<code class=\"px-1.5 py-0.5 text-[0.85em] font-mono bg-gray-100 dark:bg-gray-700 text-pink-600 dark:text-pink-400 rounded border border-gray-200 dark:border-gray-600\">{}</code>",
                    html_escape(&code)
                ));
                i += end + 2;
                continue;
            }
        }

        // Strikethrough ~~text~~
        if i + 1 < chars.len() && chars[i] == '~' && chars[i+1] == '~' {
            if let Some(end) = find_double_char(&chars[i+2..], '~') {
                let strike: String = chars[i+2..i+2+end].iter().collect();
                result.push_str(&format!("<del class=\"text-gray-500 dark:text-gray-400\">{}</del>", html_escape(&strike)));
                i += end + 4;
                continue;
            }
        }

        // Bold **text** - enhanced styling
        if i + 1 < chars.len() && chars[i] == '*' && chars[i+1] == '*' {
            if let Some(end) = find_double_char(&chars[i+2..], '*') {
                let bold: String = chars[i+2..i+2+end].iter().collect();
                result.push_str(&format!("<strong class=\"font-semibold text-gray-900 dark:text-white\">{}</strong>", html_escape(&bold)));
                i += end + 4;
                continue;
            }
        }

        // Bold __text__
        if i + 1 < chars.len() && chars[i] == '_' && chars[i+1] == '_' {
            if let Some(end) = find_double_char(&chars[i+2..], '_') {
                let bold: String = chars[i+2..i+2+end].iter().collect();
                result.push_str(&format!("<strong class=\"font-semibold text-gray-900 dark:text-white\">{}</strong>", html_escape(&bold)));
                i += end + 4;
                continue;
            }
        }

        // Italic *text* (single asterisk, not followed by another)
        if chars[i] == '*' && (i + 1 >= chars.len() || chars[i+1] != '*') {
            if let Some(end) = chars[i+1..].iter().position(|&c| c == '*') {
                if end > 0 {
                    let italic: String = chars[i+1..i+1+end].iter().collect();
                    result.push_str(&format!("<em class=\"italic text-gray-700 dark:text-gray-300\">{}</em>", html_escape(&italic)));
                    i += end + 2;
                    continue;
                }
            }
        }

        // Italic _text_ (single underscore)
        if chars[i] == '_' && (i + 1 >= chars.len() || chars[i+1] != '_') {
            if let Some(end) = chars[i+1..].iter().position(|&c| c == '_') {
                if end > 0 {
                    let italic: String = chars[i+1..i+1+end].iter().collect();
                    result.push_str(&format!("<em class=\"italic text-gray-700 dark:text-gray-300\">{}</em>", html_escape(&italic)));
                    i += end + 2;
                    continue;
                }
            }
        }

        // Link [text](url) - enhanced styling
        if chars[i] == '[' {
            if let Some(text_end) = chars[i+1..].iter().position(|&c| c == ']') {
                let link_text: String = chars[i+1..i+1+text_end].iter().collect();
                let url_start = i + 2 + text_end;
                if url_start < chars.len() && chars[url_start] == '(' {
                    if let Some(url_end) = chars[url_start+1..].iter().position(|&c| c == ')') {
                        let url: String = chars[url_start+1..url_start+1+url_end].iter().collect();
                        result.push_str(&format!(
                            "<a href=\"{}\" class=\"text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 underline decoration-blue-400/50 hover:decoration-blue-600 transition-colors\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                            html_escape(&url),
                            html_escape(&link_text)
                        ));
                        i = url_start + url_end + 2;
                        continue;
                    }
                }
            }
        }

        // Regular character
        result.push_str(&html_escape(&chars[i].to_string()));
        i += 1;
    }

    result
}

/// Find position of double character (e.g., **)
fn find_double_char(chars: &[char], c: char) -> Option<usize> {
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == c && chars[i+1] == c {
            return Some(i);
        }
    }
    None
}

/// Escape HTML special characters
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Check if an error message is related to API key issues
fn is_api_key_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("api_key") ||
    lower.contains("api key") ||
    lower.contains("environment variable") && lower.contains("not set") ||
    lower.contains("authentication") ||
    lower.contains("unauthorized") ||
    lower.contains("invalid key")
}

/// Extract the environment variable name from an API key error message
fn extract_env_var_from_error(error: &str) -> Option<String> {
    // Pattern: "Environment variable XXXX not set" or "XXXX environment variable not set"
    if error.contains("environment variable") {
        // Try to find uppercase words that look like env vars
        for word in error.split_whitespace() {
            if word.chars().all(|c| c.is_ascii_uppercase() || c == '_') && word.len() > 3 {
                return Some(word.to_string());
            }
        }
    }
    None
}

/// Format API key error with helpful guidance
fn format_api_key_error(error: &str, provider: Option<&AgentLlmProvider>) -> (String, String) {
    let env_var = extract_env_var_from_error(error);

    let title = "API Key Not Configured".to_string();

    let mut guidance = String::new();

    if let Some(var) = &env_var {
        guidance.push_str(&format!("The environment variable `{}` is not set on the server.\n\n", var));
    } else {
        guidance.push_str("The required API key environment variable is not set on the server.\n\n");
    }

    guidance.push_str("To fix this:\n");
    guidance.push_str("1. Set the environment variable before starting the server\n");

    if let Some(var) = &env_var {
        guidance.push_str(&format!("   export {}=your-api-key-here\n", var));
    }

    if let Some(prov) = provider {
        let hint = match prov {
            AgentLlmProvider::OpenAI => "Get your API key from https://platform.openai.com/api-keys",
            AgentLlmProvider::Anthropic => "Get your API key from https://console.anthropic.com/settings/keys",
            AgentLlmProvider::Gemini => "Get your API key from https://aistudio.google.com/apikey",
            AgentLlmProvider::AzureOpenAI => "Get your API key from your Azure OpenAI resource in the Azure portal",
            AgentLlmProvider::Ollama => "Ollama typically doesn't require an API key for local deployments",
        };
        guidance.push_str(&format!("2. {}\n", hint));
    }

    guidance.push_str("3. Restart the server after setting the environment variable");

    (title, guidance)
}

/// Check if provider requires API key
fn provider_requires_api_key(provider: &AgentLlmProvider) -> bool {
    !matches!(provider, AgentLlmProvider::Ollama)
}

// ============================================================================
// Searchable Multi-Select Component
// ============================================================================

/// Item that can be selected in SearchableMultiSelect
#[derive(Clone, Debug)]
pub struct SelectableItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
}

/// Color theme for the multi-select component
#[derive(Clone, Copy, PartialEq, Default)]
pub enum SelectTheme {
    #[default]
    Blue,    // Regular tools
    Orange,  // Workflows
    Purple,  // MCP tools
    Indigo,  // Agents
}

impl SelectTheme {
    fn border_class(&self) -> &'static str {
        match self {
            Self::Blue => "border-blue-300 focus:border-blue-500 focus:ring-blue-500",
            Self::Orange => "border-orange-300 focus:border-orange-500 focus:ring-orange-500",
            Self::Purple => "border-purple-300 focus:border-purple-500 focus:ring-purple-500",
            Self::Indigo => "border-indigo-300 focus:border-indigo-500 focus:ring-indigo-500",
        }
    }

    fn chip_class(&self) -> &'static str {
        match self {
            Self::Blue => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
            Self::Orange => "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200",
            Self::Purple => "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
            Self::Indigo => "bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200",
        }
    }

    fn chip_remove_class(&self) -> &'static str {
        match self {
            Self::Blue => "text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-200",
            Self::Orange => "text-orange-600 hover:text-orange-800 dark:text-orange-400 dark:hover:text-orange-200",
            Self::Purple => "text-purple-600 hover:text-purple-800 dark:text-purple-400 dark:hover:text-purple-200",
            Self::Indigo => "text-indigo-600 hover:text-indigo-800 dark:text-indigo-400 dark:hover:text-indigo-200",
        }
    }

    fn dropdown_item_hover(&self) -> &'static str {
        match self {
            Self::Blue => "hover:bg-blue-50 dark:hover:bg-blue-900/30",
            Self::Orange => "hover:bg-orange-50 dark:hover:bg-orange-900/30",
            Self::Purple => "hover:bg-purple-50 dark:hover:bg-purple-900/30",
            Self::Indigo => "hover:bg-indigo-50 dark:hover:bg-indigo-900/30",
        }
    }

    fn accent_text(&self) -> &'static str {
        match self {
            Self::Blue => "text-blue-600 dark:text-blue-400",
            Self::Orange => "text-orange-600 dark:text-orange-400",
            Self::Purple => "text-purple-600 dark:text-purple-400",
            Self::Indigo => "text-indigo-600 dark:text-indigo-400",
        }
    }
}

/// Searchable multi-select component with chips
/// Scales well for thousands of items by only showing filtered results
#[component]
fn SearchableMultiSelect(
    /// All available items to select from
    items: Signal<Vec<SelectableItem>>,
    /// Currently selected item IDs
    selected: Signal<Vec<String>>,
    /// Callback when selection changes
    on_change: Callback<Vec<String>>,
    /// Placeholder text for search input
    #[prop(default = "Search...")]
    placeholder: &'static str,
    /// Color theme
    #[prop(default = SelectTheme::Blue)]
    theme: SelectTheme,
    /// Label for the component
    #[prop(optional)]
    label: Option<&'static str>,
    /// Help text below the component
    #[prop(optional)]
    help_text: Option<&'static str>,
    /// Maximum items to show in dropdown
    #[prop(default = 10)]
    max_results: usize,
) -> impl IntoView {
    let (search_query, set_search_query) = signal(String::new());
    let (show_dropdown, set_show_dropdown) = signal(false);
    let (focused_index, set_focused_index) = signal(Option::<usize>::None);

    // Filter items based on search query
    let filtered_items = move || {
        let query = search_query.get().to_lowercase();
        let all_items = items.get();
        let selected_ids = selected.get();

        if query.is_empty() {
            // Show unselected items when no search
            all_items
                .into_iter()
                .filter(|item| !selected_ids.contains(&item.id))
                .take(max_results)
                .collect::<Vec<_>>()
        } else {
            // Filter by search query
            all_items
                .into_iter()
                .filter(|item| {
                    !selected_ids.contains(&item.id) &&
                    (item.name.to_lowercase().contains(&query) ||
                     item.description.to_lowercase().contains(&query) ||
                     item.category.as_ref().map(|c| c.to_lowercase().contains(&query)).unwrap_or(false))
                })
                .take(max_results)
                .collect::<Vec<_>>()
        }
    };

    // Get selected items for chip display
    let selected_items = move || {
        let all_items = items.get();
        let selected_ids = selected.get();
        all_items
            .into_iter()
            .filter(|item| selected_ids.contains(&item.id))
            .collect::<Vec<_>>()
    };

    // Add item to selection
    let add_item = move |item_id: String| {
        let mut current = selected.get();
        if !current.contains(&item_id) {
            current.push(item_id);
            on_change.run(current);
        }
        set_search_query.set(String::new());
        set_focused_index.set(None);
    };

    // Remove item from selection
    let remove_item = move |item_id: String| {
        let mut current = selected.get();
        current.retain(|id| id != &item_id);
        on_change.run(current);
    };

    // Handle keyboard navigation
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        let filtered = filtered_items();
        let filtered_len = filtered.len();

        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_show_dropdown.set(true);
                set_focused_index.update(|idx| {
                    *idx = Some(match idx {
                        Some(i) if *i < filtered_len.saturating_sub(1) => *i + 1,
                        Some(_) => 0,
                        None => 0,
                    });
                });
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_focused_index.update(|idx| {
                    *idx = Some(match idx {
                        Some(i) if *i > 0 => *i - 1,
                        Some(_) => filtered_len.saturating_sub(1),
                        None => filtered_len.saturating_sub(1),
                    });
                });
            }
            "Enter" => {
                ev.prevent_default();
                if let Some(idx) = focused_index.get() {
                    if let Some(item) = filtered.get(idx) {
                        add_item(item.id.clone());
                    }
                }
            }
            "Escape" => {
                set_show_dropdown.set(false);
                set_focused_index.set(None);
            }
            _ => {}
        }
    };

    let total_count = move || items.get().len();
    let selected_count = move || selected.get().len();

    view! {
        <div class="space-y-2">
            // Label
            {label.map(|l| view! {
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    {l}
                    <span class="ml-2 text-gray-400 font-normal">
                        "("{selected_count}" / "{total_count}" selected)"
                    </span>
                </label>
            })}

            // Selected items as chips
            {move || {
                let items = selected_items();
                if items.is_empty() {
                    view! { <span></span> }.into_any()
                } else {
                    view! {
                        <div class="flex flex-wrap gap-1.5 mb-2">
                            {items.into_iter().map(|item| {
                                let item_id = item.id.clone();
                                let item_id_for_remove = item_id.clone();
                                view! {
                                    <span class=format!("inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium {}", theme.chip_class())>
                                        <span class="truncate max-w-[150px]">{item.name}</span>
                                        {item.category.map(|cat| view! {
                                            <span class="text-[10px] opacity-70">"("{cat}")"</span>
                                        })}
                                        <button
                                            type="button"
                                            class=format!("ml-0.5 -mr-1 {}", theme.chip_remove_class())
                                            on:click=move |_| remove_item(item_id_for_remove.clone())
                                        >
                                            <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                                                <path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"/>
                                            </svg>
                                        </button>
                                    </span>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}

            // Search input with dropdown
            <div class="relative">
                <input
                    type="text"
                    class=format!("w-full px-3 py-2 text-sm border rounded-md shadow-sm dark:bg-gray-700 dark:text-white {}", theme.border_class())
                    placeholder=placeholder
                    prop:value=move || search_query.get()
                    on:input=move |ev| {
                        set_search_query.set(event_target_value(&ev));
                        set_show_dropdown.set(true);
                        set_focused_index.set(None);
                    }
                    on:focus=move |_| set_show_dropdown.set(true)
                    on:blur=move |_| {
                        // Delay hiding to allow click on dropdown items
                        set_timeout(move || set_show_dropdown.set(false), std::time::Duration::from_millis(200));
                    }
                    on:keydown=on_keydown
                />

                // Dropdown with filtered results
                {move || {
                    if !show_dropdown.get() {
                        return view! { <span></span> }.into_any();
                    }

                    let filtered = filtered_items();
                    if filtered.is_empty() {
                        let query = search_query.get();
                        return view! {
                            <div class="absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg">
                                <div class="px-3 py-2 text-sm text-gray-500 dark:text-gray-400">
                                    {if query.is_empty() {
                                        "All items selected or no items available"
                                    } else {
                                        "No matching items found"
                                    }}
                                </div>
                            </div>
                        }.into_any();
                    }

                    let focused_idx = focused_index.get();
                    view! {
                        <div class="absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto">
                            {filtered.into_iter().enumerate().map(|(idx, item)| {
                                let item_id = item.id.clone();
                                let item_id_for_click = item_id.clone();
                                let is_focused = focused_idx == Some(idx);
                                let bg_class = if is_focused {
                                    match theme {
                                        SelectTheme::Blue => "bg-blue-50 dark:bg-blue-900/30",
                                        SelectTheme::Orange => "bg-orange-50 dark:bg-orange-900/30",
                                        SelectTheme::Purple => "bg-purple-50 dark:bg-purple-900/30",
                                        SelectTheme::Indigo => "bg-indigo-50 dark:bg-indigo-900/30",
                                    }
                                } else {
                                    ""
                                };
                                view! {
                                    <div
                                        class=format!("px-3 py-2 cursor-pointer {} {}", theme.dropdown_item_hover(), bg_class)
                                        on:mousedown=move |_| add_item(item_id_for_click.clone())
                                    >
                                        <div class="flex items-center justify-between">
                                            <span class="text-sm font-medium text-gray-900 dark:text-white truncate">{item.name}</span>
                                            {item.category.map(|cat| view! {
                                                <span class=format!("ml-2 text-xs {}", theme.accent_text())>{cat}</span>
                                            })}
                                        </div>
                                        <div class="text-xs text-gray-500 dark:text-gray-400 truncate">{item.description}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                            {move || {
                                let total = items.get().len();
                                let shown = filtered_items().len();
                                let remaining = total.saturating_sub(selected.get().len()).saturating_sub(shown);
                                if remaining > 0 {
                                    view! {
                                        <div class="px-3 py-1.5 text-xs text-gray-400 dark:text-gray-500 border-t border-gray-100 dark:border-gray-700">
                                            {format!("... and {} more (type to search)", remaining)}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }}
            </div>

            // Help text
            {help_text.map(|h| view! {
                <p class="text-xs text-gray-500 dark:text-gray-400">{h}</p>
            })}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
}

/// Chat message for the agent test interface
#[derive(Clone, Debug)]
struct ChatMessage {
    role: ChatRole,
    content: String,
    raw_output: Option<serde_json::Value>,
    execution_time_ms: Option<u64>,
    error: Option<String>,
    timestamp: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ChatRole {
    User,
    Assistant,
    System,
}

impl ChatMessage {
    fn user(content: String) -> Self {
        Self {
            role: ChatRole::User,
            content,
            raw_output: None,
            execution_time_ms: None,
            error: None,
            timestamp: js_sys::Date::now() as u64,
        }
    }

    fn assistant(content: String, raw_output: serde_json::Value, execution_time_ms: u64, error: Option<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content,
            raw_output: Some(raw_output),
            execution_time_ms: Some(execution_time_ms),
            error,
            timestamp: js_sys::Date::now() as u64,
        }
    }

    fn system_error(error: String) -> Self {
        Self {
            role: ChatRole::System,
            content: error.clone(),
            raw_output: None,
            execution_time_ms: None,
            error: Some(error),
            timestamp: js_sys::Date::now() as u64,
        }
    }
}

/// Main Agents list/management component
#[component]
pub fn Agents() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Chat test modal state
    let (test_target, set_test_target) = signal(Option::<Agent>::None);
    let (chat_messages, set_chat_messages) = signal(Vec::<ChatMessage>::new());
    let (chat_input, set_chat_input) = signal(String::new());
    let (sending, set_sending) = signal(false);
    let (chat_view_tab, set_chat_view_tab) = signal("chat".to_string()); // "chat" or "raw"
    let (chat_session_id, set_chat_session_id) = signal(Option::<String>::None); // Session ID for multi-turn
    // Dynamic form values for input schema (key -> value)
    let (form_values, set_form_values) = signal(std::collections::HashMap::<String, String>::new());

    let agents = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_agents().await.ok() }
    });

    let on_delete_confirm = move |_| {
        if let Some(name) = delete_target.get() {
            set_deleting.set(true);
            let name_clone = name.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_agent(&name_clone).await {
                    Ok(_) => {
                        set_delete_target.set(None);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                    Err(e) => {
                        web_sys::window()
                            .and_then(|w| w.alert_with_message(&format!("Failed to delete: {}", e)).ok());
                    }
                }
                set_deleting.set(false);
            });
        }
    };

    // Helper to check if agent has custom input schema (not just prompt)
    let has_custom_schema = move |agent: &Agent| -> bool {
        if let Some(props) = agent.input_schema.get("properties").and_then(|p| p.as_object()) {
            // Has custom schema if there's more than just "prompt" OR no "prompt" at all
            props.len() > 1 || !props.contains_key("prompt")
        } else {
            false
        }
    };

    // Get schema properties for form rendering
    let get_schema_properties = move |agent: &Agent| -> Vec<(String, String, String, bool)> {
        // Returns: (name, type, description, required)
        let mut props = Vec::new();
        if let Some(properties) = agent.input_schema.get("properties").and_then(|p| p.as_object()) {
            let required: Vec<String> = agent.input_schema.get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            for (name, schema) in properties {
                let prop_type = schema.get("type").and_then(|t| t.as_str()).unwrap_or("string").to_string();
                let description = schema.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string();
                let is_required = required.contains(name);
                props.push((name.clone(), prop_type, description, is_required));
            }
        }
        // Sort so required fields come first, then by name
        props.sort_by(|a, b| {
            match (a.3, b.3) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            }
        });
        props
    };

    // Core send message logic
    let do_send_message = move || {
        if sending.get() {
            return;
        }

        if let Some(agent) = test_target.get() {
            // Build args from form values or simple prompt
            let (args, display_text) = if has_custom_schema(&agent) {
                let values = form_values.get();
                // Build JSON from form values
                let mut args_obj = serde_json::Map::new();
                let mut display_parts = Vec::new();
                for (key, value) in values.iter() {
                    if !value.is_empty() {
                        args_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
                        display_parts.push(format!("{}: {}", key, value));
                    }
                }
                if args_obj.is_empty() {
                    return; // No values to send
                }
                let display = display_parts.join("\n");
                (serde_json::Value::Object(args_obj), display)
            } else {
                let input = chat_input.get().trim().to_string();
                if input.is_empty() {
                    return;
                }
                (serde_json::json!({ "prompt": input }), input)
            };

            // Add user message to chat
            let user_msg = ChatMessage::user(display_text);
            set_chat_messages.update(|msgs| msgs.push(user_msg));
            set_chat_input.set(String::new());
            set_form_values.set(std::collections::HashMap::new());
            set_sending.set(true);
            let agent_name = agent.name.clone();
            let session_id = chat_session_id.get();

            wasm_bindgen_futures::spawn_local(async move {
                match api::test_agent(&agent_name, &args, session_id).await {
                    Ok(test_res) => {
                        // Extract and store session_id for multi-turn conversations
                        if let Some(sid) = test_res.output.get("session_id") {
                            if let Some(sid_str) = sid.as_str() {
                                set_chat_session_id.set(Some(sid_str.to_string()));
                            }
                        }

                        // Extract response text from output - handle nested structures
                        let response_text =
                            // Check output.output.content (agent response format)
                            if let Some(output) = test_res.output.get("output") {
                                if let Some(content) = output.get("content") {
                                    content.as_str().map(|s| s.to_string())
                                        .unwrap_or_else(|| serde_json::to_string_pretty(&content).unwrap_or_default())
                                } else if let Some(text) = output.as_str() {
                                    text.to_string()
                                } else {
                                    serde_json::to_string_pretty(&output).unwrap_or_default()
                                }
                            }
                            // Check direct response field
                            else if let Some(resp) = test_res.output.get("response") {
                                resp.as_str().map(|s| s.to_string())
                                    .unwrap_or_else(|| serde_json::to_string_pretty(&resp).unwrap_or_default())
                            }
                            // Check direct content field
                            else if let Some(content) = test_res.output.get("content") {
                                content.as_str().map(|s| s.to_string())
                                    .unwrap_or_else(|| serde_json::to_string_pretty(&content).unwrap_or_default())
                            }
                            // Check if output is a string
                            else if let Some(text) = test_res.output.as_str() {
                                text.to_string()
                            }
                            // Fallback to JSON
                            else {
                                serde_json::to_string_pretty(&test_res.output).unwrap_or_else(|_| "Error formatting output".to_string())
                            };

                        let assistant_msg = ChatMessage::assistant(
                            response_text,
                            test_res.output,
                            test_res.execution_time_ms,
                            test_res.error,
                        );
                        set_chat_messages.update(|msgs| msgs.push(assistant_msg));
                    }
                    Err(e) => {
                        let error_msg = ChatMessage::system_error(e);
                        set_chat_messages.update(|msgs| msgs.push(error_msg));
                    }
                }
                set_sending.set(false);
            });
        }
    };

    // Button click handler
    let on_send_click = move |_: web_sys::MouseEvent| {
        do_send_message();
    };

    // Handle Enter key in chat input
    let on_chat_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            do_send_message();
        }
    };

    // Clear chat when opening new agent test
    let open_chat_test = move |agent: Agent| {
        set_chat_messages.set(Vec::new());
        set_chat_input.set(String::new());
        set_form_values.set(std::collections::HashMap::new());
        set_chat_view_tab.set("chat".to_string());
        set_chat_session_id.set(None); // Reset session for new conversation
        set_test_target.set(Some(agent));
    };

    view! {
        <div class="p-6 space-y-6">
            // Header with actions
            <div class="flex justify-between items-center">
                <div>
                    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">"AI Agents"</h2>
                    <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
                        "Configure and manage AI agents for automated task execution"
                    </p>
                </div>
                <div class="flex items-center space-x-4">
                    // View mode toggle
                    <div class="flex rounded-md shadow-sm" role="group">
                        <button
                            type="button"
                            class=move || format!(
                                "px-3 py-2 text-sm font-medium rounded-l-lg border {}",
                                if view_mode.get() == ViewMode::Table {
                                    "bg-blue-600 text-white border-blue-600"
                                } else {
                                    "bg-white text-gray-700 border-gray-200 hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-300 dark:border-gray-600"
                                }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Table)
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"/>
                            </svg>
                        </button>
                        <button
                            type="button"
                            class=move || format!(
                                "px-3 py-2 text-sm font-medium rounded-r-lg border {}",
                                if view_mode.get() == ViewMode::Card {
                                    "bg-blue-600 text-white border-blue-600"
                                } else {
                                    "bg-white text-gray-700 border-gray-200 hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-300 dark:border-gray-600"
                                }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Card)
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/>
                            </svg>
                        </button>
                    </div>
                    // New Agent button
                    <a
                        href="/agents/new"
                        class="inline-flex items-center px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 dark:focus:ring-blue-800"
                    >
                        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Agent"
                    </a>
                </div>
            </div>

            // Content
            <Suspense fallback=move || view! { <div class="text-center py-8">"Loading agents..."</div> }>
                {move || {
                    let agents_data = agents.get().flatten();
                    match agents_data {
                        Some(agents_list) if !agents_list.is_empty() => {
                            if view_mode.get() == ViewMode::Table {
                                view! {
                                    <div class="overflow-x-auto bg-white dark:bg-gray-800 rounded-lg shadow border border-gray-200 dark:border-gray-700">
                                        <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                            <thead class="bg-gray-50 dark:bg-gray-700">
                                                <tr>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Name"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Type"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"LLM"</th>
                                                    <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Tools"</th>
                                                    <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-300 uppercase tracking-wider">"Actions"</th>
                                                </tr>
                                            </thead>
                                            <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                                {agents_list.into_iter().map(|agent| {
                                                    let agent_for_test = agent.clone();
                                                    let name_for_edit = agent.name.clone();
                                                    let name_for_delete = agent.name.clone();
                                                    view! {
                                                        <tr class="hover:bg-gray-50 dark:hover:bg-gray-700">
                                                            <td class="px-6 py-4 whitespace-nowrap">
                                                                <div class="text-sm font-medium text-gray-900 dark:text-white">{agent.name.clone()}</div>
                                                                <div class="text-sm text-gray-500 dark:text-gray-400 truncate max-w-xs">{agent.description.clone()}</div>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap">
                                                                <span class=format!(
                                                                    "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}",
                                                                    match agent.agent_type {
                                                                        AgentType::SingleTurn => "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300",
                                                                        AgentType::MultiTurn => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300",
                                                                        AgentType::ReAct => "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300",
                                                                    }
                                                                )>
                                                                    {format!("{:?}", agent.agent_type)}
                                                                </span>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                <div>{format!("{:?}", agent.llm.provider)}</div>
                                                                <div class="text-xs text-gray-400">{agent.llm.model.clone()}</div>
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                                <div>{agent.available_tools.len()}" tools"</div>
                                                                {if !agent.mcp_tools.is_empty() {
                                                                    view! { <div class="text-xs text-purple-500">{agent.mcp_tools.len()}" MCP"</div> }.into_any()
                                                                } else {
                                                                    view! { <span></span> }.into_any()
                                                                }}
                                                            </td>
                                                            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-2">
                                                                <button
                                                                    class="text-green-600 hover:text-green-900 dark:text-green-400"
                                                                    on:click={
                                                                        let agent = agent_for_test.clone();
                                                                        move |_| open_chat_test(agent.clone())
                                                                    }
                                                                >
                                                                    "Test"
                                                                </button>
                                                                <a
                                                                    href=format!("/agents/edit/{}", name_for_edit)
                                                                    class="text-blue-600 hover:text-blue-900 dark:text-blue-400"
                                                                >
                                                                    "Edit"
                                                                </a>
                                                                <button
                                                                    class="text-red-600 hover:text-red-900 dark:text-red-400"
                                                                    on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                                                >
                                                                    "Delete"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_any()
                            } else {
                                // Card view
                                view! {
                                    <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                                        {agents_list.into_iter().map(|agent| {
                                            let agent_for_test = agent.clone();
                                            let name_for_edit = agent.name.clone();
                                            let name_for_delete = agent.name.clone();
                                            view! {
                                                <div class="bg-white dark:bg-gray-800 rounded-lg shadow border border-gray-200 dark:border-gray-700 p-6">
                                                    <div class="flex justify-between items-start mb-4">
                                                        <div>
                                                            <h3 class="text-lg font-medium text-gray-900 dark:text-white">{agent.name.clone()}</h3>
                                                            <span class=format!(
                                                                "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium mt-1 {}",
                                                                match agent.agent_type {
                                                                    AgentType::SingleTurn => "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300",
                                                                    AgentType::MultiTurn => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300",
                                                                    AgentType::ReAct => "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-300",
                                                                }
                                                            )>
                                                                {format!("{:?}", agent.agent_type)}
                                                            </span>
                                                        </div>
                                                        <div class="flex space-x-2">
                                                            <button
                                                                class="text-green-600 hover:text-green-900"
                                                                on:click={
                                                                    let agent = agent_for_test.clone();
                                                                    move |_| open_chat_test(agent.clone())
                                                                }
                                                            >
                                                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                                </svg>
                                                            </button>
                                                        </div>
                                                    </div>
                                                    <p class="text-sm text-gray-500 dark:text-gray-400 mb-4 line-clamp-2">{agent.description.clone()}</p>
                                                    <div class="space-y-2 text-sm">
                                                        <div class="flex justify-between">
                                                            <span class="text-gray-500 dark:text-gray-400">"LLM:"</span>
                                                            <span class="text-gray-900 dark:text-white">{format!("{:?} - {}", agent.llm.provider, agent.llm.model)}</span>
                                                        </div>
                                                        <div class="flex justify-between">
                                                            <span class="text-gray-500 dark:text-gray-400">"Tools:"</span>
                                                            <span class="text-gray-900 dark:text-white">{agent.available_tools.len()}</span>
                                                        </div>
                                                        {if !agent.mcp_tools.is_empty() {
                                                            view! {
                                                                <div class="flex justify-between">
                                                                    <span class="text-gray-500 dark:text-gray-400">"MCP Tools:"</span>
                                                                    <span class="text-purple-600 dark:text-purple-400">{agent.mcp_tools.len()}</span>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span></span> }.into_any()
                                                        }}
                                                    </div>
                                                    <div class="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700 flex justify-end space-x-2">
                                                        <a
                                                            href=format!("/agents/edit/{}", name_for_edit)
                                                            class="px-3 py-1 text-sm text-blue-600 hover:text-blue-800 dark:text-blue-400"
                                                        >
                                                            "Edit"
                                                        </a>
                                                        <button
                                                            class="px-3 py-1 text-sm text-red-600 hover:text-red-800 dark:text-red-400"
                                                            on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                                        >
                                                            "Delete"
                                                        </button>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }
                        }
                        Some(_) => view! {
                            <div class="text-center py-12 bg-white dark:bg-gray-800 rounded-lg shadow border border-gray-200 dark:border-gray-700">
                                <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                                </svg>
                                <h3 class="mt-2 text-sm font-medium text-gray-900 dark:text-white">"No agents configured"</h3>
                                <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">"Get started by creating a new AI agent."</p>
                                <div class="mt-6">
                                    <a
                                        href="/agents/new"
                                        class="inline-flex items-center px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700"
                                    >
                                        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                                        </svg>
                                        "New Agent"
                                    </a>
                                </div>
                            </div>
                        }.into_any(),
                        None => view! {
                            <div class="text-center py-8 text-red-500">"Failed to load agents"</div>
                        }.into_any(),
                    }
                }}
            </Suspense>

            // Delete confirmation modal
            {move || delete_target.get().map(|name| view! {
                <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
                    <div class="relative top-20 mx-auto p-5 border w-96 shadow-lg rounded-md bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700">
                        <div class="mt-3 text-center">
                            <div class="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-red-100 dark:bg-red-900">
                                <svg class="h-6 w-6 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                </svg>
                            </div>
                            <h3 class="text-lg leading-6 font-medium text-gray-900 dark:text-white mt-4">"Delete Agent"</h3>
                            <div class="mt-2 px-7 py-3">
                                <p class="text-sm text-gray-500 dark:text-gray-400">
                                    "Are you sure you want to delete agent \""{name.clone()}"\"? This action cannot be undone."
                                </p>
                            </div>
                            <div class="flex justify-center space-x-4 mt-4">
                                <button
                                    class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-300"
                                    on:click=move |_| set_delete_target.set(None)
                                    disabled=deleting
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700 disabled:opacity-50"
                                    on:click=on_delete_confirm
                                    disabled=deleting
                                >
                                    {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            })}

            // Chat test modal - Full chat interface
            {move || test_target.get().map(|agent| {
                let agent_provider = agent.llm.provider.clone();
                let agent_name_display = agent.name.clone();

                view! {
                    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 z-50 flex items-center justify-center p-4">
                        <div class="w-full max-w-3xl h-[80vh] flex flex-col bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-200 dark:border-gray-700">
                            // Header with tabs
                            <div class="flex-shrink-0 border-b border-gray-200 dark:border-gray-700">
                                <div class="flex items-center justify-between px-4 py-3">
                                    <div class="flex items-center space-x-3">
                                        <div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
                                            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                                            </svg>
                                        </div>
                                        <div>
                                            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">{agent_name_display}</h3>
                                            <p class="text-xs text-gray-500 dark:text-gray-400">"AI Agent Test Chat"</p>
                                        </div>
                                    </div>
                                    <div class="flex items-center space-x-2">
                                        // Tab buttons
                                        <div class="flex bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
                                            <button
                                                class=move || format!(
                                                    "px-3 py-1 text-xs font-medium rounded-md transition-colors {}",
                                                    if chat_view_tab.get() == "chat" {
                                                        "bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm"
                                                    } else {
                                                        "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                                                    }
                                                )
                                                on:click=move |_| set_chat_view_tab.set("chat".to_string())
                                            >
                                                "Chat"
                                            </button>
                                            <button
                                                class=move || format!(
                                                    "px-3 py-1 text-xs font-medium rounded-md transition-colors {}",
                                                    if chat_view_tab.get() == "raw" {
                                                        "bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm"
                                                    } else {
                                                        "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                                                    }
                                                )
                                                on:click=move |_| set_chat_view_tab.set("raw".to_string())
                                            >
                                                "Raw"
                                            </button>
                                        </div>
                                        // Clear chat button
                                        <button
                                            class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded"
                                            title="Clear chat"
                                            on:click=move |_| {
                                                set_chat_messages.set(Vec::new());
                                                set_chat_session_id.set(None);
                                            }
                                        >
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                            </svg>
                                        </button>
                                        // Close button
                                        <button
                                            class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded"
                                            on:click=move |_| set_test_target.set(None)
                                        >
                                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            </div>

                            // Content area (Chat or Raw view)
                            {move || if chat_view_tab.get() == "chat" {
                                let provider = agent_provider.clone();
                                view! {
                                    // Chat messages area
                                    <div class="flex-1 overflow-y-auto p-4 space-y-4 bg-gray-50 dark:bg-gray-900">
                                        {move || {
                                            let messages = chat_messages.get();
                                            if messages.is_empty() {
                                                view! {
                                                    <div class="flex flex-col items-center justify-center h-full text-center">
                                                        <div class="w-16 h-16 rounded-full bg-gray-200 dark:bg-gray-700 flex items-center justify-center mb-4">
                                                            <svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"/>
                                                            </svg>
                                                        </div>
                                                        <h4 class="text-lg font-medium text-gray-700 dark:text-gray-300 mb-2">"Start a conversation"</h4>
                                                        <p class="text-sm text-gray-500 dark:text-gray-400 max-w-sm">
                                                            "Type a message below to test this AI agent. The conversation will be displayed here."
                                                        </p>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="space-y-5">
                                                        {messages.into_iter().map(|msg| {
                                                            let provider_clone = provider.clone();
                                                            match msg.role {
                                                                ChatRole::User => view! {
                                                                    <div class="flex justify-end items-end gap-2">
                                                                        <div class="max-w-[75%] rounded-2xl rounded-br-md px-4 py-2.5 bg-gradient-to-br from-blue-500 to-blue-600 text-white shadow-md">
                                                                            <div class="text-sm whitespace-pre-wrap leading-relaxed">{msg.content}</div>
                                                                        </div>
                                                                        <div class="flex-shrink-0 w-7 h-7 rounded-full bg-blue-600 flex items-center justify-center shadow-sm">
                                                                            <svg class="w-4 h-4 text-white" fill="currentColor" viewBox="0 0 24 24">
                                                                                <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                                                                            </svg>
                                                                        </div>
                                                                    </div>
                                                                }.into_any(),
                                                                ChatRole::Assistant => {
                                                                    let has_error = msg.error.is_some();
                                                                    let error_text = msg.error.clone();
                                                                    let exec_time = msg.execution_time_ms;
                                                                    view! {
                                                                        <div class="flex justify-start items-end gap-2">
                                                                            <div class="flex-shrink-0 w-7 h-7 rounded-full bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center shadow-sm">
                                                                                <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                                                                                </svg>
                                                                            </div>
                                                                            <div class="max-w-[75%]">
                                                                                <div class="rounded-2xl rounded-bl-md px-4 py-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 shadow-md">
                                                                                    <div
                                                                                        class="text-sm text-gray-800 dark:text-gray-200"
                                                                                        inner_html=render_markdown(&msg.content)
                                                                                    />
                                                                                </div>
                                                                                <div class="flex items-center mt-1.5 ml-1 space-x-2">
                                                                                    {exec_time.map(|ms| view! {
                                                                                        <span class="text-xs text-gray-400 flex items-center gap-1">
                                                                                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                                                            </svg>
                                                                                            {format!("{}ms", ms)}
                                                                                        </span>
                                                                                    })}
                                                                                    {if has_error {
                                                                                        view! {
                                                                                            <span class="text-xs text-amber-500 flex items-center gap-1" title=error_text.unwrap_or_default()>
                                                                                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                                                                                </svg>
                                                                                                "warning"
                                                                                            </span>
                                                                                        }.into_any()
                                                                                    } else {
                                                                                        view! { <span></span> }.into_any()
                                                                                    }}
                                                                                </div>
                                                                            </div>
                                                                        </div>
                                                                    }.into_any()
                                                                },
                                                                ChatRole::System => {
                                                                    let error_text = msg.error.clone().unwrap_or_default();
                                                                    if is_api_key_error(&error_text) {
                                                                        let (title, guidance) = format_api_key_error(&error_text, Some(&provider_clone));
                                                                        view! {
                                                                            <div class="flex justify-center">
                                                                                <div class="max-w-[90%] p-4 bg-amber-50 dark:bg-amber-900/30 rounded-lg border border-amber-200 dark:border-amber-800">
                                                                                    <div class="flex items-start">
                                                                                        <svg class="w-5 h-5 text-amber-600 dark:text-amber-400 mt-0.5 mr-3 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                                                                        </svg>
                                                                                        <div>
                                                                                            <h4 class="text-sm font-semibold text-amber-800 dark:text-amber-200">{title}</h4>
                                                                                            <pre class="mt-2 text-xs text-amber-700 dark:text-amber-300 whitespace-pre-wrap font-sans">{guidance}</pre>
                                                                                        </div>
                                                                                    </div>
                                                                                </div>
                                                                            </div>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! {
                                                                            <div class="flex justify-center">
                                                                                <div class="max-w-[90%] p-3 bg-red-50 dark:bg-red-900/30 rounded-lg border border-red-200 dark:border-red-800">
                                                                                    <div class="flex items-start text-sm text-red-700 dark:text-red-300">
                                                                                        <svg class="w-4 h-4 mr-2 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                                                                        </svg>
                                                                                        <span>{error_text}</span>
                                                                                    </div>
                                                                                </div>
                                                                            </div>
                                                                        }.into_any()
                                                                    }
                                                                },
                                                            }
                                                        }).collect::<Vec<_>>()}

                                                        // Typing indicator when sending
                                                        {move || if sending.get() {
                                                            view! {
                                                                <div class="flex justify-start items-end gap-2">
                                                                    <div class="flex-shrink-0 w-7 h-7 rounded-full bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center shadow-sm animate-pulse">
                                                                        <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                                                                        </svg>
                                                                    </div>
                                                                    <div class="rounded-2xl rounded-bl-md px-4 py-3 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 shadow-md">
                                                                        <div class="flex items-center space-x-1.5">
                                                                            <div class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 0ms"></div>
                                                                            <div class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 150ms"></div>
                                                                            <div class="w-2 h-2 bg-purple-400 rounded-full animate-bounce" style="animation-delay: 300ms"></div>
                                                                        </div>
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! { <div></div> }.into_any()
                                                        }}
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>

                                    // Input area - dynamic form or simple text input
                                    <div class="flex-shrink-0 border-t border-gray-200 dark:border-gray-700 p-4 bg-white dark:bg-gray-800">
                                        {move || {
                                            if let Some(agent) = test_target.get() {
                                                if has_custom_schema(&agent) {
                                                    // Dynamic form for custom input schema
                                                    let properties = get_schema_properties(&agent);
                                                    view! {
                                                        <div class="space-y-4">
                                                            <div class="grid grid-cols-1 gap-3">
                                                                {properties.into_iter().map(|(name, prop_type, description, is_required)| {
                                                                    let name_clone = name.clone();
                                                                    let name_for_input = name.clone();
                                                                    let name_for_value = name.clone();
                                                                    view! {
                                                                        <div class="space-y-1">
                                                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                                                                {name_clone}
                                                                                {if is_required {
                                                                                    view! { <span class="text-red-500 ml-1">"*"</span> }.into_any()
                                                                                } else {
                                                                                    view! { <span></span> }.into_any()
                                                                                }}
                                                                                <span class="text-xs text-gray-400 ml-2">"("{prop_type}")"</span>
                                                                            </label>
                                                                            {if !description.is_empty() {
                                                                                view! { <p class="text-xs text-gray-500 dark:text-gray-400">{description}</p> }.into_any()
                                                                            } else {
                                                                                view! { <span></span> }.into_any()
                                                                            }}
                                                                            <input
                                                                                type="text"
                                                                                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg shadow-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white text-sm"
                                                                                prop:value=move || form_values.get().get(&name_for_value).cloned().unwrap_or_default()
                                                                                on:input=move |ev| {
                                                                                    let val = event_target_value(&ev);
                                                                                    set_form_values.update(|fv| {
                                                                                        fv.insert(name_for_input.clone(), val);
                                                                                    });
                                                                                }
                                                                                disabled=sending
                                                                            />
                                                                        </div>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                            <div class="flex justify-end">
                                                                <button
                                                                    class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center space-x-2"
                                                                    on:click=on_send_click
                                                                    disabled=move || sending.get() || form_values.get().values().all(|v| v.trim().is_empty())
                                                                >
                                                                    {move || if sending.get() {
                                                                        view! {
                                                                            <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                                                                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                                            </svg>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! {
                                                                            <span class="flex items-center space-x-2">
                                                                                <span>"Send"</span>
                                                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"/>
                                                                                </svg>
                                                                            </span>
                                                                        }.into_any()
                                                                    }}
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    // Simple text input for prompt-only agents
                                                    view! {
                                                        <div class="flex space-x-3">
                                                            <textarea
                                                                rows="1"
                                                                class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-xl shadow-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white resize-none"
                                                                placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
                                                                prop:value=move || chat_input.get()
                                                                on:input=move |ev| set_chat_input.set(event_target_value(&ev))
                                                                on:keydown=on_chat_keydown
                                                                disabled=sending
                                                            />
                                                            <button
                                                                class="px-4 py-2 bg-blue-600 text-white rounded-xl hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center justify-center"
                                                                on:click=on_send_click
                                                                disabled=move || sending.get() || chat_input.get().trim().is_empty()
                                                            >
                                                                {move || if sending.get() {
                                                                    view! {
                                                                        <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
                                                                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                                        </svg>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"/>
                                                                        </svg>
                                                                    }.into_any()
                                                                }}
                                                            </button>
                                                        </div>
                                                    }.into_any()
                                                }
                                            } else {
                                                view! { <div></div> }.into_any()
                                            }
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                // Raw view - show all messages as JSON
                                view! {
                                    <div class="flex-1 overflow-y-auto p-4 bg-gray-900">
                                        <pre class="text-xs text-green-400 font-mono whitespace-pre-wrap">
                                            {move || {
                                                let messages = chat_messages.get();
                                                let raw_data: Vec<serde_json::Value> = messages.iter().map(|msg| {
                                                    let mut obj = serde_json::json!({
                                                        "role": match msg.role {
                                                            ChatRole::User => "user",
                                                            ChatRole::Assistant => "assistant",
                                                            ChatRole::System => "system",
                                                        },
                                                        "content": msg.content,
                                                        "timestamp": msg.timestamp,
                                                    });
                                                    if let Some(ref raw) = msg.raw_output {
                                                        obj["raw_output"] = raw.clone();
                                                    }
                                                    if let Some(ms) = msg.execution_time_ms {
                                                        obj["execution_time_ms"] = serde_json::json!(ms);
                                                    }
                                                    if let Some(ref err) = msg.error {
                                                        obj["error"] = serde_json::json!(err);
                                                    }
                                                    obj
                                                }).collect();
                                                serde_json::to_string_pretty(&raw_data).unwrap_or_else(|_| "[]".to_string())
                                            }}
                                        </pre>
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    </div>
                }
            })}
        </div>
    }
}

/// Agent create form component
#[component]
pub fn AgentForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (agent_type, set_agent_type) = signal(AgentType::SingleTurn);
    let (provider, set_provider) = signal(AgentLlmProvider::OpenAI);
    let (model, set_model) = signal(get_default_model(&AgentLlmProvider::OpenAI).to_string());
    let (api_key_env, set_api_key_env) = signal(get_default_api_key_env(&AgentLlmProvider::OpenAI).to_string());
    let (base_url, set_base_url) = signal(String::new());
    let (system_prompt, set_system_prompt) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    // Memory configuration (for MultiTurn and ReAct agents)
    let (memory_backend, set_memory_backend) = signal(MemoryBackend::InMemory);
    let (memory_strategy, set_memory_strategy) = signal("full".to_string());
    let (memory_max_messages, set_memory_max_messages) = signal(100u32);
    let (memory_window_size, set_memory_window_size) = signal(20usize);

    // ReAct specific settings
    let (max_iterations, set_max_iterations) = signal(10u32);

    // Schema configuration - using hierarchical editor like tools
    let (input_schema_properties, set_input_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (output_schema_properties, set_output_schema_properties) = signal(Vec::<SchemaProperty>::new());

    // Prompt template for custom input schemas
    let (prompt_template, set_prompt_template) = signal(String::new());

    // Help popup for agent type
    let (show_type_help, set_show_type_help) = signal(false);

    // Artifacts configuration (tools, agents, workflows, resources for ReAct agents)
    let (all_tools, set_all_tools) = signal(Vec::<crate::types::Tool>::new());
    let (all_agents, set_all_agents) = signal(Vec::<Agent>::new());
    let (all_workflows, set_all_workflows) = signal(Vec::<crate::types::Workflow>::new());
    let (all_resources, set_all_resources) = signal(Vec::<crate::types::Resource>::new());
    let (all_resource_templates, set_all_resource_templates) = signal(Vec::<crate::types::ResourceTemplate>::new());
    let (selected_artifacts, set_selected_artifacts) = signal(Vec::<String>::new());
    let (artifacts_loading, set_artifacts_loading) = signal(false);

    // Dynamic model list from API
    let (available_models, set_available_models) = signal(Vec::<LlmModelInfo>::new());
    let (models_loading, set_models_loading) = signal(false);
    let (models_error, set_models_error) = signal(Option::<String>::None);

    // Fetch models when provider changes
    let fetch_models = move |prov: AgentLlmProvider, base: Option<String>, api_env: Option<String>| {
        set_models_loading.set(true);
        set_models_error.set(None);
        let provider_str = provider_to_string(&prov).to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match api::fetch_llm_models(&provider_str, base.as_deref(), api_env.as_deref()).await {
                Ok(models) => {
                    set_available_models.set(models);
                    set_models_loading.set(false);
                }
                Err(e) => {
                    set_models_error.set(Some(e));
                    set_models_loading.set(false);
                }
            }
        });
    };

    // Fetch all available artifacts on mount
    Effect::new(move |_| {
        set_artifacts_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            // Fetch all artifacts sequentially
            if let Ok(tools) = api::list_tools().await {
                set_all_tools.set(tools);
            }
            if let Ok(agents) = api::list_agents().await {
                set_all_agents.set(agents);
            }
            if let Ok(workflows) = api::list_workflows().await {
                set_all_workflows.set(workflows);
            }
            if let Ok(resources) = api::list_resources().await {
                set_all_resources.set(resources);
            }
            if let Ok(templates) = api::list_resource_templates().await {
                set_all_resource_templates.set(templates);
            }
            set_artifacts_loading.set(false);
        });
    });

    // Initial fetch on mount
    Effect::new(move |_| {
        fetch_models(AgentLlmProvider::OpenAI, None, Some("OPENAI_API_KEY".to_string()));
    });

    let on_save = move |_| {
        set_saving.set(true);
        set_error.set(None);

        // Build schemas from properties
        let input_props = input_schema_properties.get();
        let input_schema = if input_props.is_empty() {
            serde_json::json!({})
        } else {
            properties_to_schema(&input_props)
        };

        let output_props = output_schema_properties.get();
        let output_schema = if output_props.is_empty() {
            None
        } else {
            Some(properties_to_schema(&output_props))
        };

        // Extract artifacts by category
        let artifacts = selected_artifacts.get();
        let tools_list = all_tools.get();
        let workflows_list = all_workflows.get();
        let agents_list = all_agents.get();
        let resources_list = all_resources.get();
        let templates_list = all_resource_templates.get();

        let tool_names: std::collections::HashSet<_> = tools_list.iter().map(|t| t.name.clone()).collect();
        let workflow_ids: std::collections::HashSet<_> = workflows_list.iter().map(|w| format!("workflow_{}", w.name)).collect();
        let agent_names: std::collections::HashSet<_> = agents_list.iter().map(|a| a.name.clone()).collect();
        let resource_uris: std::collections::HashSet<_> = resources_list.iter().map(|r| r.uri.clone()).collect();
        let template_uris: std::collections::HashSet<_> = templates_list.iter().map(|t| t.uri_template.clone()).collect();

        let available_tools: Vec<String> = artifacts.iter()
            .filter(|id| tool_names.contains(*id) || workflow_ids.contains(*id))
            .cloned()
            .collect();
        let mcp_tools: Vec<String> = artifacts.iter()
            .filter(|id| id.contains(':') && !resource_uris.contains(*id) && !template_uris.contains(*id))
            .cloned()
            .collect();
        let agent_tools: Vec<String> = artifacts.iter()
            .filter(|id| agent_names.contains(*id))
            .cloned()
            .collect();
        let available_resources: Vec<String> = artifacts.iter()
            .filter(|id| resource_uris.contains(*id))
            .cloned()
            .collect();
        let available_resource_templates: Vec<String> = artifacts.iter()
            .filter(|id| template_uris.contains(*id))
            .cloned()
            .collect();

        let agent = Agent {
            name: name.get(),
            description: description.get(),
            agent_type: agent_type.get(),
            llm: AgentLlmConfig {
                provider: provider.get(),
                model: model.get(),
                api_key_env: if api_key_env.get().is_empty() { None } else { Some(api_key_env.get()) },
                base_url: if base_url.get().is_empty() { None } else { Some(base_url.get()) },
                temperature: None,
                max_tokens: None,
                stream: true,
            },
            system_prompt: system_prompt.get(),
            prompt_template: if prompt_template.get().is_empty() { None } else { Some(prompt_template.get()) },
            available_tools,
            mcp_tools,
            agent_tools,
            available_resources,
            available_resource_templates,
            memory: MemoryConfig {
                backend: memory_backend.get(),
                strategy: match memory_strategy.get().as_str() {
                    "sliding_window" => MemoryStrategy::SlidingWindow { size: memory_window_size.get() },
                    _ => MemoryStrategy::Full,
                },
                max_messages: memory_max_messages.get(),
                file_path: None,
                database_url: None,
            },
            max_iterations: max_iterations.get(),
            timeout_seconds: 300,
            temperature: None,
            max_tokens: None,
            input_schema,
            output_schema,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_agent(&agent).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/agents");
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6">
            <div class="max-w-4xl mx-auto space-y-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">"New Agent"</h2>
                        <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">"Create a new AI agent"</p>
                    </div>
                    <a
                        href="/agents"
                        class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                    >
                        "Back to Agents"
                    </a>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="p-4 bg-red-50 dark:bg-red-900/20 rounded-md text-red-700 dark:text-red-300 border border-red-200 dark:border-red-800">
                        {e}
                    </div>
                })}

                <div class="bg-white dark:bg-gray-800 shadow rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-6">
                    // Basic Info
                    <div class="grid grid-cols-2 gap-4">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Name"</label>
                            <input
                                type="text"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                placeholder="my-agent"
                                prop:value=move || name.get()
                                on:input=move |ev| set_name.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="relative">
                            <div class="flex items-center gap-1 mb-1">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Type"</label>
                                <button
                                    type="button"
                                    class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none"
                                    on:click=move |_| set_show_type_help.update(|v| *v = !*v)
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                    </svg>
                                </button>
                            </div>
                            // Help popup
                            <Show when=move || show_type_help.get()>
                                <div class="absolute z-50 top-7 left-0 w-80 bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-200 dark:border-gray-700 p-4">
                                    <div class="flex justify-between items-start mb-3">
                                        <h4 class="font-semibold text-gray-900 dark:text-white">"Agent Types"</h4>
                                        <button
                                            type="button"
                                            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                            on:click=move |_| set_show_type_help.set(false)
                                        >
                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                            </svg>
                                        </button>
                                    </div>
                                    <div class="space-y-3 text-sm">
                                        <div>
                                            <span class="font-medium text-blue-600 dark:text-blue-400">"Single Turn"</span>
                                            <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                "One request, one response. No conversation history. Best for stateless tasks like classification, translation, or one-off queries."
                                            </p>
                                        </div>
                                        <div>
                                            <span class="font-medium text-green-600 dark:text-green-400">"Multi Turn"</span>
                                            <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                "Maintains conversation history across messages. Ideal for chatbots, assistants, and interactive dialogues where context matters."
                                            </p>
                                        </div>
                                        <div>
                                            <span class="font-medium text-purple-600 dark:text-purple-400">"ReAct"</span>
                                            <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                "Reasoning + Action loop with tool calling. The agent can use tools, analyze results, and iterate. Best for complex tasks requiring external data or actions."
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            </Show>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                on:change=move |ev| {
                                    set_agent_type.set(match event_target_value(&ev).as_str() {
                                        "multi_turn" => AgentType::MultiTurn,
                                        "react" => AgentType::ReAct,
                                        _ => AgentType::SingleTurn,
                                    });
                                }
                            >
                                <option value="single_turn" selected=move || agent_type.get() == AgentType::SingleTurn>"Single Turn"</option>
                                <option value="multi_turn" selected=move || agent_type.get() == AgentType::MultiTurn>"Multi Turn"</option>
                                <option value="react" selected=move || agent_type.get() == AgentType::ReAct>"ReAct"</option>
                            </select>
                        </div>
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Description"</label>
                        <textarea
                            rows="2"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                            placeholder="A helpful assistant that..."
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        />
                    </div>

                    // Artifacts Configuration (for ReAct agents)
                    {move || {
                        if agent_type.get() == AgentType::ReAct {
                            // Build unified artifacts list
                            let all_artifacts = Signal::derive(move || {
                                let current_name = name.get();
                                let mut items = Vec::new();

                                // Add regular tools
                                for tool in all_tools.get().into_iter().filter(|t| !t.name.starts_with("mcp__")) {
                                    items.push(ArtifactItem::tool(tool.name, tool.description));
                                }

                                // Add MCP tools
                                for tool in all_tools.get().into_iter().filter(|t| t.name.starts_with("mcp__")) {
                                    let parts: Vec<&str> = tool.name.strip_prefix("mcp__")
                                        .unwrap_or(&tool.name)
                                        .splitn(2, '_')
                                        .collect();
                                    if parts.len() == 2 {
                                        items.push(ArtifactItem::mcp_tool(parts[0], parts[1], tool.description));
                                    }
                                }

                                // Add workflows
                                for workflow in all_workflows.get() {
                                    items.push(ArtifactItem::workflow(workflow.name, workflow.description));
                                }

                                // Add other agents (not self)
                                for agent in all_agents.get().into_iter().filter(|a| a.name != current_name) {
                                    items.push(ArtifactItem::agent(agent.name, agent.description));
                                }

                                // Add resources
                                for resource in all_resources.get() {
                                    items.push(ArtifactItem::resource(
                                        resource.uri,
                                        resource.name,
                                        resource.description.unwrap_or_default()
                                    ));
                                }

                                // Add resource templates
                                for template in all_resource_templates.get() {
                                    items.push(ArtifactItem::resource_template(
                                        template.uri_template,
                                        template.name,
                                        template.description.unwrap_or_default()
                                    ));
                                }

                                items
                            });

                            view! {
                                <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"Artifacts Configuration"</h3>
                                    <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                        "Select tools, workflows, agents, and resources the agent can use"
                                    </p>

                                    {if artifacts_loading.get() {
                                        view! {
                                            <div class="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                                                <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                                                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                    <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                                </svg>
                                                "Loading artifacts..."
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <ArtifactSelector
                                                items=all_artifacts
                                                selected=selected_artifacts.into()
                                                on_change=Callback::new(move |new_selected| set_selected_artifacts.set(new_selected))
                                                mode=SelectionMode::Multi
                                                placeholder="Search tools, workflows, agents, resources..."
                                                label="Available Artifacts"
                                                help_text="Search and select from tools, workflows, agents, resources, and MCP tools"
                                                group_by_category=true
                                            />
                                        }.into_any()
                                    }}
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}

                    // Memory & Iteration Configuration (for MultiTurn and ReAct)
                    {move || {
                        let at = agent_type.get();
                        if at == AgentType::MultiTurn || at == AgentType::ReAct {
                            view! {
                                <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                                    <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">
                                        {if at == AgentType::ReAct { "Memory & ReAct Configuration" } else { "Memory Configuration" }}
                                    </h3>
                                    <div class="grid grid-cols-2 gap-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Memory Backend"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                on:change=move |ev| {
                                                    set_memory_backend.set(match event_target_value(&ev).as_str() {
                                                        "file" => MemoryBackend::File,
                                                        "database" => MemoryBackend::Database,
                                                        _ => MemoryBackend::InMemory,
                                                    });
                                                }
                                            >
                                                <option value="in_memory" selected=move || memory_backend.get() == MemoryBackend::InMemory>"In Memory"</option>
                                                <option value="file" selected=move || memory_backend.get() == MemoryBackend::File>"File"</option>
                                                <option value="database" selected=move || memory_backend.get() == MemoryBackend::Database>"Database"</option>
                                            </select>
                                            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Where to store conversation history"</p>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Memory Strategy"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                on:change=move |ev| set_memory_strategy.set(event_target_value(&ev))
                                            >
                                                <option value="full" selected=move || memory_strategy.get() == "full">"Full History"</option>
                                                <option value="sliding_window" selected=move || memory_strategy.get() == "sliding_window">"Sliding Window"</option>
                                            </select>
                                            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"How to manage message history"</p>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Max Messages"</label>
                                            <input
                                                type="number"
                                                min="10"
                                                max="1000"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                prop:value=move || memory_max_messages.get()
                                                on:input=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                                        set_memory_max_messages.set(v);
                                                    }
                                                }
                                            />
                                            <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Maximum messages to store"</p>
                                        </div>
                                        {move || if memory_strategy.get() == "sliding_window" {
                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Window Size"</label>
                                                    <input
                                                        type="number"
                                                        min="5"
                                                        max="100"
                                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                        prop:value=move || memory_window_size.get()
                                                        on:input=move |ev| {
                                                            if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                                                                set_memory_window_size.set(v);
                                                            }
                                                        }
                                                    />
                                                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Recent messages to include"</p>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}
                                        {move || if at == AgentType::ReAct {
                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Max Iterations"</label>
                                                    <input
                                                        type="number"
                                                        min="1"
                                                        max="50"
                                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                                        prop:value=move || max_iterations.get()
                                                        on:input=move |ev| {
                                                            if let Ok(v) = event_target_value(&ev).parse::<u32>() {
                                                                set_max_iterations.set(v);
                                                            }
                                                        }
                                                    />
                                                    <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">"Max reasoning/action cycles"</p>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! { <div></div> }.into_any()
                        }
                    }}

                    // LLM Configuration
                    <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"LLM Configuration"</h3>
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Provider"</label>
                                <select
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    on:change=move |ev| {
                                        let new_provider = match event_target_value(&ev).as_str() {
                                            "anthropic" => AgentLlmProvider::Anthropic,
                                            "gemini" => AgentLlmProvider::Gemini,
                                            "ollama" => AgentLlmProvider::Ollama,
                                            "azureopenai" => AgentLlmProvider::AzureOpenAI,
                                            _ => AgentLlmProvider::OpenAI,
                                        };
                                        set_provider.set(new_provider.clone());
                                        // Update model to default for new provider
                                        set_model.set(get_default_model(&new_provider).to_string());
                                        // Update API key env variable
                                        let new_api_key_env = get_default_api_key_env(&new_provider).to_string();
                                        set_api_key_env.set(new_api_key_env.clone());
                                        // Update base URL
                                        let new_base_url = get_default_base_url(&new_provider).to_string();
                                        set_base_url.set(new_base_url.clone());
                                        // Fetch models for new provider
                                        let base = if new_base_url.is_empty() { None } else { Some(new_base_url) };
                                        let api_env = if new_api_key_env.is_empty() { None } else { Some(new_api_key_env) };
                                        fetch_models(new_provider, base, api_env);
                                    }
                                >
                                    <option value="openai" selected=move || provider.get() == AgentLlmProvider::OpenAI>"OpenAI"</option>
                                    <option value="anthropic" selected=move || provider.get() == AgentLlmProvider::Anthropic>"Anthropic"</option>
                                    <option value="gemini" selected=move || provider.get() == AgentLlmProvider::Gemini>"Google Gemini"</option>
                                    <option value="ollama" selected=move || provider.get() == AgentLlmProvider::Ollama>"Ollama (Local)"</option>
                                    <option value="azureopenai" selected=move || provider.get() == AgentLlmProvider::AzureOpenAI>"Azure OpenAI"</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    "Model"
                                    {move || if models_loading.get() {
                                        view! { <span class="text-blue-500 text-xs ml-2">"Loading..."</span> }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </label>
                                <select
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    on:change=move |ev| set_model.set(event_target_value(&ev))
                                    disabled=models_loading
                                >
                                    {move || {
                                        let current_model = model.get();
                                        let models = available_models.get();
                                        if models.is_empty() {
                                            vec![view! {
                                                <option value=current_model.clone() selected=true>{current_model.clone()}</option>
                                            }]
                                        } else {
                                            models.into_iter().map(|m| {
                                                let is_selected = current_model == m.id;
                                                let label = if let Some(desc) = m.description {
                                                    format!("{} - {}", m.name, desc)
                                                } else {
                                                    m.name.clone()
                                                };
                                                view! {
                                                    <option value=m.id selected=is_selected>{label}</option>
                                                }
                                            }).collect::<Vec<_>>()
                                        }
                                    }}
                                </select>
                                {move || models_error.get().map(|e| view! {
                                    <p class="text-red-500 text-xs mt-1">{e}</p>
                                })}
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    "API Key Env Variable"
                                    {move || if provider.get() == AgentLlmProvider::Ollama {
                                        view! { <span class="text-gray-400 text-xs ml-1">"(optional for local)"</span> }.into_any()
                                    } else {
                                        view! { <span class="text-amber-600 dark:text-amber-400 text-xs ml-1">"(required)"</span> }.into_any()
                                    }}
                                </label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    placeholder=move || get_default_api_key_env(&provider.get())
                                    prop:value=move || api_key_env.get()
                                    on:input=move |ev| set_api_key_env.set(event_target_value(&ev))
                                />
                                // Warning when API key is required but not set
                                {move || {
                                    let prov = provider.get();
                                    let key = api_key_env.get();
                                    if provider_requires_api_key(&prov) && key.is_empty() {
                                        let default_var = get_default_api_key_env(&prov);
                                        view! {
                                            <div class="mt-1.5 p-2 bg-amber-50 dark:bg-amber-900/30 rounded border border-amber-200 dark:border-amber-700">
                                                <div class="flex items-start text-xs">
                                                    <svg class="w-4 h-4 text-amber-500 dark:text-amber-400 mr-1.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                                    </svg>
                                                    <span class="text-amber-700 dark:text-amber-300">
                                                        "Using default: "
                                                        <code class="bg-amber-100 dark:bg-amber-800 px-1 rounded">{default_var}</code>
                                                        ". Ensure this environment variable is set on the server."
                                                    </span>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    "Base URL"
                                    {move || if provider.get() == AgentLlmProvider::Ollama || provider.get() == AgentLlmProvider::AzureOpenAI {
                                        view! { <span class="text-gray-400 text-xs ml-1">"(required)"</span> }.into_any()
                                    } else {
                                        view! { <span class="text-gray-400 text-xs ml-1">"(optional)"</span> }.into_any()
                                    }}
                                </label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    placeholder=move || match provider.get() {
                                        AgentLlmProvider::Ollama => "http://localhost:11434",
                                        AgentLlmProvider::AzureOpenAI => "https://your-resource.openai.azure.com",
                                        _ => "Leave empty for default",
                                    }
                                    prop:value=move || base_url.get()
                                    on:input=move |ev| set_base_url.set(event_target_value(&ev))
                                />
                            </div>
                        </div>
                    </div>

                    // System Prompt
                    <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"System Prompt"</h3>
                        <textarea
                            rows="6"
                            class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white font-mono text-sm"
                            placeholder="You are a helpful assistant..."
                            prop:value=move || system_prompt.get()
                            on:input=move |ev| set_system_prompt.set(event_target_value(&ev))
                        />
                    </div>

                    // Schema Configuration
                    <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"Schema Configuration"</h3>
                        <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                            "Define JSON schemas for structured input/output when agent is exposed as a tool. "
                            "Custom input fields are serialized to the agent's prompt. Output schema enables JSON parsing of responses."
                        </p>
                        <div class="space-y-4">
                            <JsonSchemaEditor
                                properties=input_schema_properties
                                set_properties=set_input_schema_properties
                                label="Input Schema (optional)"
                                color="green"
                            />
                            <SchemaPreview properties=input_schema_properties />

                            // Prompt Template (shown when input schema has properties)
                            {move || {
                                let props = input_schema_properties.get();
                                if !props.is_empty() {
                                    let prop_names: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
                                    view! {
                                        <div class="mt-4 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg border border-purple-200 dark:border-purple-800">
                                            <label class="block text-sm font-medium text-purple-800 dark:text-purple-300 mb-2">
                                                "Prompt Template"
                                            </label>
                                            <p class="text-xs text-purple-600 dark:text-purple-400 mb-2">
                                                "Define how input values are formatted into the user prompt. Use "
                                                <code class="bg-purple-100 dark:bg-purple-800 px-1 rounded">"{{field_name}}"</code>
                                                " for variable substitution."
                                            </p>
                                            <div class="flex flex-wrap gap-1 mb-2">
                                                {prop_names.iter().map(|name| {
                                                    let var = format!("{{{{{}}}}}", name);
                                                    view! {
                                                        <span class="inline-flex items-center px-2 py-0.5 text-xs font-mono bg-purple-100 dark:bg-purple-800 text-purple-700 dark:text-purple-300 rounded">
                                                            {var}
                                                        </span>
                                                    }
                                                }).collect_view()}
                                            </div>
                                            <textarea
                                                rows="3"
                                                class="w-full px-3 py-2 border border-purple-300 dark:border-purple-600 rounded-md shadow-sm focus:ring-purple-500 focus:border-purple-500 dark:bg-gray-800 dark:text-white font-mono text-sm"
                                                placeholder="Example: Analyze the topic '{{topic}}' for a {{audience}} audience with focus on {{focus_area}}."
                                                prop:value=move || prompt_template.get()
                                                on:input=move |ev| set_prompt_template.set(event_target_value(&ev))
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}

                            <JsonSchemaEditor
                                properties=output_schema_properties
                                set_properties=set_output_schema_properties
                                label="Output Schema (optional)"
                                color="blue"
                            />
                            <SchemaPreview properties=output_schema_properties />
                        </div>
                    </div>

                    // Save button
                    <div class="border-t border-gray-200 dark:border-gray-700 pt-6 flex justify-end space-x-4">
                        <a
                            href="/agents"
                            class="px-4 py-2 text-gray-700 bg-gray-200 rounded-md hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-300"
                        >
                            "Cancel"
                        </a>
                        <button
                            class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                            on:click=on_save
                            disabled=saving
                        >
                            {move || if saving.get() { "Creating..." } else { "Create Agent" }}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Agent edit form component
#[component]
pub fn AgentEditForm() -> impl IntoView {
    let params = use_params_map();
    let agent_name = move || params.read().get("name").unwrap_or_default();

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (agent_type, set_agent_type) = signal(AgentType::SingleTurn);
    let (provider, set_provider) = signal(AgentLlmProvider::OpenAI);
    let (model, set_model) = signal(String::new());
    let (api_key_env, set_api_key_env) = signal(String::new());
    let (base_url, set_base_url) = signal(String::new());
    let (system_prompt, set_system_prompt) = signal(String::new());
    let (original_name, set_original_name) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let (loading, set_loading) = signal(true);

    // Dynamic model list from API
    let (available_models, set_available_models) = signal(Vec::<LlmModelInfo>::new());
    let (models_loading, set_models_loading) = signal(false);
    let (models_error, set_models_error) = signal(Option::<String>::None);

    // Tools configuration (for ReAct agents)
    let (all_tools, set_all_tools) = signal(Vec::<crate::types::Tool>::new());
    let (selected_tools, set_selected_tools) = signal(Vec::<String>::new());
    let (selected_mcp_tools, set_selected_mcp_tools) = signal(Vec::<String>::new());
    let (selected_agent_tools, set_selected_agent_tools) = signal(Vec::<String>::new());
    let (selected_workflow_tools, set_selected_workflow_tools) = signal(Vec::<String>::new());
    let (tools_loading, set_tools_loading) = signal(false);

    // Available agents (for agent-as-tool selection)
    let (all_agents, set_all_agents) = signal(Vec::<Agent>::new());

    // Available workflows (for workflow-as-tool selection)
    let (all_workflows, set_all_workflows) = signal(Vec::<crate::types::Workflow>::new());

    // Schema configuration - using hierarchical editor like tools
    let (input_schema_properties, set_input_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (output_schema_properties, set_output_schema_properties) = signal(Vec::<SchemaProperty>::new());

    // Prompt template for custom input schemas
    let (prompt_template, set_prompt_template) = signal(String::new());

    // Help popup for agent type
    let (show_type_help, set_show_type_help) = signal(false);

    // Fetch models function
    let fetch_models = move |prov: AgentLlmProvider, base: Option<String>, api_env: Option<String>| {
        set_models_loading.set(true);
        set_models_error.set(None);
        let provider_str = provider_to_string(&prov).to_string();
        wasm_bindgen_futures::spawn_local(async move {
            match api::fetch_llm_models(&provider_str, base.as_deref(), api_env.as_deref()).await {
                Ok(models) => {
                    set_available_models.set(models);
                    set_models_loading.set(false);
                }
                Err(e) => {
                    set_models_error.set(Some(e));
                    set_models_loading.set(false);
                }
            }
        });
    };

    // Fetch available tools on mount
    Effect::new(move |_| {
        set_tools_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::list_tools().await {
                Ok(tools) => {
                    set_all_tools.set(tools);
                    set_tools_loading.set(false);
                }
                Err(_) => {
                    set_tools_loading.set(false);
                }
            }
        });
    });

    // Fetch available agents on mount (for agent-as-tool selection)
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(agents) = api::list_agents().await {
                set_all_agents.set(agents);
            }
        });
    });

    // Fetch available workflows on mount (for workflow-as-tool selection)
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(workflows) = api::list_workflows().await {
                set_all_workflows.set(workflows);
            }
        });
    });

    // Load existing agent
    Effect::new(move |_| {
        let name_param = agent_name();
        // Skip if name is empty (params not ready yet)
        if name_param.is_empty() {
            return;
        }
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_agent(&name_param).await {
                Ok(agent) => {
                    set_original_name.set(agent.name.clone());
                    set_name.set(agent.name.clone());
                    set_description.set(agent.description.clone());
                    set_agent_type.set(agent.agent_type.clone());
                    let loaded_provider = agent.llm.provider.clone();
                    set_provider.set(loaded_provider.clone());
                    set_model.set(agent.llm.model.clone());
                    let loaded_api_key_env = agent.llm.api_key_env.clone().unwrap_or_default();
                    set_api_key_env.set(loaded_api_key_env.clone());
                    let loaded_base_url = agent.llm.base_url.clone().unwrap_or_default();
                    set_base_url.set(loaded_base_url.clone());
                    set_system_prompt.set(agent.system_prompt.clone());
                    set_prompt_template.set(agent.prompt_template.clone().unwrap_or_default());
                    set_selected_tools.set(agent.available_tools.clone());
                    set_selected_mcp_tools.set(agent.mcp_tools.clone());
                    set_selected_agent_tools.set(agent.agent_tools.clone());
                    // Load schema fields using schema_to_properties
                    set_input_schema_properties.set(schema_to_properties(&agent.input_schema));
                    if let Some(output_schema) = &agent.output_schema {
                        set_output_schema_properties.set(schema_to_properties(output_schema));
                    }
                    set_loading.set(false);

                    // Fetch models for the loaded provider
                    let base = if loaded_base_url.is_empty() { None } else { Some(loaded_base_url) };
                    let api_env = if loaded_api_key_env.is_empty() { None } else { Some(loaded_api_key_env) };
                    fetch_models(loaded_provider, base, api_env);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    let on_save = move |_| {
        set_saving.set(true);
        set_error.set(None);
        let orig_name = original_name.get();

        // Build schemas from properties
        let input_props = input_schema_properties.get();
        let input_schema = if input_props.is_empty() {
            serde_json::json!({})
        } else {
            properties_to_schema(&input_props)
        };

        let output_props = output_schema_properties.get();
        let output_schema = if output_props.is_empty() {
            None
        } else {
            Some(properties_to_schema(&output_props))
        };

        let agent = Agent {
            name: name.get(),
            description: description.get(),
            agent_type: agent_type.get(),
            llm: AgentLlmConfig {
                provider: provider.get(),
                model: model.get(),
                api_key_env: if api_key_env.get().is_empty() { None } else { Some(api_key_env.get()) },
                base_url: if base_url.get().is_empty() { None } else { Some(base_url.get()) },
                temperature: None,
                max_tokens: None,
                stream: true,
            },
            system_prompt: system_prompt.get(),
            prompt_template: if prompt_template.get().is_empty() { None } else { Some(prompt_template.get()) },
            available_tools: {
                let mut tools = selected_tools.get();
                // Add workflow tools to available_tools
                tools.extend(selected_workflow_tools.get());
                tools
            },
            mcp_tools: selected_mcp_tools.get(),
            agent_tools: selected_agent_tools.get(),
            available_resources: Vec::new(),
            available_resource_templates: Vec::new(),
            memory: MemoryConfig {
                backend: MemoryBackend::InMemory,
                strategy: MemoryStrategy::Full,
                max_messages: 100,
                file_path: None,
                database_url: None,
            },
            max_iterations: 10,
            timeout_seconds: 300,
            temperature: None,
            max_tokens: None,
            input_schema,
            output_schema,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_agent(&orig_name, &agent).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/agents");
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6">
            <div class="max-w-4xl mx-auto space-y-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">"Edit Agent"</h2>
                        <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">"Update agent configuration"</p>
                    </div>
                    <a
                        href="/agents"
                        class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                    >
                        "Back to Agents"
                    </a>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="p-4 bg-red-50 dark:bg-red-900/20 rounded-md text-red-700 dark:text-red-300 border border-red-200 dark:border-red-800">
                        {e}
                    </div>
                })}

                <Show when=move || !loading.get() fallback=|| view! { <div class="text-center py-8">"Loading..."</div> }>
                    <div class="bg-white dark:bg-gray-800 shadow rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-6">
                        // Basic Info
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Name"</label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white disabled:opacity-50"
                                    prop:value=move || name.get()
                                    on:input=move |ev| set_name.set(event_target_value(&ev))
                                    disabled=true
                                />
                            </div>
                            <div class="relative">
                                <div class="flex items-center gap-1 mb-1">
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">"Type"</label>
                                    <button
                                        type="button"
                                        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none"
                                        on:click=move |_| set_show_type_help.update(|v| *v = !*v)
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                        </svg>
                                    </button>
                                </div>
                                // Help popup
                                <Show when=move || show_type_help.get()>
                                    <div class="absolute z-50 top-7 left-0 w-80 bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-200 dark:border-gray-700 p-4">
                                        <div class="flex justify-between items-start mb-3">
                                            <h4 class="font-semibold text-gray-900 dark:text-white">"Agent Types"</h4>
                                            <button
                                                type="button"
                                                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                                on:click=move |_| set_show_type_help.set(false)
                                            >
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                                </svg>
                                            </button>
                                        </div>
                                        <div class="space-y-3 text-sm">
                                            <div>
                                                <span class="font-medium text-blue-600 dark:text-blue-400">"Single Turn"</span>
                                                <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                    "One request, one response. No conversation history. Best for stateless tasks like classification, translation, or one-off queries."
                                                </p>
                                            </div>
                                            <div>
                                                <span class="font-medium text-green-600 dark:text-green-400">"Multi Turn"</span>
                                                <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                    "Maintains conversation history across messages. Ideal for chatbots, assistants, and interactive dialogues where context matters."
                                                </p>
                                            </div>
                                            <div>
                                                <span class="font-medium text-purple-600 dark:text-purple-400">"ReAct"</span>
                                                <p class="text-gray-600 dark:text-gray-400 mt-0.5">
                                                    "Reasoning + Action loop with tool calling. The agent can use tools, analyze results, and iterate. Best for complex tasks requiring external data or actions."
                                                </p>
                                            </div>
                                        </div>
                                    </div>
                                </Show>
                                <select
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                    on:change=move |ev| {
                                        set_agent_type.set(match event_target_value(&ev).as_str() {
                                            "multi_turn" => AgentType::MultiTurn,
                                            "react" => AgentType::ReAct,
                                            _ => AgentType::SingleTurn,
                                        });
                                    }
                                >
                                    <option value="single_turn" selected=move || agent_type.get() == AgentType::SingleTurn>"Single Turn"</option>
                                    <option value="multi_turn" selected=move || agent_type.get() == AgentType::MultiTurn>"Multi Turn"</option>
                                    <option value="react" selected=move || agent_type.get() == AgentType::ReAct>"ReAct"</option>
                                </select>
                            </div>
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Description"</label>
                            <textarea
                                rows="2"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                prop:value=move || description.get()
                                on:input=move |ev| set_description.set(event_target_value(&ev))
                            />
                        </div>

                        // Tools Configuration (for ReAct agents)
                        {move || {
                            if agent_type.get() == AgentType::ReAct {
                                // Convert tools to SelectableItems
                                let regular_tools_items = Signal::derive(move || {
                                    all_tools.get()
                                        .into_iter()
                                        .filter(|t| !t.name.starts_with("mcp__"))
                                        .map(|t| SelectableItem {
                                            id: t.name.clone(),
                                            name: t.name.clone(),
                                            description: t.description.clone(),
                                            category: None,
                                        })
                                        .collect::<Vec<_>>()
                                });

                                let mcp_tools_items = Signal::derive(move || {
                                    all_tools.get()
                                        .into_iter()
                                        .filter(|t| t.name.starts_with("mcp__"))
                                        .map(|t| {
                                            // Extract server name from mcp__{server}_{tool}
                                            let display_name = t.name.strip_prefix("mcp__")
                                                .map(|s| s.replacen('_', ":", 1))
                                                .unwrap_or_else(|| t.name.clone());
                                            let server = t.name.strip_prefix("mcp__")
                                                .and_then(|s| s.split('_').next())
                                                .map(String::from);
                                            SelectableItem {
                                                id: t.name.clone(),
                                                name: display_name,
                                                description: t.description.clone(),
                                                category: server,
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                });

                                let agent_tools_items = Signal::derive(move || {
                                    let current_name = original_name.get();
                                    all_agents.get()
                                        .into_iter()
                                        .filter(|a| a.name != current_name)
                                        .map(|a| SelectableItem {
                                            id: a.name.clone(),
                                            name: a.name.clone(),
                                            description: a.description.clone(),
                                            category: Some(format!("{:?}", a.agent_type)),
                                        })
                                        .collect::<Vec<_>>()
                                });

                                let workflow_tools_items = Signal::derive(move || {
                                    all_workflows.get()
                                        .into_iter()
                                        .map(|w| SelectableItem {
                                            id: w.name.clone(),
                                            name: w.name.clone(),
                                            description: w.description.clone(),
                                            category: Some(format!("{} steps", w.steps.len())),
                                        })
                                        .collect::<Vec<_>>()
                                });

                                view! {
                                    <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                                        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"Tools Configuration"</h3>

                                        {if tools_loading.get() {
                                            view! {
                                                <div class="text-sm text-gray-500 dark:text-gray-400">"Loading tools..."</div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="space-y-4">
                                                    // Regular Tools
                                                    <SearchableMultiSelect
                                                        items=regular_tools_items
                                                        selected=selected_tools.into()
                                                        on_change=Callback::new(move |new_selected| set_selected_tools.set(new_selected))
                                                        placeholder="Search tools..."
                                                        theme=SelectTheme::Blue
                                                        label="Available Tools"
                                                        help_text="Select tools the agent can use"
                                                    />

                                                    // Workflow Tools
                                                    <SearchableMultiSelect
                                                        items=workflow_tools_items
                                                        selected=selected_workflow_tools.into()
                                                        on_change=Callback::new(move |new_selected| set_selected_workflow_tools.set(new_selected))
                                                        placeholder="Search workflows..."
                                                        theme=SelectTheme::Orange
                                                        label="Workflow Tools"
                                                        help_text="Workflows that can be called as tools"
                                                    />

                                                    // MCP Tools
                                                    <SearchableMultiSelect
                                                        items=mcp_tools_items
                                                        selected=selected_mcp_tools.into()
                                                        on_change=Callback::new(move |new_selected| set_selected_mcp_tools.set(new_selected))
                                                        placeholder="Search MCP tools..."
                                                        theme=SelectTheme::Purple
                                                        label="MCP Tools"
                                                        help_text="Tools from external MCP servers"
                                                    />

                                                    // Agent Tools
                                                    <SearchableMultiSelect
                                                        items=agent_tools_items
                                                        selected=selected_agent_tools.into()
                                                        on_change=Callback::new(move |new_selected| set_selected_agent_tools.set(new_selected))
                                                        placeholder="Search agents..."
                                                        theme=SelectTheme::Indigo
                                                        label="Agent Tools"
                                                        help_text="Other agents that can be called as tools"
                                                    />
                                                </div>
                                            }.into_any()
                                        }}
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }}

                        // LLM Configuration
                        <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"LLM Configuration"</h3>
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">"Provider"</label>
                                    <select
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                        on:change=move |ev| {
                                            let new_provider = match event_target_value(&ev).as_str() {
                                                "anthropic" => AgentLlmProvider::Anthropic,
                                                "gemini" => AgentLlmProvider::Gemini,
                                                "ollama" => AgentLlmProvider::Ollama,
                                                "azureopenai" => AgentLlmProvider::AzureOpenAI,
                                                _ => AgentLlmProvider::OpenAI,
                                            };
                                            set_provider.set(new_provider.clone());
                                            // Update model to default for new provider
                                            set_model.set(get_default_model(&new_provider).to_string());
                                            // Update API key env variable
                                            let new_api_key_env = get_default_api_key_env(&new_provider).to_string();
                                            set_api_key_env.set(new_api_key_env.clone());
                                            // Update base URL
                                            let new_base_url = get_default_base_url(&new_provider).to_string();
                                            set_base_url.set(new_base_url.clone());
                                            // Fetch models for new provider
                                            let base = if new_base_url.is_empty() { None } else { Some(new_base_url) };
                                            let api_env = if new_api_key_env.is_empty() { None } else { Some(new_api_key_env) };
                                            fetch_models(new_provider, base, api_env);
                                        }
                                    >
                                        <option value="openai" selected=move || provider.get() == AgentLlmProvider::OpenAI>"OpenAI"</option>
                                        <option value="anthropic" selected=move || provider.get() == AgentLlmProvider::Anthropic>"Anthropic"</option>
                                        <option value="gemini" selected=move || provider.get() == AgentLlmProvider::Gemini>"Google Gemini"</option>
                                        <option value="ollama" selected=move || provider.get() == AgentLlmProvider::Ollama>"Ollama (Local)"</option>
                                        <option value="azureopenai" selected=move || provider.get() == AgentLlmProvider::AzureOpenAI>"Azure OpenAI"</option>
                                    </select>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        "Model"
                                        {move || if models_loading.get() {
                                            view! { <span class="text-blue-500 text-xs ml-2">"Loading..."</span> }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </label>
                                    <select
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                        on:change=move |ev| set_model.set(event_target_value(&ev))
                                        disabled=models_loading
                                    >
                                        {move || {
                                            let current_model = model.get();
                                            let models = available_models.get();
                                            if models.is_empty() {
                                                vec![view! {
                                                    <option value=current_model.clone() selected=true>{current_model.clone()}</option>
                                                }]
                                            } else {
                                                models.into_iter().map(|m| {
                                                    let is_selected = current_model == m.id;
                                                    let label = if let Some(desc) = m.description {
                                                        format!("{} - {}", m.name, desc)
                                                    } else {
                                                        m.name.clone()
                                                    };
                                                    view! {
                                                        <option value=m.id selected=is_selected>{label}</option>
                                                    }
                                                }).collect::<Vec<_>>()
                                            }
                                        }}
                                    </select>
                                    {move || models_error.get().map(|e| view! {
                                        <p class="text-red-500 text-xs mt-1">{e}</p>
                                    })}
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        "API Key Env Variable"
                                        {move || if provider.get() == AgentLlmProvider::Ollama {
                                            view! { <span class="text-gray-400 text-xs ml-1">"(optional for local)"</span> }.into_any()
                                        } else {
                                            view! { <span class="text-amber-600 dark:text-amber-400 text-xs ml-1">"(required)"</span> }.into_any()
                                        }}
                                    </label>
                                    <input
                                        type="text"
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                        placeholder=move || get_default_api_key_env(&provider.get())
                                        prop:value=move || api_key_env.get()
                                        on:input=move |ev| set_api_key_env.set(event_target_value(&ev))
                                    />
                                    // Warning when API key is required but not set
                                    {move || {
                                        let prov = provider.get();
                                        let key = api_key_env.get();
                                        if provider_requires_api_key(&prov) && key.is_empty() {
                                            let default_var = get_default_api_key_env(&prov);
                                            view! {
                                                <div class="mt-1.5 p-2 bg-amber-50 dark:bg-amber-900/30 rounded border border-amber-200 dark:border-amber-700">
                                                    <div class="flex items-start text-xs">
                                                        <svg class="w-4 h-4 text-amber-500 dark:text-amber-400 mr-1.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                                        </svg>
                                                        <span class="text-amber-700 dark:text-amber-300">
                                                            "Using default: "
                                                            <code class="bg-amber-100 dark:bg-amber-800 px-1 rounded">{default_var}</code>
                                                            ". Ensure this environment variable is set on the server."
                                                        </span>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }
                                    }}
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        "Base URL"
                                        {move || if provider.get() == AgentLlmProvider::Ollama || provider.get() == AgentLlmProvider::AzureOpenAI {
                                            view! { <span class="text-gray-400 text-xs ml-1">"(required)"</span> }.into_any()
                                        } else {
                                            view! { <span class="text-gray-400 text-xs ml-1">"(optional)"</span> }.into_any()
                                        }}
                                    </label>
                                    <input
                                        type="text"
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                                        placeholder=move || match provider.get() {
                                            AgentLlmProvider::Ollama => "http://localhost:11434",
                                            AgentLlmProvider::AzureOpenAI => "https://your-resource.openai.azure.com",
                                            _ => "Leave empty for default",
                                        }
                                        prop:value=move || base_url.get()
                                        on:input=move |ev| set_base_url.set(event_target_value(&ev))
                                    />
                                </div>
                            </div>
                        </div>

                        // System Prompt
                        <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"System Prompt"</h3>
                            <textarea
                                rows="6"
                                class="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white font-mono text-sm"
                                placeholder="You are a helpful assistant..."
                                prop:value=move || system_prompt.get()
                                on:input=move |ev| set_system_prompt.set(event_target_value(&ev))
                            />
                        </div>

                        // Schema Configuration
                        <div class="border-t border-gray-200 dark:border-gray-700 pt-6">
                            <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-4">"Schema Configuration"</h3>
                            <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                "Define JSON schemas for structured input/output when agent is exposed as a tool. "
                                "Custom input fields are serialized to the agent's prompt. Output schema enables JSON parsing of responses."
                            </p>
                            <div class="space-y-4">
                                <JsonSchemaEditor
                                    properties=input_schema_properties
                                    set_properties=set_input_schema_properties
                                    label="Input Schema (optional)"
                                    color="green"
                                />
                                <SchemaPreview properties=input_schema_properties />

                                // Prompt Template (shown when input schema has properties)
                                {move || {
                                    let props = input_schema_properties.get();
                                    if !props.is_empty() {
                                        let prop_names: Vec<String> = props.iter().map(|p| p.name.clone()).collect();
                                        view! {
                                            <div class="mt-4 p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg border border-purple-200 dark:border-purple-800">
                                                <label class="block text-sm font-medium text-purple-800 dark:text-purple-300 mb-2">
                                                    "Prompt Template"
                                                </label>
                                                <p class="text-xs text-purple-600 dark:text-purple-400 mb-2">
                                                    "Define how input values are formatted into the user prompt. Use "
                                                    <code class="bg-purple-100 dark:bg-purple-800 px-1 rounded">"{{field_name}}"</code>
                                                    " for variable substitution."
                                                </p>
                                                <div class="flex flex-wrap gap-1 mb-2">
                                                    {prop_names.iter().map(|name| {
                                                        let var = format!("{{{{{}}}}}", name);
                                                        view! {
                                                            <span class="inline-flex items-center px-2 py-0.5 text-xs font-mono bg-purple-100 dark:bg-purple-800 text-purple-700 dark:text-purple-300 rounded">
                                                                {var}
                                                            </span>
                                                        }
                                                    }).collect_view()}
                                                </div>
                                                <textarea
                                                    rows="3"
                                                    class="w-full px-3 py-2 border border-purple-300 dark:border-purple-600 rounded-md shadow-sm focus:ring-purple-500 focus:border-purple-500 dark:bg-gray-800 dark:text-white font-mono text-sm"
                                                    placeholder="Example: Analyze the topic '{{topic}}' for a {{audience}} audience with focus on {{focus_area}}."
                                                    prop:value=move || prompt_template.get()
                                                    on:input=move |ev| set_prompt_template.set(event_target_value(&ev))
                                                />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}

                                <JsonSchemaEditor
                                    properties=output_schema_properties
                                    set_properties=set_output_schema_properties
                                    label="Output Schema (optional)"
                                    color="blue"
                                />
                                <SchemaPreview properties=output_schema_properties />
                            </div>
                        </div>

                        // Save button
                        <div class="border-t border-gray-200 dark:border-gray-700 pt-6 flex justify-end space-x-4">
                            <a
                                href="/agents"
                                class="px-4 py-2 text-gray-700 bg-gray-200 rounded-md hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-300"
                            >
                                "Cancel"
                            </a>
                            <button
                                class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
                                on:click=on_save
                                disabled=saving
                            >
                                {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                            </button>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}
