#!/bin/bash
# Simple sleep benchmark script using perl for fractional seconds support
# Check if argument was provided
if [ -z "$1" ]; then
    echo "Usage: $0 <seconds>"
    exit 1
fi

# Use perl for cross-platform fractional sleep
perl -e "select(undef,undef,undef,${1});"