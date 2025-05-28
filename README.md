# AI Evals

A simple, fast AI evaluation library in Rust for testing language models with AI-as-a-judge scoring.

## Features

- **Multi-provider support**: Anthropic and OpenAI models
- **AI judge evaluation**: Automated scoring using Claude as judge
- **Flexible configuration**: All parameters via CLI
- **File-based system prompts**: Load prompts from files with `@` prefix
- **Async execution**: Built on tokio for performance
- **Clean architecture**: Extensible for new providers

## Quick Start

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and build**:
   ```bash
   git clone <repository>
   cd evals
   cargo build --release
   ```

3. **Set API keys**:
   ```bash
   export ANTHROPIC_API_KEY=your_anthropic_key
   export OPENAI_API_KEY=your_openai_key  # Optional, only for OpenAI models
   ```

4. **Run example**:
   ```bash
   ./examples/run-anthropic.sh
   ```

## Usage

### Basic Command
```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --system "You are a helpful assistant" \
    --threshold 0.8
```

### Parameters

**Required:**
- `--cases-file`: JSON file with test cases
- `--provider`: "anthropic" or "openai"
- `--model`: Model name

**Optional:**
- `--max-tokens`: Max tokens (default: 1000)
- `--temperature`: Sampling temperature
- `--top-k`: Top-k sampling (Anthropic only)
- `--top-p`: Top-p sampling  
- `--system`: System prompt or `@file.txt`
- `--threshold`: Pass threshold (default: 0.8)
- `--judge-model`: Judge model (default: claude-3-5-sonnet-20241022)

### Test Cases Format

Create a JSON file with test cases:
```json
[
  {
    "input": "What is 2 + 2?",
    "expected_output": "4",
    "metadata": {
      "category": "math",
      "difficulty": "easy"
    }
  },
  {
    "input": "Explain photosynthesis",
    "expected_output": null,
    "metadata": {
      "category": "science", 
      "difficulty": "medium"
    }
  }
]
```

## Examples

See the `examples/` directory for:
- Sample test cases (`cases.json`)
- System prompt files (`system-prompt.txt`)
- Ready-to-run scripts (`run-anthropic.sh`, `run-openai.sh`, `compare-models.sh`)
- Detailed documentation (`examples/README.md`)

## Architecture

- **ConversationModel trait**: Abstraction for different AI providers
- **Environment-based auth**: API keys from environment variables
- **Async execution**: Non-blocking evaluation of test cases
- **AI judge scoring**: Uses Claude to score model responses
- **Extensible design**: Easy to add new providers (Ollama, etc.)

## Supported Models

**Anthropic:**
- claude-3-5-sonnet-20241022
- claude-3-5-haiku-20241022
- claude-3-opus-20240229

**OpenAI:**
- gpt-4
- gpt-4-turbo
- gpt-3.5-turbo

## Contributing

This is a simple library focused on core evaluation functionality. To add new providers:

1. Implement the `ConversationModel` trait
2. Add the provider to `create_model()` function
3. Update documentation

## License

MIT