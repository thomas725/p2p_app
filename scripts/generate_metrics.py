#!/usr/bin/env python3
"""
Generate codebase metrics table by analyzing Rust, Dart, and Kotlin source files.
Counts lines, characters, determines maximum nesting depth.
With --with-coverage, uses cargo-tarpaulin for Rust, LCOV for Dart, JaCoCo XML for Kotlin.
"""

import argparse
import json
import os
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Dict, List, Optional, Tuple

DART_APP_DIR = 'apps/flutter_app'
DART_LIB_DIR = 'apps/flutter_app/lib'
DART_TEST_DIR = 'apps/flutter_app/test'
KOTLIN_SRC_DIR = 'apps/flutter_app/android/app/src/main/kotlin'
KOTLIN_TEST_DIR = 'apps/flutter_app/android/app/src/test/kotlin'

CoverageData = Tuple[Dict[str, Tuple[int, int]], Tuple[int, int]]


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


def calculate_max_nesting(filepath: str, spaces_per_tab: int = 4) -> int:
    max_nesting = 0
    try:
        with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                stripped = line.lstrip()
                if not stripped or stripped.startswith('//'):
                    continue
                leading_ws = len(line) - len(stripped)
                tabs_in_leading = line[:leading_ws].count('\t')
                spaces_in_leading = line[:leading_ws].count(' ')
                total_spaces = (tabs_in_leading * spaces_per_tab) + spaces_in_leading
                nesting_depth = total_spaces // spaces_per_tab
                max_nesting = max(max_nesting, nesting_depth)
    except Exception:
        pass
    return max_nesting


def get_coverage_str(coverage: Optional[float]) -> str:
    if coverage is None:
        return "      -"
    return f"{coverage:>6.2f}%"


# ─── Rust ────────────────────────────────────────────────────────────────────


def load_tarpaulin_coverage(report_path: str) -> CoverageData:
    with open(report_path) as f:
        data = json.load(f)
    files = data['files']
    totals = (data['covered'], data['coverable'])
    prefix = list(files[0]['path'])
    for entry in files[1:]:
        i = 0
        while i < len(prefix) and i < len(entry['path']) and prefix[i] == entry['path'][i]:
            i += 1
        prefix = prefix[:i]
    prefix_len = len(prefix)
    coverage = {}
    for entry in files:
        rel_path = '/'.join(entry['path'][prefix_len:])
        coverage[rel_path] = (entry['covered'], entry['coverable'])
    return coverage, totals


def run_tarpaulin(force: bool = False) -> CoverageData:
    report_path = 'tarpaulin-report.json'
    if force:
        Path(report_path).unlink(missing_ok=True)
    if Path(report_path).exists():
        print("Using existing tarpaulin-report.json", file=sys.stderr)
        return load_tarpaulin_coverage(report_path)
    print("Running cargo tarpaulin --all-features -o Json ...", file=sys.stderr)
    sys.stderr.flush()
    target_dir = str(Path.cwd() / 'target' / 'tarpaulin')
    tmp_dir = Path(target_dir) / 'tmp'
    tmp_dir.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, 'TMPDIR': str(tmp_dir)}
    proc = subprocess.Popen(
        ['cargo', 'tarpaulin', '--all-features', '-o', 'Json', '--target-dir', target_dir],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=env,
    )
    try:
        for line in proc.stdout or []:
            print(line, end='', file=sys.stderr)
        proc.wait(timeout=900)
    except subprocess.TimeoutExpired:
        print("tarpaulin timed out after 900 seconds", file=sys.stderr)
        proc.kill()
        Path(report_path).unlink(missing_ok=True)
        return {}, (0, 0)
    if proc.returncode != 0:
        print(f"tarpaulin failed (exit {proc.returncode})", file=sys.stderr)
        Path(report_path).unlink(missing_ok=True)
        return {}, (0, 0)
    if not Path(report_path).exists():
        print(f"tarpaulin did not produce {report_path}", file=sys.stderr)
        return {}, (0, 0)
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
        'dioxus_app.rs': 'Web UI app shell & components (Dioxus)',
        'dioxus_swarm.rs': 'Web UI swarm event handling (Dioxus)',
        'dioxus_styles.rs': 'Web UI CSS styles (Dioxus)',
        'mobile_node.rs': 'Mobile node lifecycle & swarm',
        'mobile_api.rs': 'Mobile FRB API surface',
        'api.rs': 'FRB API surface',
        'frb_generated.rs': 'flutter_rust_bridge codegen',
    }
    return purposes.get(Path(filepath).name, 'Source file')


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


