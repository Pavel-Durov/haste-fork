#!/usr/bin/env python3
"""Fibonacci benchmark that outputs multiple measurements for ReBench PlainText adapter."""

import sys
import time

def fibonacci(n):
    """Calculate nth Fibonacci number recursively."""
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 25
    iterations = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    
    for i in range(iterations):
        start = time.perf_counter()
        result = fibonacci(n)
        elapsed_ms = (time.perf_counter() - start) * 1000
        print(f"RESULT: {elapsed_ms:.2f}")
