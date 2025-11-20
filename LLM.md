# LLM Integration Guide

## Overview

Metis now supports LLM (Large Language Model) integration for AI-powered mock responses. This allows you to use OpenAI GPT models or Anthropic Claude models to generate dynamic, intelligent responses.

## Supported Providers

- **OpenAI** - GPT-3.5, GPT-4, and other models
- **Anthropic** - Claude 3 models (Opus, Sonnet, Haiku)

## Configuration

### Basic Setup

```toml
[[tools]]
name = "ai_assistant"
description = "AI-powered assistant"
[tools.input_schema]
type = "object"
properties = { prompt = { type = "string" } }

[tools.mock]
strategy = "llm"
[tools.mock.llm]
provider = "openai"
model = "gpt-3.5-turbo"
temperature = 0.7
max_tokens = 1000
api_key_env = "OPENAI_API_KEY"
```

### Configuration Options

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `provider` | string | Yes | "openai" or "anthropic" |
| `model` | string | Yes | Model name (e.g., "gpt-4", "claude-3-sonnet-20240229") |
| `api_key_env` | string | Yes | Environment variable containing API key |
| `system_prompt` | string | No | System message to set context |
| `temperature` | float | No | Randomness (0.0-2.0, default: 0.7) |
| `max_tokens` | integer | No | Maximum response length |
| `stream` | boolean | No | Enable streaming (not yet implemented) |

## Environment Variables

Set your API keys as environment variables:

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
```

## Examples

### Code Reviewer

```toml
[[tools]]
name = "code_reviewer"
description = "AI code review"
[tools.input_schema]
type = "object"
properties = { code = { type = "string" }, language = { type = "string" } }

[tools.mock]
strategy = "llm"
[tools.mock.llm]
provider = "openai"
model = "gpt-4"
system_prompt = "You are an expert code reviewer. Provide constructive feedback."
temperature = 0.3
max_tokens = 2000
api_key_env = "OPENAI_API_KEY"
```

### Text Summarizer

```toml
[[tools]]
name = "summarizer"
description = "Summarize text"
[tools.input_schema]
type = "object"
properties = { text = { type = "string" } }

[tools.mock]
strategy = "llm"
[tools.mock.llm]
provider = "anthropic"
model = "claude-3-sonnet-20240229"
system_prompt = "Summarize the following text concisely."
temperature = 0.5
max_tokens = 500
api_key_env = "ANTHROPIC_API_KEY"
```

## Usage

### Start Server

```bash
export OPENAI_API_KEY="your-key"
cargo run -- --config examples/llm.toml
```

### Call LLM Tool

```bash
curl -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "ai_assistant",
      "arguments": {
        "prompt": "Explain quantum computing in simple terms"
      }
    },
    "id": 1
  }'
```

## Model Options

### OpenAI Models

| Model | Description | Cost (per 1K tokens) |
|-------|-------------|---------------------|
| gpt-4 | Most capable | $0.03 |
| gpt-4-turbo | Fast GPT-4 | $0.01 |
| gpt-3.5-turbo | Fast & cheap | $0.002 |

### Anthropic Models

| Model | Description | Cost (per 1K tokens) |
|-------|-------------|---------------------|
| claude-3-opus-20240229 | Most capable | $0.015 |
| claude-3-sonnet-20240229 | Balanced | $0.003 |
| claude-3-haiku-20240307 | Fastest | $0.00025 |

## Best Practices

### 1. Use Appropriate Models
- **GPT-4** - Complex reasoning, code review
- **GPT-3.5-turbo** - General chat, simple tasks
- **Claude Opus** - Long context, analysis
- **Claude Haiku** - Speed-critical applications

### 2. Optimize Temperature
- **0.0-0.3** - Deterministic, factual responses
- **0.4-0.7** - Balanced creativity
- **0.8-2.0** - Creative, varied responses

### 3. Set Token Limits
- Prevent excessive costs
- Typical limits: 500-2000 tokens
- Monitor usage via metrics

### 4. Use System Prompts
- Set context and behavior
- Define output format
- Specify constraints

## Error Handling

### Common Errors

**API Key Not Set**
```
Error: API key environment variable OPENAI_API_KEY not set
```
Solution: Set the environment variable

**Rate Limit**
```
Error: OpenAI API error: Rate limit exceeded
```
Solution: Implement backoff/retry or reduce request rate

**Invalid Model**
```
Error: Model 'gpt-5' not found
```
Solution: Use valid model name

## Performance

- **Latency**: 1-5 seconds per request
- **Throughput**: Limited by API rate limits
- **Caching**: Not yet implemented (future feature)

## Limitations

- No streaming support yet
- No token counting/cost tracking yet
- No response caching
- Requires internet connection
- Subject to provider rate limits

## Future Enhancements

- [ ] Streaming responses
- [ ] Token counting
- [ ] Cost tracking
- [ ] Response caching
- [ ] Retry logic with backoff
- [ ] More providers (Cohere, AI21, etc.)
- [ ] Function calling support
- [ ] Vision API support

## Security

- API keys stored in environment variables
- Never commit API keys to version control
- Use separate keys for dev/prod
- Monitor API usage and costs

## Troubleshooting

### Test API Connection

```bash
# Test OpenAI
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Test Anthropic
curl https://api.anthropic.com/v1/messages \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  -H "anthropic-version: 2023-06-01"
```

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run -- --config examples/llm.toml
```

---

**Ready to use AI-powered mocking!** 🤖
