#!/bin/bash

# Example script to run evaluations with OpenAI GPT
# Make sure to set OPENAI_API_KEY environment variable

set -e

echo "Running AI Evaluations with OpenAI GPT..."
echo "=========================================="

# Check if API key is set
if [ -z "$OPENAI_API_KEY" ]; then
    echo "Error: OPENAI_API_KEY environment variable not set"
    echo "Please run: export OPENAI_API_KEY=your_api_key"
    exit 1
fi

# Build the project
echo "Building project..."
cargo build --release

echo "Running evaluations..."

# Run with inline system prompt
cargo run --release -- run \
    --cases-file examples/cases.json \
    --provider openai \
    --model gpt-4 \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "You are a helpful and accurate assistant. Provide clear, concise answers." \
    --threshold 0.7 \
    --judge-model claude-3-5-sonnet-20241022 \
    --mcp-servers examples/mcp-servers.json \
    --output openai-evaluation-report.json

echo ""
echo "Evaluation completed!"
