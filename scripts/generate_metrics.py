#!/usr/bin/env python3
"""
Generate codebase metrics table by analyzing all Rust source files.
Counts lines, characters, and determines maximum nesting depth for each file.
"""

import os
import re
from pathlib import Path
from typing import List, Tuple

def count_lines(filepath: str) -> int:
    """Count the number of lines in a file."""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(1 for _ in f)
    except Exception:
        return 0

def count_characters(filepath: str) -> int:
    """Count the number of characters in a file."""
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(len(line) for line in f)
    except Exception:
        return 0

def calculate_max_nesting(filepath: str) -> int:
    """
    Calculate the maximum nesting depth in a file by counting indentation levels.

    Measures the maximum leading whitespace indentation across all non-empty lines.
    This directly reflects the visual nesting shown in the code editor.

    Converts tabs to 4 spaces for consistent measurement.
    Ignores blank lines and comment-only lines.
    """
    max_nesting = 0
    SPACES_PER_TAB = 4

    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                # Skip empty lines and line-comment-only lines
                stripped = line.lstrip()
                if not stripped or stripped.startswith('//'):
                    continue

                # Count leading whitespace
                leading_ws = len(line) - len(stripped)

                # Convert tabs to spaces
                tabs_in_leading = line[:leading_ws].count('\t')
                spaces_in_leading = line[:leading_ws].count(' ')
                total_spaces = (tabs_in_leading * SPACES_PER_TAB) + spaces_in_leading

                # Calculate nesting depth (assuming 4-space indentation)
                nesting_depth = total_spaces // 4

                max_nesting = max(max_nesting, nesting_depth)
    except Exception:
        pass

    return max_nesting

def get_file_purpose(filepath: str) -> str:
    """Get a brief description of file purpose from first doc comment or context."""
    purposes = {
        'build.rs': 'Build script',
        'lib.rs': 'Module declarations & re-exports',
        'db.rs': 'Database connection & identity mgmt',
        'logging.rs': 'Logging utilities & setup',
        'swarm_handler.rs': 'Network event translation',
        'messages.rs': 'Message persistence & retrieval',
        'peers.rs': 'Peer management & tracking',
        'nickname.rs': 'Nickname management',
        'fmt.rs': 'Formatting & display utilities',
        'behavior.rs': 'Network behavior definitions',
        'network.rs': 'Network size classification',
        'types.rs': 'Event & command type defs',
        'logging_config.rs': 'Tracing configuration',
        'tui_tabs.rs': 'Tab management & navigation',
        'tui_test_state.rs': 'TUI test state & mouse handling',
        'tui_events.rs': 'Event/command types & channels',
        'mod.rs': 'Module declarations',
        'columns.rs': 'Auto-generated column definitions',
        'schema.rs': 'Database schema (Diesel)',
        'models_insertable.rs': 'Insertable data models',
        'models_queryable.rs': 'Queryable data models',
        'p2p_chat_tui.rs': 'Main TUI application entry point',
        'p2p_chat.rs': 'CLI chat application',
        'p2p_chat_dioxus.rs': 'Web UI (Dioxus framework)',
        'command_processor.rs': 'Event routing & state updates',
        'input_handlers.rs': 'Keyboard & mouse input processing',
        'message_handlers.rs': 'Message sending logic',
        'main_loop.rs': 'Task orchestration & async',
        'render_loop.rs': '60 FPS rendering loop',
        'state.rs': 'Shared application state',
        'input_handler.rs': 'Terminal event polling',
        'tracing_writer.rs': 'Tracing log output handling',
        'constants.rs': 'TUI constants & config',
    }
    return purposes.get(Path(filepath).name, 'Source file')

def normalize_path_for_display(filepath: str) -> Tuple[str, str]:
    """Convert filepath to display folder and filename."""
    path = Path(filepath)

    if path.name == 'build.rs':
        return ('/', 'build.rs')
    elif 'src/bin/tui' in filepath:
        return ('src/bin/tui', path.name)
    elif 'src/bin' in filepath:
        return ('src/bin', path.name)
    elif 'src/generated' in filepath:
        return ('src/generated', path.name)
    elif 'src' in filepath:
        return ('src', path.name)
    else:
        return (str(path.parent), path.name)

def collect_files() -> List[Tuple[str, str, str, int, int, int]]:
    """Collect all Rust files with their metrics (excluding tests)."""
    files_data = []

    # Directories to scan - only src/, src/bin/, src/generated/, and build.rs
    base_path = Path('.')

    # Collect build.rs
    if Path('build.rs').exists():
        filepath = 'build.rs'
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, purpose))

    # Collect src/ files
    for rs_file in sorted(Path('src').glob('**/*.rs')):
        filepath = str(rs_file)

        # Skip test module files
        if 'tests' in filepath or '#[cfg(test)]' in str(rs_file):
            continue

        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)

        files_data.append((folder, filename, filepath, lines, chars, nesting, purpose))

    # Sort by folder, then by filename for consistent ordering
    files_data.sort(key=lambda x: (x[0], x[1]))

    return files_data

def generate_markdown_table(files_data: List[Tuple]) -> str:
    """Generate markdown table from file data."""
    output = []
    output.append('| Folder        | File                 | Lines | Chars | Depth | Purpose                             |')
    output.append('|---------------|---------------------:|------:|------:|------:|------------------------------------:|')

    for folder, filename, _, lines, chars, nesting, purpose in files_data:
        # Truncate purpose if too long
        if len(purpose) > 35:
            purpose = purpose[:32] + '...'

        output.append(f'| {folder:<13} | {filename:<20} | {lines:>5} | {chars:>5} | {nesting:>5} | {purpose:<35} |')

    return '\n'.join(output)

def main():
    """Main entry point."""
    print("Scanning Rust source files...")
    files_data = collect_files()

    # Calculate totals
    total_lines = sum(f[3] for f in files_data)
    total_chars = sum(f[4] for f in files_data)
    total_files = len(files_data)
    avg_lines = total_lines // total_files if total_files > 0 else 0
    avg_chars = total_chars // total_files if total_files > 0 else 0

    print()
    print("=" * 80)
    print("CODEBASE METRICS")
    print("=" * 80)
    print()

    print("## Summary")
    print()
    print("| Metric                      | Count   |")
    print("|:--------------------------|--------:|")
    print(f"| Total Rust Files            | {total_files:>7}|")
    print(f"| Total Lines of Code         | {total_lines:>7,}|")
    print(f"| Total Characters            | {total_chars:>7,}|")
    print(f"| Average Lines per File      | {avg_lines:>7}|")
    print(f"| Average Characters per File | {avg_chars:>7}|")
    print()
    print()

    print("## All Source Files")
    print()
    table = generate_markdown_table(files_data)
    print(table)
    print()
    print(f"**Total:** {total_files} files, {total_lines:,} lines, {total_chars:,} characters")
    print()

    # Print individual file details for verification
    print()
    print("=" * 80)
    print("FILE DETAILS (for verification)")
    print("=" * 80)
    print()
    for folder, filename, filepath, lines, chars, nesting, purpose in files_data:
        print(f"{filepath:40} {lines:>5} lines {chars:>8} chars  nesting={nesting:>2}")

if __name__ == '__main__':
    main()
