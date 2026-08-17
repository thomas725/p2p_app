# Metrics Generation Script

## generate_metrics.py

Automatically generates codebase metrics by analyzing all Rust source files.

### Features

- **Line counting**: Counts total lines in each source file
- **Character counting**: Counts total characters including whitespace
- **Nesting depth calculation**: Determines maximum nesting depth by tracking brace nesting levels
  - Correctly ignores strings and comments
  - Handles escape sequences in strings
  - Counts `{}[]()` brace pairs

### Usage

```bash
python3 scripts/generate_metrics.py
```

### Output

Generates two sections:

1. **Summary**: Statistics about the entire codebase
   - Total files, lines, characters
   - Average metrics per file

2. **All Source Files**: Markdown table with columns:
   - Folder: Directory location (/, src, src/bin, src/generated, src/bin/tui)
   - File: Filename
   - Lines: Total lines of code
   - Characters: Total characters
   - Nesting: Maximum nesting depth
   - Purpose: Brief description of file purpose

### File Scope

The script analyzes:
- `build.rs` (project root)
- All files under `src/` (main library code)
- Recursively includes subdirectories: `src/bin/`, `src/generated/`, etc.

The script excludes:
- Test files (anything under `tests/`)
- Build artifacts (`target/` directory)
- Git metadata (`.git/` directory)

### Updating Documentation

To update the metrics in `docs/codebase_metrics.md`:

1. Run the script: `python3 scripts/generate_metrics.py`
2. Copy the markdown table output
3. Replace the table in the documentation file
4. Update the generated date in the docs

Or manually update the "All Source Files" section as needed.

### Nesting Depth Algorithm

The script calculates nesting depth by counting **maximum indentation level** in the file:

1. For each non-empty, non-comment line, count leading whitespace (spaces and tabs)
2. Convert tabs to 4 spaces for consistent measurement
3. Calculate nesting depth: `depth = leading_spaces / 4`
4. Track and report the maximum depth found

**Why indentation?** This directly measures what developers see in their editor. The 
visual indentation level is the most accurate representation of code nesting complexity
because:

- It counts actual code structure (not just parentheses from type signatures)
- It correlates with cognitive load and readability
- It's what code formatters enforce (in Rust, `cargo fmt` enforces consistent indentation)
- It's language-independent and tool-agnostic

This approach is simpler, more accurate, and more intuitive than counting braces,
since indentation is what humans use to understand nesting depth at a glance.
