#!/bin/bash
# Simple loop benchmark for testing

count=${1:-1000}

sum=0
for ((i=1; i<=count; i++)); do
    sum=$((sum + i))
done

# Don't print to avoid output in timing
