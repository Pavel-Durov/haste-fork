#!/usr/bin/env python3
"""Prime number calculation benchmark."""

import sys

def is_prime(n):
    """Check if n is prime."""
    if n < 2:
        return False
    for i in range(2, int(n ** 0.5) + 1):
        if n % i == 0:
            return False
    return True

def find_primes(limit):
    """Find all prime numbers up to limit."""
    return [n for n in range(2, limit) if is_prime(n)]

if __name__ == "__main__":
    limit = int(sys.argv[1]) if len(sys.argv) > 1 else 1000
    primes = find_primes(limit)
    # Don't print to avoid output in timing
