#!/bin/bash
set -e

# Script to generate test coverage report using cargo-tarpaulin

echo "Checking for cargo-tarpaulin..."

# Check if cargo-tarpaulin is installed
if ! command -v cargo-tarpaulin &> /dev/null; then
    echo "cargo-tarpaulin not found. Installing..."
    cargo install cargo-tarpaulin
else
    echo "cargo-tarpaulin is already installed"
fi

echo ""
echo "Generating coverage report..."

# Generate coverage report
# --out Html: Generate HTML report
# --output-dir: Directory for output files
# --exclude-files: Exclude test files from coverage
cargo tarpaulin \
    --out Html \
    --output-dir ./coverage \
    --exclude-files 'tests/*' \
    --exclude-files 'examples/*'

echo ""
echo "Coverage report generated!"
echo "Open ./coverage/tarpaulin-report.html in your browser to view the report."
echo ""
echo "Coverage summary:"
cargo tarpaulin --out stdout --exclude-files 'tests/*' --exclude-files 'examples/*' | tail -5

