# AI Evals Examples

This directory contains example files and scripts to demonstrate how to use the AI evaluation library.

## Files

### Test Cases
- **`cases.json`** - Sample evaluation cases covering math, science, programming, geography, and language tasks

### System Prompts
- **`system-prompt.txt`** - Example system prompt that can be loaded with `@examples/system-prompt.txt`

### Scripts
- **`run-anthropic.sh`** - Run evaluations using Anthropic Claude models
- **`run-openai.sh`** - Run evaluations using OpenAI GPT models  
- **`compare-models.sh`** - Compare multiple models against the same test cases

## Setup

1. **Set API Keys**:
   ```bash
   export ANTHROPIC_API_KEY=your_anthropic_key
   export OPENAI_API_KEY=your_openai_key
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

## Usage Examples

### Basic Usage
```bash
# Run with Anthropic Claude
./examples/run-anthropic.sh

# Run with OpenAI GPT
./examples/run-openai.sh

# Compare multiple models
./examples/compare-models.sh
```

### Manual Commands

**Anthropic with file-based system prompt:**
```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "@examples/system-prompt.txt" \
    --threshold 0.7
```

**OpenAI with inline system prompt:**
```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider openai \
    --model gpt-4 \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "You are a helpful assistant" \
    --threshold 0.7
```

**High creativity settings:**
```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --max-tokens 2000 \
    --temperature 0.8 \
    --top-p 0.9 \
    --system "@examples/system-prompt.txt" \
    --threshold 0.6
```

## Parameters

### Required
- `--cases-file`: Path to JSON file with test cases
- `--provider`: "anthropic" or "openai"  
- `--model`: Model name (e.g., "claude-3-5-sonnet-20241022", "gpt-4")

### Optional
- `--max-tokens`: Maximum tokens to generate (default: 1000)
- `--temperature`: Sampling temperature (0.0-1.0)
- `--top-k`: Top-k sampling (Anthropic only)
- `--top-p`: Top-p sampling (0.0-1.0)
- `--system`: System prompt (text or @file)
- `--threshold`: Pass threshold (default: 0.8)
- `--judge-model`: Judge model (default: claude-3-5-sonnet-20241022)

## Creating Custom Test Cases

Create a JSON file with this structure:
```json
[
  {
    "input": "Your question or prompt",
    "expected_output": "Expected answer (optional)",
    "metadata": {
      "category": "classification",
      "difficulty": "easy|medium|hard"
    }
  }
]
```

The `expected_output` can be `null` for open-ended questions where the judge will evaluate based on quality rather than exact matching.