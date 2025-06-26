#!/bin/bash

echo "Testing cycle detection..."

echo "1. Testing self-connection prevention"
echo -e "create sine_oscillator osc1\nconnect-by-id osc1:audio_out osc1:frequency_cv\nexit" | ./target/release/orbital-modulator interactive

echo -e "\n2. Testing normal connection"
echo -e "create sine_oscillator osc1\ncreate output out1\nconnect-by-id \$(echo 'create sine_oscillator osc1\nlist' | ./target/release/orbital-modulator interactive 2>/dev/null | grep osc1 | cut -d' ' -f4) audio_out \$(echo 'create output out1\nlist' | ./target/release/orbital-modulator interactive 2>/dev/null | grep out1 | cut -d' ' -f4) audio_in_l\ntree\nexit" | ./target/release/orbital-modulator interactive

echo "Done!"