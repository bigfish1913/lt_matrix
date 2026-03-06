#!/bin/bash
# Generate man pages for distribution

set -e

echo "Generating man pages for ltmatrix..."

# Create man directory
mkdir -p target/man

# Generate man pages using the ltmatrix binary
cargo build --release --bin ltmatrix
./target/release/ltmatrix man --output target/man

echo "Man pages generated successfully in target/man/"
echo ""
echo "To install system-wide:"
echo "  sudo cp target/man/*.1 /usr/local/share/man/man1/"
echo ""
echo "To verify installation:"
echo "  man ltmatrix"
