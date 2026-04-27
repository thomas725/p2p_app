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

1. Tracking **only curly braces** `{}` (code block nesting)
2. Ignoring parentheses `()` and square brackets `[]` (these are function parameters, 
   generics, pattern matching - they don't affect code block complexity)
3. Maintaining a current depth counter
4. Recording the maximum depth reached
5. Properly handling:
   - String literals (ignores braces inside strings)
   - Character escapes (`\"`, `\\`, etc.)
   - Line comments (`//` - code after is ignored)

**Why only `{}`?** In Rust, parentheses and brackets are abundant in type signatures,
function calls, and pattern matching, but they don't represent actual code nesting
that increases cognitive complexity. Only curly braces represent control flow structures
like functions, loops, conditionals, and blocks that actually nest.

This provides a realistic measure of code complexity that correlates with actual
readability and maintainability.
