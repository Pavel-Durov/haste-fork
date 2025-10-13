#!/usr/bin/env python3
"""Simple Fibonacci benchmark for testing haste."""

import sys

def fibonacci(n):
    """Calculate nth Fibonacci number recursively."""
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 25
    result = fibonacci(n)
    # Don't print to avoid output in timing
