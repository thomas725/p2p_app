# Codebase Metrics

## Summary

| Metric                |           Value |
|:----------------------|:----------------|
| Total Rust Files      |              41 |
| Total Rust Lines      |          10,093 |
| Total Rust Chars      |         357,662 |
| Avg Lines/Rust File   |             246 |
| Avg Chars/Rust File   |           8,723 |
| Total Dart Files      |               9 |
| Total Dart Lines      |           4,504 |
| Total Dart Chars      |         137,425 |
| Avg Lines/Dart File   |             500 |
| Avg Chars/Dart File   |          15,269 |
| Covered Dart Lines    | 41 / 1,656 (2%) |
| Total Kotlin Files    |               2 |
| Total Kotlin Lines    |             199 |
| Total Kotlin Chars    |           6,491 |
| Avg Lines/Kotlin File |              99 |
| Avg Chars/Kotlin File |           3,245 |

**Grand Total:** 52 files, 14,796 lines, 501,578 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  3949 |   118 |        - |       - | Build script                           |
| src                     | behavior.rs          |     4 |  6157 |   181 |        - |       - | Network behavior definitions           |
| src                     | db.rs                |     6 | 16831 |   454 |        - |       - | Database connection & identity mgmt    |
| src                     | dioxus_app.rs        |    11 | 19980 |   552 |        - |       - | Web UI app shell & components (Dioxus) |
| src                     | dioxus_styles.rs     |     0 |  3862 |    50 |        - |       - | Web UI CSS styles (Dioxus)             |
| src                     | dioxus_swarm.rs      |     5 |  6079 |   160 |        - |       - | Web UI swarm event handling (Dioxus)   |
| src                     | fmt.rs               |     4 |  3839 |   129 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     7 | 60411 |  1526 |        - |       - | flutter_rust_bridge codegen            |
| src                     | lib.rs               |     1 |  6105 |   160 |        - |       - | Module declarations & re-exports       |
| src                     | logging.rs           |     4 |  9619 |   300 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     4 | 10142 |   276 |        - |       - | Message persistence & retrieval        |
| src                     | mobile_api.rs        |     3 |  7760 |   232 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 33318 |   982 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     1 |  4224 |   117 |        - |       - | Module declarations                    |
| src                     | network.rs           |     3 |  1851 |    61 |        - |       - | Network size classification            |
| src                     | nickname.rs          |     5 |  9501 |   252 |        - |       - | Nickname management                    |
| src                     | peers.rs             |     4 |  8124 |   235 |        - |       - | Peer management & tracking             |
| src                     | swarm_handler.rs     |     7 |  9362 |   272 |        - |       - | Network event translation              |
| src                     | tui_helpers.rs       |     3 |  6896 |   226 |        - |       - | TUI helper functions & utilities       |
| src                     | tui_render.rs        |     5 | 11816 |   363 |        - |       - | TUI rendering & state management       |
| src                     | tui_render_state.rs  |     4 | 11951 |   379 |        - |       - | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     5 |  4793 |   168 |        - |       - | Tab management & navigation            |
| src                     | types.rs             |     2 |  3546 |   115 |        - |       - | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  4936 |   134 |        - |       - | CLI chat application                   |
| src/bin                 | p2p_chat_dioxus.rs   |     8 |  8302 |   218 |        - |       - | Web UI (Dioxus framework)              |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4547 |   117 |        - |       - | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     5 |  5185 |   137 |        - |       - | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 10698 |   321 |        - |       - | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1224 |    40 |        - |       - | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     5 | 12090 |   356 |        - |       - | Input event routing & processing       |
| src/bin/tui             | main_loop.rs         |     4 | 10740 |   305 |        - |       - | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     5 |  4473 |   138 |        - |       - | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 |  8766 |   252 |        - |       - | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     3 |  8341 |   226 |        - |       - | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     3 |  1882 |    56 |        - |       - | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     4 |  5074 |   151 |        - |       - | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  1743 |    44 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  3383 |    94 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  4022 |   112 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  1652 |    73 |        - |       - | Database schema (Diesel)               |

**Total:** 41 files, 10,093 lines, 357,662 characters

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   241 |  6963 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   119 |  3892 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   207 |  5990 |     5 | behavior module tests                 |
| tests  | db.rs                             |   262 |  8507 |     3 | database module tests                 |
| tests  | db_selection.rs                   |    53 |  1557 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   291 |  7626 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   296 |  8311 |     3 | logging module tests                  |
| tests  | messages.rs                       |   537 | 16259 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   505 | 17208 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1018 | 36219 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   322 |  8933 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   519 | 17594 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    36 |  1518 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   284 |  8871 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   759 | 24232 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   665 | 19787 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   456 | 14487 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   640 | 19710 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   282 |  8574 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   233 |  7533 |     7 | TUI task tests                        |
| tests  | types.rs                          |   695 | 21445 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   226 |  7841 |     2 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   874 | 28985 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   610 | 18585 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   239 |  7348 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   251 |  8114 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   193 |  5783 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   497 | 15673 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   233 |  6672 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    62 |  1896 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_db.rs                        |   257 |  7960 |     2 | Unit tests for database module        |
| unit   | unit_lib.rs                       |   264 |  7191 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   204 |  5201 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   187 |  6143 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    41 |  1267 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   129 |  4123 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |    75 |  2156 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |    98 |  2844 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   126 |  3954 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   256 |  7337 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   193 |  5390 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   254 |  6822 |     3 | Unit tests for types module           |

**Total:** 49 test files, 14,200 lines, 442,659 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| lib          | main.dart              |    19 | 62724 |  2007 |      925 |   4.00% | Flutter app entry point             |
| lib/src/rust | api.dart               |     2 |  4371 |   106 |       34 |   5.88% | FRB API bindings (generated)        |
| lib/src/rust | frb_generated.dart     |     6 | 46553 |  1520 |      595 |   0.34% | flutter_rust_bridge codegen         |
| lib/src/rust | frb_generated.io.dart  |     3 |  6917 |   275 |        5 |   0.00% | FRB IO bindings (generated)         |
| lib/src/rust | frb_generated.web.dart |     2 |  6837 |   275 |        - |       - | FRB web bindings (generated)        |
| lib/src/rust | messages.dart          |     5 |  1254 |    38 |        9 |   0.00% | Dart source file                    |
| lib/src/rust | mobile_api.dart        |     5 |  3113 |    93 |       25 |   0.00% | Mobile API bindings (generated)     |
| lib/src/rust | mobile_node.dart       |     5 |  4573 |   153 |       63 |   0.00% | Mobile node bindings (generated)    |
| lib/src/rust | types.dart             |     5 |  1083 |    37 |        - |       - | Shared type definitions (generated) |

**Total:** 9 files, 4,504 lines, 137,425 characters (41/1656 testable lines covered, 2%)

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
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  2669 |    80 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  3822 |   119 |        - |       - | Foreground service for background networking |

**Total:** 2 files, 199 lines, 6,491 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | MainActivityTest.kt            |    69 |  2411 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 2 test files, 168 lines, 5,504 characters