def collect_files(coverage_per_file: Dict[str, Tuple[int, int]]) -> List[Tuple]:
    files_data = []
    if Path('build.rs').exists():
        filepath = 'build.rs'
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        cov = coverage_per_file.get(filepath)
        coverable = cov[1] if cov else None
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, coverable, pct, purpose))
    for rs_file in sorted(Path('src').glob('**/*.rs')):
        filepath = str(rs_file)
        if 'tests' in filepath:
            continue
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath)
        cov = coverage_per_file.get(filepath)
        coverable = cov[1] if cov else None
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_path_for_display(filepath)
        purpose = get_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, coverable, pct, purpose))
    files_data.sort(key=lambda x: (x[0], x[1]))
    return files_data


def collect_test_files() -> List[Tuple]:
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
    unique = []
    for item in test_files:
        key = (item[0], item[1])
        if key not in seen:
            seen.add(key)
            unique.append(item)
    unique.sort(key=lambda x: (x[0], x[1]))
    return unique


# ─── Dart ────────────────────────────────────────────────────────────────────


def get_dart_file_purpose(filepath: str) -> str:
    purposes = {
        'main.dart': 'Flutter app entry point',
        'api.dart': 'FRB API bindings (generated)',
        'frb_generated.dart': 'flutter_rust_bridge codegen',
        'frb_generated.io.dart': 'FRB IO bindings (generated)',
        'frb_generated.web.dart': 'FRB web bindings (generated)',
        'mobile_api.dart': 'Mobile API bindings (generated)',
        'mobile_node.dart': 'Mobile node bindings (generated)',
        'types.dart': 'Shared type definitions (generated)',
        'test_helpers.dart': 'Test utilities & helpers',
        'api_test.dart': 'Dart API layer tests',
        'widget_test.dart': 'Widget smoke tests',
    }
    return purposes.get(Path(filepath).name, 'Dart source file')


def get_dart_test_purpose(filepath: str) -> str:
    purposes = {
        'test_helpers.dart': 'Test utilities & helpers',
        'api_test.dart': 'Dart API layer unit tests',
        'widget_test.dart': 'Widget smoke tests',
    }
    return purposes.get(Path(filepath).name, 'Dart test file')


def normalize_dart_path(filepath: str) -> Tuple[str, str]:
    path = Path(filepath)
    if 'src/rust' in filepath:
        return ('lib/src/rust', path.name)
    elif 'lib/' in filepath:
        return ('lib', path.name)
    elif 'test/' in filepath:
        parts = path.parts
        rel = '/'.join(parts[parts.index('test') + 1:]) if 'test' in parts else path.name
        return (str(Path(rel).parent) if '/' in rel else 'test', path.name)
    else:
        return (str(path.parent), path.name)


def collect_dart_files(dart_coverage: Dict[str, Tuple[int, int]]) -> List[Tuple]:
    files_data = []
    lib_dir = Path(DART_LIB_DIR)
    if not lib_dir.exists():
        return files_data
    for dart_file in sorted(lib_dir.glob('**/*.dart')):
        filepath = str(dart_file)
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath, spaces_per_tab=2)
        cov = dart_coverage.get(filepath)
        coverable = cov[1] if cov else None
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_dart_path(filepath)
        purpose = get_dart_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, coverable, pct, purpose))
    return files_data


def collect_dart_test_files() -> List[Tuple]:
    test_files = []
    test_dir = Path(DART_TEST_DIR)
    if not test_dir.exists():
        return test_files
    for dart_file in sorted(test_dir.glob('**/*.dart')):
        filepath = str(dart_file)
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath, spaces_per_tab=2)
        folder, filename = normalize_dart_path(filepath)
        purpose = get_dart_test_purpose(filepath)
        test_files.append((folder, filename, lines, chars, nesting, purpose))
    return test_files


