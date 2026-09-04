# Codebase Metrics

## Summary

| Metric                |               Value |
|:----------------------|:--------------------|
| Total Rust Files      |                  39 |
| Total Rust Lines      |              11,607 |
| Total Rust Chars      |             417,879 |
| Avg Lines/Rust File   |                 297 |
| Avg Chars/Rust File   |              10,714 |
| Covered Rust Lines    | 2,374 / 3,811 (62%) |
| Total Dart Files      |                   8 |
| Total Dart Lines      |               5,047 |
| Total Dart Chars      |             153,959 |
| Avg Lines/Dart File   |                 630 |
| Avg Chars/Dart File   |              19,244 |
| Covered Dart Lines    |    120 / 1,826 (7%) |
| Total Kotlin Files    |                   2 |
| Total Kotlin Lines    |                 204 |
| Total Kotlin Chars    |               6,805 |
| Avg Lines/Kotlin File |                 102 |
| Avg Chars/Kotlin File |               3,402 |

**Grand Total:** 49 files, 16,858 lines, 578,643 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                        |
| src                     | behavior.rs          |     6 |  6685 |   188 |       38 |  94.74% | Network behavior definitions        |
| src                     | connected.rs         |     3 |  2324 |    71 |       18 | 100.00% | Source file                         |
| src                     | db.rs                |     5 | 23496 |   604 |       88 |  92.05% | Database connection & identity mgmt |
| src                     | fmt.rs               |     4 |  3906 |   129 |       43 |  97.67% | Formatting & display utilities      |
| src                     | frb_generated.rs     |     6 | 70223 |  1743 |      767 |   0.00% | flutter_rust_bridge codegen         |
| src                     | lib.rs               |     1 |  5939 |   156 |        1 | 100.00% | Module declarations & re-exports    |
| src                     | logging.rs           |     4 | 11635 |   359 |      109 |  98.17% | Logging utilities & setup           |
| src                     | messages.rs          |     5 | 12171 |   351 |      101 |  90.10% | Message persistence & retrieval     |
| src                     | mobile_api.rs        |     4 | 17585 |   529 |       42 | 100.00% | Mobile FRB API surface              |
| src                     | mobile_node.rs       |     5 | 36038 |  1051 |      195 |  48.72% | Mobile node lifecycle & swarm       |
| src                     | mod.rs               |     1 |  5560 |   168 |       36 |   0.00% | Module declarations                 |
| src                     | network.rs           |     3 |  2203 |    69 |       13 | 100.00% | Network size classification         |
| src                     | nickname.rs          |     5 | 11658 |   310 |      125 |  99.20% | Nickname management                 |
| src                     | peers.rs             |     4 |  8137 |   234 |       73 |  95.89% | Peer management & tracking          |
| src                     | swarm_handler.rs     |     7 |  9571 |   275 |      118 |  88.14% | Network event translation           |
| src                     | tui_helpers.rs       |     5 | 15055 |   470 |      171 |  98.83% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |     5 | 20508 |   581 |      295 |  82.71% | TUI rendering & state management    |
| src                     | tui_render_state.rs  |     4 | 14555 |   429 |      140 |  95.71% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |     5 |  7909 |   239 |       87 |  95.40% | Tab management & navigation         |
| src                     | types.rs             |     2 |  3770 |   120 |        2 | 100.00% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |     7 |  4958 |   134 |       62 |   0.00% | CLI chat application                |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4836 |   124 |       20 |   0.00% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |     6 | 12149 |   299 |      158 |  93.67% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |     6 | 12023 |   352 |      180 |  98.89% | Event routing & state updates       |
| src/bin/tui             | event_source.rs      |     4 |  1265 |    41 |       12 |  41.67% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |     5 | 20190 |   544 |      240 |  78.33% | Input event routing & processing    |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |       73 |   0.00% | Source file                         |
| src/bin/tui             | main_loop.rs         |     4 | 13266 |   353 |      161 |  24.84% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |     5 |  4759 |   146 |       75 |  97.33% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |     5 | 10336 |   288 |      133 |  96.24% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |     5 | 11235 |   277 |       63 |  95.24% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |     3 |  2881 |    85 |       31 |  22.58% | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |     6 |  8032 |   236 |       88 |  50.00% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |     1 |  2180 |    53 |        0 |       - | Auto-generated column definitions   |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                 |
| src/generated           | models_insertable.rs |     1 |  3383 |    94 |        0 |       - | Insertable data models              |
| src/generated           | models_queryable.rs  |     1 |  4022 |   112 |        0 |       - | Queryable data models               |
| src/generated           | schema.rs            |     2 |  2702 |   103 |       53 |  88.68% | Database schema (Diesel)            |

