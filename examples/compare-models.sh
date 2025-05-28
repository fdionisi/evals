#!/bin/bash

# Script to compare multiple models against the same eval cases
# Requires both ANTHROPIC_API_KEY and OPENAI_API_KEY

set -e

echo "Comparing Multiple AI Models..."
echo "==============================="

# Check if API keys are set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "Error: ANTHROPIC_API_KEY environment variable not set"
    exit 1
fi

if [ -z "$OPENAI_API_KEY" ]; then
    echo "Error: OPENAI_API_KEY environment variable not set"
    exit 1
fi

# Build the project
echo "Building project..."
cargo build --release

CASES_FILE="examples/cases.json"
SYSTEM_PROMPT="@examples/system-prompt.txt"
THRESHOLD=0.7

echo ""
echo "Testing Claude 3.5 Sonnet..."
echo "============================"
cargo run --release -- run \
    --cases-file "$CASES_FILE" \
    --provider anthropic \
    --model claude-3-5-sonnet-20241022 \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "$SYSTEM_PROMPT" \
    --threshold "$THRESHOLD" \
    --judge-model claude-3-5-sonnet-20241022

echo ""
echo "Testing GPT-4..."
echo "================"
cargo run --release -- run \
    --cases-file "$CASES_FILE" \
    --provider openai \
    --model gpt-4 \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "$SYSTEM_PROMPT" \
    --threshold "$THRESHOLD" \
    --judge-model claude-3-5-sonnet-20241022

echo ""
echo "Testing GPT-3.5 Turbo..."
echo "========================"
cargo run --release -- run \
    --cases-file "$CASES_FILE" \
    --provider openai \
    --model gpt-3.5-turbo \
    --max-tokens 1000 \
    --temperature 0.3 \
    --system "$SYSTEM_PROMPT" \
    --threshold "$THRESHOLD" \
    --judge-model claude-3-5-sonnet-20241022

echo ""
echo "All model comparisons completed!"