def parse_dart_lcov(lcov_path: str, prefix: str) -> CoverageData:
    coverage = {}
    current_file = None
    lf = 0
    lh = 0
    with open(lcov_path) as f:
        for line in f:
            line = line.strip()
            if line.startswith('SF:'):
                current_file = line[3:]
            elif line.startswith('LF:'):
                lf = int(line[3:])
            elif line.startswith('LH:'):
                lh = int(line[3:])
            elif line == 'end_of_record' and current_file:
                full_path = str(Path(prefix) / current_file)
                coverage[full_path] = (lh, lf)
                current_file = None
    total_hit = sum(v[0] for v in coverage.values())
    total_found = sum(v[1] for v in coverage.values())
    return coverage, (total_hit, total_found)


def run_dart_coverage() -> CoverageData:
    lcov_path = Path(DART_APP_DIR) / 'coverage' / 'lcov.info'
    if lcov_path.exists():
        print(f"Using existing {lcov_path}", file=sys.stderr)
        return parse_dart_lcov(str(lcov_path), DART_APP_DIR)
    print("Running flutter test --coverage ...", file=sys.stderr)
    sys.stderr.flush()
    proc = subprocess.Popen(
        ['flutter', 'test', '--coverage'],
        cwd=DART_APP_DIR,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    try:
        for line in proc.stdout or []:
            print(line, end='', file=sys.stderr)
        proc.wait(timeout=120)
    except subprocess.TimeoutExpired:
        print("flutter test timed out", file=sys.stderr)
        proc.kill()
        return {}, (0, 0)
    if proc.returncode != 0:
        print(f"flutter test failed (exit {proc.returncode})", file=sys.stderr)
        return {}, (0, 0)
    if not lcov_path.exists():
        print(f"flutter test did not produce {lcov_path}", file=sys.stderr)
        return {}, (0, 0)
    return parse_dart_lcov(str(lcov_path), DART_APP_DIR)


# ─── Kotlin ──────────────────────────────────────────────────────────────────


def get_kotlin_file_purpose(filepath: str) -> str:
    purposes = {
        'MainActivity.kt': 'Flutter activity & method channel bridge',
        'P2pForegroundService.kt': 'Foreground service for background networking',
    }
    return purposes.get(Path(filepath).name, 'Kotlin source file')


def get_kotlin_test_purpose(filepath: str) -> str:
    purposes = {
        'MainActivityTest.kt': 'MainActivity unit tests',
        'P2pForegroundServiceTest.kt': 'ForegroundService unit tests',
    }
    return purposes.get(Path(filepath).name, 'Kotlin test file')


def normalize_kotlin_path(filepath: str, base: str) -> Tuple[str, str]:
    path = Path(filepath)
    try:
        idx = filepath.index(base)
        rel = filepath[idx:]
        parts = Path(rel).parts
        if len(parts) > 2:
            folder = '/'.join(parts[1:-1])
        else:
            folder = str(Path(rel).parent)
        return (folder, path.name)
    except ValueError:
        return (str(path.parent), path.name)


def collect_kotlin_files(kotlin_coverage: Dict[str, Tuple[int, int]]) -> List[Tuple]:
    files_data = []
    src_dir = Path(KOTLIN_SRC_DIR)
    if not src_dir.exists():
        return files_data
    for kt_file in sorted(src_dir.glob('**/*.kt')):
        filepath = str(kt_file)
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath, spaces_per_tab=4)
        cov = kotlin_coverage.get(filepath)
        coverable = cov[1] if cov else None
        pct = (cov[0] / cov[1] * 100) if cov and cov[1] > 0 else None
        folder, filename = normalize_kotlin_path(filepath, 'kotlin/')
        purpose = get_kotlin_file_purpose(filepath)
        files_data.append((folder, filename, filepath, lines, chars, nesting, coverable, pct, purpose))
    return files_data