**Total:** 39 files, 11,607 lines, 417,879 characters (2374/3811 testable lines covered, 62%)

## Rust Test Files

| Folder | File                               | Lines | Chars | Depth | Description                           |
|:-------|:-----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs                |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                 |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs              |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                  |   251 |  7367 |     6 | Test file                             |
| tests  | additional_coverage.rs             |   130 |  4115 |     2 | Additional coverage tests             |
| tests  | behavior.rs                        |   218 |  6210 |     5 | behavior module tests                 |
| tests  | db.rs                              |   281 |  8650 |     4 | database module tests                 |
| tests  | db_selection.rs                    |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                             |   262 |  6939 |     2 | fmt module tests                      |
| tests  | logging.rs                         |   296 |  8311 |     3 | logging module tests                  |
| tests  | messages.rs                        |   477 | 14302 |     3 | messages module tests                 |
| tests  | network.rs                         |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                        |   516 | 17422 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                 |  1033 | 36651 |    10 | P2P integration tests                 |
| tests  | peers.rs                           |   257 |  7305 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                   |   542 | 18172 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                      |    49 |  1818 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs          |   292 |  8969 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                        |   733 | 23268 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                     |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                 |   482 | 15237 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs          |   831 | 26690 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                       |   268 |  8191 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                       |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                           |   646 | 20117 |     3 | types module tests                    |
| unit   | unit_behavior.rs                   |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs     |   438 | 15793 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs  |   908 | 29970 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs       |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs    |   992 | 33913 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs          |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs   |   248 |  8189 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_layout.rs |    54 |  1624 |     2 | Test file                             |
| unit   | unit_bin_tui_render_loop_mod.rs    |   231 |  7176 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs    |   569 | 17984 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs              |   228 |  6532 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs       |    60 |  1835 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_connected.rs                  |    65 |  2019 |     1 | Test file                             |
| unit   | unit_db.rs                         |   273 |  8621 |     3 | Unit tests for database module        |
| unit   | unit_lib.rs                        |   249 |  6836 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                    |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                   |   204 |  7532 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                    |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                   |   147 |  4848 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                      |    63 |  1805 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs              |    98 |  2897 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs                |   477 | 16450 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs           |   256 |  7362 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                   |   206 |  5919 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs             |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                      |   256 |  6968 |     3 | Unit tests for types module           |

**Total:** 51 test files, 15,336 lines, 487,012 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |    19 | 64123 |  2049 |      949 |  12.01% | Flutter app entry point          |
| lib/src/rust | api.dart               |     2 |  5410 |   150 |       32 |   6.25% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 54850 |  1773 |      700 |   0.29% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 |  8364 |   335 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 |  8284 |   335 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        8 |   0.00% | Dart source file                 |
| lib/src/rust | mobile_api.dart        |     5 |  7193 |   217 |       69 |   1.45% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 |  4668 |   156 |       63 |   1.59% | Mobile node bindings (generated) |

**Total:** 8 files, 5,047 lines, 153,959 characters (120/1826 testable lines covered, 7%)

## Dart Test Files

| Folder  | File                           | Lines | Chars | Depth | Description                           |
|:--------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | test_helpers.dart              |    46 |  1422 |     4 | Test utilities & helpers              |
| unit    | api_test.dart                  |    66 |  2434 |     5 | Dart API layer unit tests             |
| widget  | peer_list_test.dart            |   182 |  6257 |     5 | Dart test file                        |
| test    | widget_test.dart               |    19 |   615 |     2 | Widget smoke tests                    |

**Total:** 4 test files, 313 lines, 10,728 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  2983 |    85 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  3822 |   119 |        - |       - | Foreground service for background networking |

**Total:** 2 files, 204 lines, 6,805 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | MainActivityTest.kt            |    69 |  2411 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 2 test files, 168 lines, 5,504 characters
