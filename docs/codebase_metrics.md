# Codebase Metrics

## Summary

| Metric                |           Value |
|:----------------------|:----------------|
| Total Rust Files      |              43 |
| Total Rust Lines      |          12,048 |
| Total Rust Chars      |         434,443 |
| Avg Lines/Rust File   |             280 |
| Avg Chars/Rust File   |          10,103 |
| Total Dart Files      |               9 |
| Total Dart Lines      |           4,522 |
| Total Dart Chars      |         137,907 |
| Avg Lines/Dart File   |             502 |
| Avg Chars/Dart File   |          15,323 |
| Covered Dart Lines    | 41 / 1,664 (2%) |
| Total Kotlin Files    |               2 |
| Total Kotlin Lines    |             204 |
| Total Kotlin Chars    |           6,805 |
| Avg Lines/Kotlin File |             102 |
| Avg Chars/Kotlin File |           3,402 |

**Grand Total:** 54 files, 16,774 lines, 579,155 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        - |       - | Build script                           |
| src                     | behavior.rs          |     6 |  7356 |   207 |        - |       - | Network behavior definitions           |
| src                     | connected.rs         |     3 |  2692 |    84 |        - |       - | Source file                            |
| src                     | db.rs                |     5 | 23496 |   604 |        - |       - | Database connection & identity mgmt    |
| src                     | dioxus_app.rs        |    11 | 20021 |   551 |        - |       - | Web UI app shell & components (Dioxus) |
| src                     | dioxus_styles.rs     |     0 |  3862 |    50 |        - |       - | Web UI CSS styles (Dioxus)             |
| src                     | dioxus_swarm.rs      |     6 |  6613 |   168 |        - |       - | Web UI swarm event handling (Dioxus)   |
| src                     | fmt.rs               |     4 |  3425 |   117 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     7 | 60496 |  1530 |        - |       - | flutter_rust_bridge codegen            |
| src                     | lib.rs               |     1 |  5845 |   157 |        - |       - | Module declarations & re-exports       |
| src                     | logging.rs           |     4 | 11659 |   359 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     4 |  9473 |   281 |        - |       - | Message persistence & retrieval        |
| src                     | mobile_api.rs        |     3 |  7859 |   232 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 36030 |  1051 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     1 |  5754 |   174 |        - |       - | Module declarations                    |
| src                     | network.rs           |     3 |  2203 |    69 |        - |       - | Network size classification            |
| src                     | nickname.rs          |     5 | 11789 |   318 |        - |       - | Nickname management                    |
| src                     | peers.rs             |     4 |  7969 |   235 |        - |       - | Peer management & tracking             |
| src                     | swarm_handler.rs     |     7 |  9571 |   275 |        - |       - | Network event translation              |
| src                     | tui_helpers.rs       |     5 | 15490 |   481 |        - |       - | TUI helper functions & utilities       |
| src                     | tui_render.rs        |     5 | 20752 |   592 |        - |       - | TUI rendering & state management       |
| src                     | tui_render_state.rs  |     4 | 14464 |   428 |        - |       - | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     5 |  7909 |   239 |        - |       - | Tab management & navigation            |
| src                     | types.rs             |     2 |  3770 |   120 |        - |       - | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  4958 |   134 |        - |       - | CLI chat application                   |
| src/bin                 | p2p_chat_dioxus.rs   |     8 |  8528 |   222 |        - |       - | Web UI (Dioxus framework)              |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4836 |   124 |        - |       - | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     6 | 12281 |   304 |        - |       - | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 12023 |   352 |        - |       - | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1265 |    41 |        - |       - | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     5 | 20132 |   543 |        - |       - | Input event routing & processing       |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |        - |       - | Source file                            |
| src/bin/tui             | main_loop.rs         |     4 | 13338 |   355 |        - |       - | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     5 |  4461 |   138 |        - |       - | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 | 10486 |   294 |        - |       - | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     4 | 11186 |   285 |        - |       - | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     3 |  2881 |    85 |        - |       - | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     6 |  8036 |   236 |        - |       - | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  1743 |    44 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  3383 |    94 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  4022 |   112 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  1652 |    73 |        - |       - | Database schema (Diesel)               |

**Total:** 43 files, 12,048 lines, 434,443 characters

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
| unit   | unit_bin_tui_click_handlers.rs     |   433 | 15721 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs  |   908 | 29970 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs       |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs    |   992 | 33913 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs          |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs   |   268 |  8523 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_layout.rs |    54 |  1624 |     2 | Test file                             |
| unit   | unit_bin_tui_render_loop_mod.rs    |   232 |  7200 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs    |   569 | 17997 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs              |   233 |  6672 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs       |    62 |  1896 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_connected.rs                  |    76 |  2194 |     1 | Test file                             |
| unit   | unit_db.rs                         |   273 |  8621 |     3 | Unit tests for database module        |
| unit   | unit_lib.rs                        |   249 |  6836 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                    |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                   |   163 |  5471 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                    |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                   |   147 |  4848 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                      |    63 |  1805 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs              |    98 |  2897 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs                |   414 | 14382 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs           |   256 |  7362 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                   |   206 |  5919 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs             |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                      |   256 |  6968 |     3 | Unit tests for types module           |

**Total:** 51 test files, 15,266 lines, 483,558 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| lib          | main.dart              |    19 | 63206 |  2025 |      933 |   3.97% | Flutter app entry point             |
| lib/src/rust | api.dart               |     2 |  4371 |   106 |       34 |   5.88% | FRB API bindings (generated)        |
| lib/src/rust | frb_generated.dart     |     6 | 46553 |  1520 |      595 |   0.34% | flutter_rust_bridge codegen         |
| lib/src/rust | frb_generated.io.dart  |     3 |  6917 |   275 |        5 |   0.00% | FRB IO bindings (generated)         |
| lib/src/rust | frb_generated.web.dart |     2 |  6837 |   275 |        - |       - | FRB web bindings (generated)        |
| lib/src/rust | messages.dart          |     5 |  1254 |    38 |        9 |   0.00% | Dart source file                    |
| lib/src/rust | mobile_api.dart        |     5 |  3113 |    93 |       25 |   0.00% | Mobile API bindings (generated)     |
| lib/src/rust | mobile_node.dart       |     5 |  4573 |   153 |       63 |   0.00% | Mobile node bindings (generated)    |
| lib/src/rust | types.dart             |     5 |  1083 |    37 |        - |       - | Shared type definitions (generated) |

**Total:** 9 files, 4,522 lines, 137,907 characters (41/1664 testable lines covered, 2%)

## Dart Test Files

| Folder  | File                           | Lines | Chars | Depth | Description                           |
|:--------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | test_helpers.dart              |    46 |  1422 |     4 | Test utilities & helpers              |
| unit    | api_test.dart                  |    42 |  1059 |     4 | Dart API layer unit tests             |
| test    | widget_test.dart               |    19 |   615 |     2 | Widget smoke tests                    |

**Total:** 3 test files, 107 lines, 3,096 characters

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