def collect_kotlin_test_files() -> List[Tuple]:
    test_files = []
    test_dir = Path(KOTLIN_TEST_DIR)
    if not test_dir.exists():
        return test_files
    for kt_file in sorted(test_dir.glob('**/*.kt')):
        filepath = str(kt_file)
        lines = count_lines(filepath)
        chars = count_characters(filepath)
        nesting = calculate_max_nesting(filepath, spaces_per_tab=4)
        folder, filename = normalize_kotlin_path(filepath, 'kotlin/')
        purpose = get_kotlin_test_purpose(filepath)
        test_files.append((folder, filename, lines, chars, nesting, purpose))
    return test_files


def parse_kotlin_jacoco(xml_path: str, src_base: str) -> CoverageData:
    coverage = {}
    try:
        tree = ET.parse(xml_path)
        root = tree.getroot()
    except ET.ParseError as e:
        print(f"Failed to parse JaCoCo XML: {e}", file=sys.stderr)
        return {}, (0, 0)
    for sourcefile in root.iter('sourcefile'):
        name = sourcefile.get('name', '')
        ci = sum(int(line.get('ci', '0')) for line in sourcefile.findall('line'))
        mi = sum(int(line.get('mi', '0')) for line in sourcefile.findall('line'))
        total = ci + mi
        if total == 0:
            continue
        found = False
        for candidate in Path(src_base).rglob(name):
            coverage[str(candidate)] = (ci, total)
            found = True
            break
        if not found:
            coverage[name] = (ci, total)
    total_hit = sum(v[0] for v in coverage.values())
    total_found = sum(v[1] for v in coverage.values())
    return coverage, (total_hit, total_found)


def run_kotlin_coverage() -> CoverageData:
    report_xml = Path(DART_APP_DIR) / 'build' / 'reports' / 'jacoco' / 'testDebugUnitTest' / 'jacocoTestReport.xml'
    if report_xml.exists():
        print(f"Using existing {report_xml}", file=sys.stderr)
        return parse_kotlin_jacoco(str(report_xml), KOTLIN_SRC_DIR)
    print("Running Gradle JaCoCo coverage report ...", file=sys.stderr)
    sys.stderr.flush()
    proc = subprocess.Popen(
        ['./gradlew', ':app:testDebugUnitTestJacocoReport', '--no-daemon'],
        cwd=DART_APP_DIR + '/android',
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
    )
    try:
        for line in proc.stdout or []:
            print(line, end='', file=sys.stderr)
        proc.wait(timeout=300)
    except subprocess.TimeoutExpired:
        print("Gradle JaCoCo timed out", file=sys.stderr)
        proc.kill()
        return {}, (0, 0)
    if proc.returncode != 0:
        print(f"Gradle JaCoCo failed (exit {proc.returncode})", file=sys.stderr)
        return {}, (0, 0)
    if not report_xml.exists():
        print(f"Gradle did not produce {report_xml}", file=sys.stderr)
        return {}, (0, 0)
    return parse_kotlin_jacoco(str(report_xml), KOTLIN_SRC_DIR)


# ─── Table generation ────────────────────────────────────────────────────────


def generate_source_table(files_data: List[Tuple], max_folder: int = 23, max_file: int = 20, max_purpose: int = 35) -> str:
    output = []
    output.append(f'| {"Folder":<{max_folder}} | {"File":<{max_file}} | {"Depth":>5} | {"Chars":>5} | {"Lines":>5} | {"Testable":>8} | {"Covered":>7} | {"Purpose":<{max_purpose}} |')
    output.append(f'|:{("-" * (max_folder - 1))}|:{("-" * (max_file - 1))}|{"-" * 4}:|{"-" * 4}:|{"-" * 4}:|{"-" * 7}:|{"-" * 6}:|{"-" * max_purpose}:|')
    for folder, filename, _, lines, chars, nesting, coverable, pct, purpose in files_data:
        if len(purpose) > max_purpose:
            purpose = purpose[:max_purpose - 1] + '…'
        testable_str = f'{coverable:>8}' if coverable is not None else '        -'
        output.append(f'| {folder:<{max_folder}} | {filename:<{max_file}} | {nesting:>5} | {chars:>5} | {lines:>5} | {testable_str} | {get_coverage_str(pct)} | {purpose:<{max_purpose}} |')
    return '\n'.join(output)


