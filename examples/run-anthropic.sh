#!/bin/bash

# Example script to run evaluations with Anthropic Claude
# Make sure to set ANTHROPIC_API_KEY environment variable

set -e

echo "Running AI Evaluations with Anthropic Claude..."
echo "=============================================="

# Check if API key is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "Error: ANTHROPIC_API_KEY environment variable not set"
    echo "Please run: export ANTHROPIC_API_KEY=your_api_key"
    exit 1
fi

# Build the project
echo "Building project..."
cargo build --release

echo "Running evaluations..."

# Run with system prompt from file
cargo run --release -- run \
    --cases-file examples/cases.json \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --max-tokens 1000 \
    --temperature 0.7 \
    --system @examples/system-prompt.txt \
    --threshold 0.7 \
    --judge-model claude-3-5-sonnet-20241022 \
    --mcp-servers examples/mcp-servers.json \
    --iterations 2 \
    --output anthropic-evaluation-report.json

echo ""
echo "Evaluation completed!"
