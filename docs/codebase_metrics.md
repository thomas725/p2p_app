# Codebase Metrics

## Summary

| Metric                |           Value |
|:----------------------|:----------------|
| Total Rust Files      |              42 |
| Total Rust Lines      |          11,619 |
| Total Rust Chars      |         417,350 |
| Avg Lines/Rust File   |             276 |
| Avg Chars/Rust File   |           9,936 |
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

**Grand Total:** 53 files, 16,345 lines, 562,062 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        - |       - | Build script                           |
| src                     | behavior.rs          |     4 |  7699 |   212 |        - |       - | Network behavior definitions           |
| src                     | connected.rs         |     3 |  2691 |    84 |        - |       - | Source file                            |
| src                     | db.rs                |     5 | 21167 |   560 |        - |       - | Database connection & identity mgmt    |
| src                     | dioxus_app.rs        |    11 | 20052 |   552 |        - |       - | Web UI app shell & components (Dioxus) |
| src                     | dioxus_styles.rs     |     0 |  3862 |    50 |        - |       - | Web UI CSS styles (Dioxus)             |
| src                     | dioxus_swarm.rs      |     5 |  6140 |   161 |        - |       - | Web UI swarm event handling (Dioxus)   |
| src                     | fmt.rs               |     4 |  3943 |   131 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     7 | 60496 |  1530 |        - |       - | flutter_rust_bridge codegen            |
| src                     | lib.rs               |     1 |  6130 |   161 |        - |       - | Module declarations & re-exports       |
| src                     | logging.rs           |     4 | 11659 |   359 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     4 | 10986 |   309 |        - |       - | Message persistence & retrieval        |
| src                     | mobile_api.rs        |     3 |  7768 |   232 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 36166 |  1053 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     1 |  5754 |   174 |        - |       - | Module declarations                    |
| src                     | network.rs           |     3 |  2203 |    69 |        - |       - | Network size classification            |
| src                     | nickname.rs          |     5 | 10337 |   281 |        - |       - | Nickname management                    |
| src                     | peers.rs             |     4 |  8822 |   264 |        - |       - | Peer management & tracking             |
| src                     | swarm_handler.rs     |     7 |  9571 |   275 |        - |       - | Network event translation              |
| src                     | tui_helpers.rs       |     5 | 15253 |   483 |        - |       - | TUI helper functions & utilities       |
| src                     | tui_render.rs        |     5 | 19335 |   548 |        - |       - | TUI rendering & state management       |
| src                     | tui_render_state.rs  |     4 | 15284 |   446 |        - |       - | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     5 |  8454 |   261 |        - |       - | Tab management & navigation            |
| src                     | types.rs             |     2 |  3546 |   115 |        - |       - | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  4958 |   134 |        - |       - | CLI chat application                   |
| src/bin                 | p2p_chat_dioxus.rs   |     8 |  8545 |   223 |        - |       - | Web UI (Dioxus framework)              |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4547 |   117 |        - |       - | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     6 | 11514 |   279 |        - |       - | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 12497 |   365 |        - |       - | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1265 |    41 |        - |       - | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     5 | 17836 |   501 |        - |       - | Input event routing & processing       |
| src/bin/tui             | main_loop.rs         |     5 | 11162 |   315 |        - |       - | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     5 |  4461 |   138 |        - |       - | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 |  9187 |   255 |        - |       - | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     3 |  9399 |   245 |        - |       - | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     3 |  2564 |    71 |        - |       - | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     4 |  6741 |   174 |        - |       - | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  1743 |    44 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  3383 |    94 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  4022 |   112 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  1652 |    73 |        - |       - | Database schema (Diesel)               |

**Total:** 42 files, 11,619 lines, 417,350 characters

## Rust Test Files

| Folder | File                               | Lines | Chars | Depth | Description                           |
|:-------|:-----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs                |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                 |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs              |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                  |   251 |  7367 |     6 | Test file                             |
| tests  | additional_coverage.rs             |   121 |  4081 |     2 | Additional coverage tests             |
| tests  | behavior.rs                        |   209 |  6176 |     5 | behavior module tests                 |
| tests  | db.rs                              |   266 |  8773 |     3 | database module tests                 |
| tests  | db_selection.rs                    |    56 |  1737 |     3 | Database selection tests              |
| tests  | fmt.rs                             |   293 |  7808 |     2 | fmt module tests                      |
| tests  | logging.rs                         |   296 |  8311 |     3 | logging module tests                  |
| tests  | messages.rs                        |   541 | 16495 |     3 | messages module tests                 |
| tests  | network.rs                         |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                        |   507 | 17388 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                 |  1024 | 36617 |    10 | P2P integration tests                 |
| tests  | peers.rs                           |   325 |  9149 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                   |   529 | 18060 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                      |    40 |  1784 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs          |   287 |  9127 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                        |   763 | 24480 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                     |   665 | 19787 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                 |   473 | 15203 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs          |   713 | 23067 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                       |   286 |  9037 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                       |   236 |  7777 |     7 | TUI task tests                        |
| tests  | types.rs                           |   697 | 21619 |     3 | types module tests                    |
| unit   | unit_behavior.rs                   |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs     |   410 | 15076 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs  |   876 | 29288 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs       |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs    |   735 | 23544 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs          |   240 |  7524 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs   |   253 |  8467 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_layout.rs |    31 |  1131 |     2 | Test file                             |
| unit   | unit_bin_tui_render_loop_mod.rs    |   230 |  7182 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs    |   497 | 15673 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs              |   233 |  6672 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs       |    62 |  1896 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_connected.rs                  |    76 |  2193 |     1 | Test file                             |
| unit   | unit_db.rs                         |   259 |  8046 |     2 | Unit tests for database module        |
| unit   | unit_lib.rs                        |   263 |  7158 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                    |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                   |   188 |  6169 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                    |    41 |  1267 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                   |   129 |  4123 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                      |    77 |  2212 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs              |    98 |  2897 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs                |   232 |  8164 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs           |   256 |  7362 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                   |   232 |  6703 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs             |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                      |   256 |  6968 |     3 | Unit tests for types module           |

**Total:** 51 test files, 14,968 lines, 474,938 characters

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
