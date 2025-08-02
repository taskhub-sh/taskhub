#!/usr/bin/env python3
"""
Simple progress bar demo using tqdm.
uv run --with tqdm progress_demo.py
"""

import time
from tqdm import tqdm

def main():
    print("Simple Progress Bar Demo")
    print("-" * 30)

    # Basic progress bar
    for i in tqdm(range(100), desc="Processing", unit="items"):
        time.sleep(0.01)  # Simulate work

    print("\nDone!")

if __name__ == "__main__":
    main()
