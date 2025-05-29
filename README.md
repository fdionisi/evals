<div align="center">
    <h1>Evals 🧪</h1>
    <p>
        A deadly simple evaluation framework for AI models.
    </p>
    <sub>
        Built with Rust and MCP integration.
    </sub>
</div>

## Abstract

Many **production-ready** evaluation frameworks exist; **this is not one of them**. Behind this project lies pure exploration of what makes AI model evaluation simple and effective. The focus is on delivering a fast, no-nonsense evaluation tool that integrates seamlessly with Model Context Protocol (MCP) servers, allowing models to use external tools during testing.

The framework uses AI-as-a-judge methodology with configurable scoring, supports multiple providers (Anthropic, OpenAI), and generates structured reports. It's designed to be embedded in CI pipelines or used standalone for model comparison and quality assessment.

As with many of my Rust projects, this is also an opportunity to practice clean architecture and async patterns whilst building something genuinely useful.

## Quick start

Install Rust, clone the repository, and set your API keys:

```bash
git clone <repository>
cd evals
export ANTHROPIC_API_KEY=your_key
cargo run -- run --cases-file examples/cases.json --provider anthropic --model claude-3-5-sonnet-20241022
```

## Core features

At its heart, the framework provides **multi-provider support** that lets you test both Anthropic and OpenAI foundation models through consistent interfaces, removing the friction of switching between different API formats. The evaluation process relies on **AI judge methodology**, where Claude automatically scores responses against your test cases with configurable pass thresholds, eliminating the need for manual assessment.

Test cases themselves are designed to be **flexible** - you can define exact string matches for factual questions, describe expected behaviors for complex interactions, or leave evaluations completely open-ended for quality assessment. It seamlessly integrate with **MCP** servers, connecting external tools like web search capabilities, enabling you to test tool usage alongside pure reasoning.

Everything flows into **structured reports** that generate detailed JSON output with comprehensive statistics and category breakdowns, while **file-based configuration** keeps your system prompts and settings organized and version-controlled.

## Usage

### Basic evaluation

```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --system "You are a helpful assistant" \
    --threshold 0.8
```

### With MCP tools

```bash
cargo run -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --mcp-servers mcp-config.json \
    --output evaluation-report.json
```

### Parameters

**Required:**

- `--cases-file`: JSON file containing test cases
- `--provider`: "anthropic" or "openai"
- `--model`: Model identifier

**Optional:**

- `--max-tokens`: Generation limit (default: 1000)
- `--temperature`: Sampling temperature
- `--top-k`, `--top-p`: Sampling parameters
- `--system`: System prompt or `@filename.txt`
- `--threshold`: Pass threshold (default: 0.8)
- `--judge-model`: Judge model (default: claude-3-5-sonnet-20241022)
- `--output`: Report output path
- `--mcp-servers`: MCP configuration file

## Test cases format

Create evaluation cases in JSON:

```json
[
  {
    "input": "What is 2 + 2?",
    "expected_output": "4",
    "metadata": { "category": "math", "difficulty": "easy" }
  },
  {
    "input": "Search for recent AI developments",
    "expected_output": {
      "type": "behavior",
      "description": "Uses search tools to find current information"
    },
    "metadata": { "category": "tool_use" }
  }
]
```

**Expected output types:**

- **String**: Exact content matching
- **null**: Open-ended quality evaluation
- **Object**: Flexible comparison or behaviour matching

## MCP integration

Configure external tools via MCP servers:

```json
{
  "servers": [
    {
      "name": "web_search",
      "type": "local",
      "command": ["search-mcp-server"],
      "args": [],
      "env": {}
    }
  ]
}
```

Models can then use these tools during evaluation, enabling testing of tool usage capabilities alongside general knowledge.

## Examples

See `examples/` directory for ready-to-run scripts and sample configurations:

```bash
./examples/run-anthropic.sh     # Basic Anthropic evaluation
./examples/run-openai.sh        # OpenAI model testing
./examples/compare-models.sh    # Multi-model comparison
```

## License

_Evals_ is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for details.
