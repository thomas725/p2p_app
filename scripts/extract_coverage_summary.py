#!/usr/bin/env python3
"""Extract a human-readable coverage summary from tarpaulin-report.json."""
import json

with open('tarpaulin-report.json') as f:
    data = json.load(f)

# Compute common path prefix from the report entries
files = data['files']
prefix = list(files[0]['path'])
for entry in files[1:]:
    i = 0
    while i < len(prefix) and i < len(entry['path']) and prefix[i] == entry['path'][i]:
        i += 1
    prefix = prefix[:i]
prefix_len = len(prefix)

print(f'Overall coverage: {data["coverage"]:.2f}%')
print(f'Coverable lines: {data["coverable"]}')
print(f'Covered lines: {data["covered"]}')
print()

for entry in files:
    rel = '/'.join(entry['path'][prefix_len:])
    if entry['coverable'] > 0:
        pct = entry['covered'] / entry['coverable'] * 100
        print(f'{rel}: {pct:.1f}% ({entry["covered"]}/{entry["coverable"]})')