def generate_test_files_table(test_files: List[Tuple], max_folder: int = 6, max_file: int = 30, max_desc: int = 37) -> str:
    if not test_files:
        return "(none)"
    actual_folder = max(max_folder, max((len(f[0]) for f in test_files), default=0))
    actual_file = max(max_file, max((len(f[1]) for f in test_files), default=0))
    actual_desc = max(max_desc, max((len(f[5]) for f in test_files), default=0))
    output = []
    output.append(f'| {"Folder":<{actual_folder}} | {"File":<{actual_file}} | {"Lines":>5} | {"Chars":>5} | {"Depth":>5} | {"Description":<{actual_desc}} |')
    output.append(f'|:{("-" * (actual_folder - 1))}|:{("-" * (actual_file - 1))}|{"-" * 4}:|{"-" * 4}:|{"-" * 4}:|{"-" * actual_desc}:|')
    for folder, filename, lines, chars, nesting, purpose in test_files:
        if len(purpose) > actual_desc:
            purpose = purpose[:actual_desc - 1] + '…'
        output.append(f'| {folder:<{actual_folder}} | {filename:<{actual_file}} | {lines:>5} | {chars:>5} | {nesting:>5} | {purpose:<{actual_desc}} |')
    return '\n'.join(output)


# ─── Main ────────────────────────────────────────────────────────────────────


def print_summary(label: str, total_files: int, total_lines: int, total_chars: int,
                  covered: int, coverable: int):
    avg_lines = total_lines // total_files if total_files > 0 else 0
    avg_chars = total_chars // total_files if total_files > 0 else 0
    W = max(len(str(total_files)), len(f"{total_lines:,}"), len(f"{total_chars:,}"),
            len(str(avg_lines)), len(f"{avg_chars:,}"))
    LW = 24
    print(f"| {'Total ' + label + ' Files':<{LW}} | {str(total_files).rjust(W)} |")
    print(f"| {'Total ' + label + ' Lines':<{LW}} | {f'{total_lines:,}'.rjust(W)} |")
    print(f"| {'Total ' + label + ' Chars':<{LW}} | {f'{total_chars:,}'.rjust(W)} |")
    print(f"| {'Avg Lines/' + label + ' File':<{LW}} | {str(avg_lines).rjust(W)} |")
    print(f"| {'Avg Chars/' + label + ' File':<{LW}} | {f'{avg_chars:,}'.rjust(W)} |")
    if coverable > 0:
        pct = covered / coverable * 100
        print(f"| {'Covered ' + label + ' Lines':<{LW}} | {f'{covered:,} / {coverable:,} ({pct:.0f}%)'.rjust(W)} |")


