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

The script calculates nesting depth by:

1. Tracking opening and closing braces: `{[()]}` 
2. Maintaining a current depth counter
3. Recording the maximum depth reached
4. Properly handling:
   - String literals (ignores braces inside strings)
   - Character escapes (`\"`, `\\`, etc.)
   - Line comments (`//` - code after is ignored)

This provides a realistic measure of code complexity based on actual nesting level
rather than indentation, which can be misleading in some Rust code patterns.
