#!/usr/bin/env python3
"""
Generate codebase metrics table by analyzing all Rust source files.
Counts lines, characters, determines maximum nesting depth.
With --with-coverage, uses cargo-tarpaulin JSON output for real coverage data.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple


def count_lines(filepath: str) -> int:
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(1 for _ in f)
    except Exception:
        return 0


def count_characters(filepath: str) -> int:
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            return sum(len(line) for line in f)
    except Exception:
        return 0


def calculate_max_nesting(filepath: str) -> int:
    max_nesting = 0
    SPACES_PER_TAB = 4

    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                stripped = line.lstrip()
                if not stripped or stripped.startswith('//'):
                    continue

                leading_ws = len(line) - len(stripped)
                tabs_in_leading = line[:leading_ws].count('\t')
                spaces_in_leading = line[:leading_ws].count(' ')
                total_spaces = (tabs_in_leading * SPACES_PER_TAB) + spaces_in_leading
                nesting_depth = total_spaces // 4

                max_nesting = max(max_nesting, nesting_depth)
    except Exception:
        pass

    return max_nesting


def load_tarpaulin_coverage(report_path: str) -> Dict[str, Tuple[int, int]]:
    """
    Load coverage data from cargo-tarpaulin JSON report.
    Returns dict mapping relative source path -> (covered_lines, coverable_lines).
    """
    with open(report_path) as f:
        data = json.load(f)

    files = data['files']

    # Compute the common path prefix from the report entries
    prefix = list(files[0]['path'])
    for entry in files[1:]:
        i = 0
        while i < len(prefix) and i < len(entry['path']) and prefix[i] == entry['path'][i]:
            i += 1
        prefix = prefix[:i]
    prefix_len = len(prefix)

    coverage = {}
    for entry in files:
        covered = entry['covered']
        coverable = entry['coverable']
        rel_path = '/'.join(entry['path'][prefix_len:])
        coverage[rel_path] = (covered, coverable)

    return coverage


def run_tarpaulin(force: bool = False) -> Dict[str, Tuple[int, int]]:
    """Run cargo tarpaulin with --all-features and return coverage data."""
    report_path = 'tarpaulin-report.json'

    if force:
        Path(report_path).unlink(missing_ok=True)

    if Path(report_path).exists():
        print("Using existing tarpaulin-report.json", file=sys.stderr)
        return load_tarpaulin_coverage(report_path)

    print("Running cargo tarpaulin --all-features -o Json ...", file=sys.stderr)
    sys.stderr.flush()

    try:
        result = subprocess.run(
            ['cargo', 'tarpaulin', '--all-features', '-o', 'Json'],
            timeout=600,
        )
    except subprocess.TimeoutExpired:
        print("tarpaulin timed out after 600 seconds", file=sys.stderr)
        return {}

    if result.returncode != 0:
        print(f"tarpaulin failed (exit {result.returncode})", file=sys.stderr)
        return {}

    if not Path(report_path).exists():
        print(f"tarpaulin did not produce {report_path}", file=sys.stderr)
        return {}

    return load_tarpaulin_coverage(report_path)


def get_file_purpose(filepath: str) -> str:
    if 'render_loop/mod.rs' in filepath:
        return 'Render loop orchestration (60 FPS)'
    if 'render_loop/visibility.rs' in filepath:
        return 'Message visibility calculations'
    if 'render_loop/layout.rs' in filepath:
        return 'UI layout component rendering'
    if 'render_loop/tab_renderers.rs' in filepath:
        return 'Tab-specific renderers'

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
        'tui_tabs.rs': 'Tab management & navigation',
        'tui_test_state.rs': 'TUI test state & mouse handling',
        'tui_events.rs': 'Event/command types & channels',
        'columns.rs': 'Auto-generated column definitions',
        'schema.rs': 'Database schema (Diesel)',
        'models_insertable.rs': 'Insertable data models',
        'models_queryable.rs': 'Queryable data models',
        'p2p_chat_tui.rs': 'Main TUI application entry point',
        'p2p_chat.rs': 'CLI chat application',
        'p2p_chat_dioxus.rs': 'Web UI (Dioxus framework)',
        'command_processor.rs': 'Event routing & state updates',
        'event_source.rs': 'Terminal event polling (60 FPS)',
        'input_processor.rs': 'Input event routing & processing',
        'scroll_handlers.rs': 'Scroll & hover-aware navigation',
        'click_handlers.rs': 'Click handlers & index mapping',
        'message_handlers.rs': 'Message sending logic',
        'main_loop.rs': 'Task orchestration & async',
        'state.rs': 'Shared application state',
        'constants.rs': 'TUI constants & config',
        'mod.rs': 'Module declarations',
        'tui_helpers.rs': 'TUI helper functions & utilities',
        'tui_render.rs': 'TUI rendering & state management',
        'tui_render_state.rs': 'TUI render state & tab content',
        'presentation.rs': 'TUI presentation & formatting helpers',
    }
    return purposes.get(Path(filepath).name, 'Source file')


def get_coverage_str(coverage: Optional[float]) -> str:
    if coverage is None:
        return "    -"
    elif coverage >= 100:
        return " 100%"
    elif coverage >= 10:
        return f" {coverage:>3.0f}%"
    else:
        return f"{coverage:>4.1f}%"


def get_test_file_purpose(filepath: str) -> str:
    purposes = {
        'fmt.rs': 'fmt module tests',
        'logging.rs': 'logging module tests',
        'messages.rs': 'messages module tests',
        'nickname.rs': 'nickname module tests',
        'peers.rs': 'peers module tests',
        'db.rs': 'database module tests',
        'behavior.rs': 'behavior module tests',
        'network.rs': 'network module tests',
        'types.rs': 'types module tests',
        'tui_helpers.rs': 'TUI helpers tests',
        'tui_state.rs': 'TUI state tests',
        'tui_events.rs': 'TUI events tests',
        'tui_chat.rs': 'TUI chat functionality tests',
        'tui_integration.rs': 'TUI integration tests',
        'tui_render_integration.rs': 'TUI render integration tests',
        'tui_tasks.rs': 'TUI task tests',
        'tui_binary_integration.rs': 'TUI binary integration tests',
        'additional_coverage.rs': 'Additional coverage tests',
        'p2p_integration.rs': 'P2P integration tests',
        'db_selection.rs': 'Database selection tests',
        'test_utils.rs': 'Test utilities',
        'queryable_tests.rs': 'Diesel queryable model tests',
        'insertable_tests.rs': 'Diesel insertable model tests',
        'swarm_handler.rs': 'swarm_handler module tests',
        'tui_tabs_dedicated.rs': 'Dedicated TUI tabs tests',
        'tui_test_state_dedicated.rs': 'Dedicated TUI test-state tests',
        'unit_behavior.rs': 'Unit tests for behavior module',
        'unit_bin_tui_click_handlers.rs': 'Unit tests for TUI click handlers',
        'unit_bin_tui_command_processor.rs': 'Unit tests for TUI command processor',
        'unit_bin_tui_event_source.rs': 'Unit tests for TUI event source',
        'unit_bin_tui_input_processor.rs': 'Unit tests for TUI input processor',
        'unit_bin_tui_main_loop.rs': 'Unit tests for TUI main loop',
        'unit_bin_tui_message_handlers.rs': 'Unit tests for TUI message handlers',
        'unit_bin_tui_render_loop_mod.rs': 'Unit tests for TUI render loop',
        'unit_bin_tui_scroll_handlers.rs': 'Unit tests for TUI scroll handlers',
        'unit_bin_tui_state.rs': 'Unit tests for TUI state',
        'unit_bin_tui_test_helpers.rs': 'Unit tests for TUI test helpers',
        'unit_db.rs': 'Unit tests for database module',
        'unit_lib.rs': 'Unit tests for library re-exports/api',
        'unit_logging.rs': 'Unit tests for logging module',
        'unit_messages.rs': 'Unit tests for messages module',
        'unit_network.rs': 'Unit tests for network module',
        'unit_nickname.rs': 'Unit tests for nickname module',
        'unit_peers.rs': 'Unit tests for peers module',
        'unit_swarm_handler.rs': 'Unit tests for swarm_handler module',
        'unit_tui_helpers.rs': 'Unit tests for TUI helpers',
        'unit_tui_render_state.rs': 'Unit tests for TUI render state',
        'unit_tui_tabs.rs': 'Unit tests for TUI tabs',
        'unit_tui_test_state.rs': 'Unit tests for TUI test state',
        'unit_types.rs': 'Unit tests for types module',
    }
    return purposes.get(Path(filepath).name, 'Test file')


def normalize_path_for_display(filepath: str) -> Tuple[str, str]:
    path = Path(filepath)

    if path.name == 'build.rs':
        return ('/', 'build.rs')
    elif 'src/bin/tui/render_loop' in filepath:
        return ('src/bin/tui/render_loop', path.name)
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


def collect_files(
    coverage_data: Dict[str, Tuple[int, int]],
) -> List[Tuple[str, str, str, int, int, int, str, Optional[float]]]:
    files_data = []

    if Path('build.rs').exists():
        filepath = 'build.rs'
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        cov = coverage_data.get(filepath)
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, purpose, pct))

    for rs_file in sorted(Path('src').glob('**/*.rs')):
        filepath = str(rs_file)

        if 'tests' in filepath or '#[cfg(test)]' in str(rs_file):
            continue

        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        cov = coverage_data.get(filepath)
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)

        files_data.append((folder, filename, filepath, lines, chars, nesting, purpose, pct))

    files_data.sort(key=lambda x: (x[0], x[1]))
    return files_data


def collect_test_files() -> List[Tuple[str, str, int, int, int, str]]:
    test_files = []

    for pattern in ['tests/*.rs', 'tests/**/*.rs']:
        for test_file in sorted(Path('.').glob(pattern)):
            filepath = str(test_file)
            if not filepath.endswith('.rs'):
                continue

            lines = count_lines(filepath)
            chars = count_characters(filepath)
            nesting = calculate_max_nesting(filepath)

            folder = str(test_file.parent)
            if folder == '.':
                folder = 'tests'
            elif folder.startswith('tests/'):
                folder = folder[6:]

            purpose = get_test_file_purpose(filepath)
            test_files.append((folder, test_file.name, lines, chars, nesting, purpose))

    seen = set()
    unique_tests = []
    for item in test_files:
        key = (item[0], item[1])
        if key not in seen:
            seen.add(key)
            unique_tests.append(item)

    unique_tests.sort(key=lambda x: (x[0], x[1]))
    return unique_tests


def generate_test_files_table(test_files: List[Tuple]) -> str:
    folder_col_width = max(len("Folder"), 6)
    file_col_width = max(
        len("File"),
        max((len(filename) for _, filename, _, _, _, _ in test_files), default=0),
    )
    lines_col_width = max(len("Lines"), 5)
    chars_col_width = max(len("Chars"), 5)
    depth_col_width = max(len("Depth"), 5)
    desc_col_width = max(len("Description"), 37)

    output = []
    output.append(
        f"| {'Folder':<{folder_col_width}} | {'File':<{file_col_width}} | {'Lines':>{lines_col_width}} | {'Chars':>{chars_col_width}} | {'Depth':>{depth_col_width}} | {'Description':<{desc_col_width}} |"
    )
    folder_sep_width = folder_col_width + 2
    file_sep_width = file_col_width + 2
    lines_sep_width = lines_col_width + 2
    chars_sep_width = chars_col_width + 2
    depth_sep_width = depth_col_width + 2
    desc_sep_width = desc_col_width + 2
    output.append(
        f"|:{'-' * (folder_sep_width - 1)}|:{'-' * (file_sep_width - 1)}|{'-' * (lines_sep_width - 1)}:|{'-' * (chars_sep_width - 1)}:|{'-' * (depth_sep_width - 1)}:|{'-' * (desc_sep_width - 1)}:|"
    )

    for folder, filename, lines, chars, nesting, purpose in test_files:
        if len(purpose) > desc_col_width:
            purpose = purpose[:desc_col_width - 1] + '…'

        folder_display = folder if folder else 'tests'
        output.append(
            f"| {folder_display:<{folder_col_width}} | {filename:<{file_col_width}} | {lines:>{lines_col_width}} | {chars:>{chars_col_width}} | {nesting:>{depth_col_width}} | {purpose:<{desc_col_width}} |"
        )

    return '\n'.join(output)


def generate_markdown_table(files_data: List[Tuple]) -> str:
    output = []
    output.append('| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |')
    output.append('|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|')

    for folder, filename, _, lines, chars, nesting, purpose, coverage in files_data:
        if len(purpose) > 35:
            purpose = purpose[:34] + '…'

        output.append(f'| {folder:<23} | {filename:<20} | {lines:>5} | {chars:>5} | {nesting:>5} | {get_coverage_str(coverage)} | {purpose:<35} |')

    return '\n'.join(output)


def main():
    parser = argparse.ArgumentParser(description='Generate codebase metrics')
    parser.add_argument(
        '--with-coverage',
        action='store_true',
        help='Include real code coverage data from cargo-tarpaulin (slow: runs tarpaulin if no cached report)',
    )
    parser.add_argument(
        '--force-coverage',
        action='store_true',
        help='Force re-running tarpaulin even if cached report exists (implies --with-coverage)',
    )
    args = parser.parse_args()

    coverage_data: Dict[str, Tuple[int, int]] = {}
    if args.with_coverage or args.force_coverage:
        coverage_data = run_tarpaulin(force=args.force_coverage)

    files_data = collect_files(coverage_data)

    total_lines = sum(f[3] for f in files_data)
    total_chars = sum(f[4] for f in files_data)
    total_files = len(files_data)
    avg_lines = total_lines // total_files if total_files > 0 else 0
    avg_chars = total_chars // total_files if total_files > 0 else 0

    W = max(
        len(str(total_files)),
        len(f"{total_lines:,}"),
        len(f"{total_chars:,}"),
        len(str(avg_lines)),
        len(f"{avg_chars:,}")
    )

    v1 = str(total_files).rjust(W)
    v2 = f"{total_lines:,}".rjust(W)
    v3 = f"{total_chars:,}".rjust(W)
    v4 = str(avg_lines).rjust(W)
    v5 = f"{avg_chars:,}".rjust(W)

    print("# Codebase Metrics")
    print()
    print("## Summary")
    print()
    print("| Metric                  | Value   |")
    print("|:------------------------|--------:|")
    print(f"| Total Rust Files        | {v1} |")
    print(f"| Total Lines of Code     | {v2} |")
    print(f"| Total Characters        | {v3} |")
    print(f"| Average Lines per File  | {v4} |")
    print(f"| Average Characters/File | {v5} |")
    print()
    print("## All Source Files")
    print()
    table = generate_markdown_table(files_data)
    print(table)
    print()
    print(f"**Total:** {total_files} files, {total_lines:,} lines, {total_chars:,} characters")
    print()

    test_files = collect_test_files()
    test_total_lines = sum(f[2] for f in test_files)
    test_total_chars = sum(f[3] for f in test_files)
    test_total_files = len(test_files)

    print("## Test Files")
    print()
    test_table = generate_test_files_table(test_files)
    print(test_table)
    print()
    print(f"**Total:** {test_total_files} test files, {test_total_lines:,} lines, {test_total_chars:,} characters")


if __name__ == '__main__':
    main()
