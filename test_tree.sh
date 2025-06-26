#!/bin/bash

echo "Testing tree command..."

# Create a test setup
echo "1. Creating oscillator and output nodes"
./target/release/orbital-modulator demo &
DEMO_PID=$!

# Wait a bit for setup
sleep 2

# Stop the demo
kill $DEMO_PID 2>/dev/null

echo "2. Testing standalone tree command"
./target/release/orbital-modulator tree

echo "Done!"