def main():
    parser = argparse.ArgumentParser(description='Generate codebase metrics')
    parser.add_argument('--with-coverage', action='store_true',
                        help='Include Rust coverage from cargo-tarpaulin')
    parser.add_argument('--force-coverage', action='store_true',
                        help='Force re-running tarpaulin (implies --with-coverage)')
    parser.add_argument('--dart-coverage', action='store_true',
                        help='Include Dart coverage from flutter test --coverage')
    parser.add_argument('--kotlin-coverage', action='store_true',
                        help='Include Kotlin coverage from Gradle JaCoCo')
    args = parser.parse_args()

    rust_cov: CoverageData = ({}, (0, 0))
    if args.with_coverage or args.force_coverage:
        rust_cov = run_tarpaulin(force=args.force_coverage)
    rust_coverage, rust_totals = rust_cov

    dart_cov: CoverageData = ({}, (0, 0))
    if args.dart_coverage:
        dart_cov = run_dart_coverage()
    dart_coverage, dart_totals = dart_cov

    kotlin_cov: CoverageData = ({}, (0, 0))
    if args.kotlin_coverage:
        kotlin_cov = run_kotlin_coverage()
    kotlin_coverage, kotlin_totals = kotlin_cov

    rust_files = collect_files(rust_coverage)
    dart_files = collect_dart_files(dart_coverage)
    kotlin_files = collect_kotlin_files(kotlin_coverage)

    rust_test = collect_test_files()
    dart_test = collect_dart_test_files()
    kotlin_test = collect_kotlin_test_files()

    all_files = rust_files + dart_files + kotlin_files
    total_files = len(all_files)
    total_lines = sum(f[3] for f in all_files)
    total_chars = sum(f[4] for f in all_files)

    print("# Codebase Metrics")
    print()
    print("## Summary")
    print()
    print("| Metric                  | Value   |")
    print("|:------------------------|--------:|")
    print_summary("Rust", len(rust_files), sum(f[3] for f in rust_files), sum(f[4] for f in rust_files), *rust_totals)
    print_summary("Dart", len(dart_files), sum(f[3] for f in dart_files), sum(f[4] for f in dart_files), *dart_totals)
    print_summary("Kotlin", len(kotlin_files), sum(f[3] for f in kotlin_files), sum(f[4] for f in kotlin_files), *kotlin_totals)
    print()
    print(f"**Grand Total:** {total_files} files, {total_lines:,} lines, {total_chars:,} characters")
    print()

    # ── Rust ──
    print("## Rust Source Files")
    print()
    print(generate_source_table(rust_files))
    print()
    r_cov = rust_totals
    if r_cov[1] > 0:
        print(f"**Total:** {len(rust_files)} files, {sum(f[3] for f in rust_files):,} lines, {sum(f[4] for f in rust_files):,} characters ({r_cov[0]}/{r_cov[1]} testable lines covered, {r_cov[0] / r_cov[1] * 100:.0f}%)")
    else:
        print(f"**Total:** {len(rust_files)} files, {sum(f[3] for f in rust_files):,} lines, {sum(f[4] for f in rust_files):,} characters")
    print()

    print("## Rust Test Files")
    print()
    print(generate_test_files_table(rust_test))
    rtl = sum(f[2] for f in rust_test)
    rtc = sum(f[3] for f in rust_test)
    print()
    print(f"**Total:** {len(rust_test)} test files, {rtl:,} lines, {rtc:,} characters")
    print()

    # ── Dart ──
    print("## Dart Source Files")
    print()
    print(generate_source_table(dart_files))
    print()
    d_cov = dart_totals
    if d_cov[1] > 0:
        print(f"**Total:** {len(dart_files)} files, {sum(f[3] for f in dart_files):,} lines, {sum(f[4] for f in dart_files):,} characters ({d_cov[0]}/{d_cov[1]} testable lines covered, {d_cov[0] / d_cov[1] * 100:.0f}%)")
    else:
        print(f"**Total:** {len(dart_files)} files, {sum(f[3] for f in dart_files):,} lines, {sum(f[4] for f in dart_files):,} characters")
    print()

    print("## Dart Test Files")
    print()
    print(generate_test_files_table(dart_test))
    dtl = sum(f[2] for f in dart_test)
    dtc = sum(f[3] for f in dart_test)
    print()
    print(f"**Total:** {len(dart_test)} test files, {dtl:,} lines, {dtc:,} characters")
    print()

    # ── Kotlin ──
    print("## Kotlin Source Files")
    print()
    print(generate_source_table(kotlin_files))
    print()
    k_cov = kotlin_totals
    if k_cov[1] > 0:
        print(f"**Total:** {len(kotlin_files)} files, {sum(f[3] for f in kotlin_files):,} lines, {sum(f[4] for f in kotlin_files):,} characters ({k_cov[0]}/{k_cov[1]} testable lines covered, {k_cov[0] / k_cov[1] * 100:.0f}%)")
    else:
        print(f"**Total:** {len(kotlin_files)} files, {sum(f[3] for f in kotlin_files):,} lines, {sum(f[4] for f in kotlin_files):,} characters")
    print()

    print("## Kotlin Test Files")
    print()
    print(generate_test_files_table(kotlin_test))
    ktl = sum(f[2] for f in kotlin_test)
    ktc = sum(f[3] for f in kotlin_test)
    print()
    print(f"**Total:** {len(kotlin_test)} test files, {ktl:,} lines, {ktc:,} characters")


if __name__ == '__main__':
    main()
