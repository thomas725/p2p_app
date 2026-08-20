# Codebase Metrics

## Summary

| Metric                |   Value |
|:----------------------|:--------|
| Total Rust Files      |      41 |
| Total Rust Lines      |   9,289 |
| Total Rust Chars      | 324,998 |
| Avg Lines/Rust File   |     226 |
| Avg Chars/Rust File   |   7,926 |
| Total Dart Files      |       8 |
| Total Dart Lines      |   3,281 |
| Total Dart Chars      |  97,727 |
| Avg Lines/Dart File   |     410 |
| Avg Chars/Dart File   |  12,215 |
| Total Kotlin Files    |       2 |
| Total Kotlin Lines    |     199 |
| Total Kotlin Chars    |   6,491 |
| Avg Lines/Kotlin File |      99 |
| Avg Chars/Kotlin File |   3,245 |

**Grand Total:** 51 files, 12,769 lines, 429,216 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  3949 |   118 |        - |       - | Build script                           |
| src                     | behavior.rs          |     4 |  6157 |   181 |        - |       - | Network behavior definitions           |
| src                     | db.rs                |     6 | 15218 |   421 |        - |       - | Database connection & identity mgmt    |
| src                     | dioxus_app.rs        |    11 | 19980 |   552 |        - |       - | Web UI app shell & components (Dioxus) |
| src                     | dioxus_styles.rs     |     0 |  3862 |    50 |        - |       - | Web UI CSS styles (Dioxus)             |
| src                     | dioxus_swarm.rs      |     5 |  6079 |   160 |        - |       - | Web UI swarm event handling (Dioxus)   |
| src                     | fmt.rs               |     4 |  3838 |   129 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     6 | 45651 |  1164 |        - |       - | flutter_rust_bridge codegen            |
| src                     | lib.rs               |     1 |  6054 |   160 |        - |       - | Module declarations & re-exports       |
| src                     | logging.rs           |     4 |  8974 |   282 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     4 |  8406 |   227 |        - |       - | Message persistence & retrieval        |
| src                     | mobile_api.rs        |     3 |  6908 |   206 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 28470 |   873 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     1 |  3247 |    94 |        - |       - | Module declarations                    |
| src                     | network.rs           |     3 |  1851 |    61 |        - |       - | Network size classification            |
| src                     | nickname.rs          |     5 |  4699 |   129 |        - |       - | Nickname management                    |
| src                     | peers.rs             |     3 |  6171 |   182 |        - |       - | Peer management & tracking             |
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
| src/generated           | columns.rs           |     1 |  1653 |    42 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  3256 |    92 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  3895 |   110 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  1570 |    71 |        - |       - | Database schema (Diesel)               |

**Total:** 41 files, 9,289 lines, 324,998 characters

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | db_test_utils.rs                  |     8 |   209 |     2 | Test file                             |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   241 |  6963 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   119 |  3892 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   207 |  5990 |     5 | behavior module tests                 |
| tests  | db.rs                             |   188 |  5305 |     3 | database module tests                 |
| tests  | db_selection.rs                   |    57 |  1737 |     4 | Database selection tests              |
| tests  | fmt.rs                            |   291 |  7626 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   296 |  8311 |     3 | logging module tests                  |
| tests  | messages.rs                       |   496 | 14906 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   398 | 12478 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1021 | 36271 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   322 |  8933 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   513 | 17514 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    33 |  1292 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   284 |  8871 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   759 | 24232 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   665 | 19787 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   456 | 14487 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   640 | 19710 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   282 |  8574 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   233 |  7533 |     7 | TUI task tests                        |
| tests  | types.rs                          |   695 | 21445 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   226 |  7867 |     2 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   874 | 29128 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   610 | 18637 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   236 |  7238 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   251 |  8153 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   193 |  5783 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   497 | 15673 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   233 |  6685 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    62 |  1896 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_db.rs                        |   281 |  8768 |     4 | Unit tests for database module        |
| unit   | unit_lib.rs                       |   264 |  7191 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   204 |  5201 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   190 |  6084 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    46 |  1371 |     3 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   126 |  3935 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |    80 |  2260 |     3 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |    98 |  2844 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   126 |  3954 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   256 |  7337 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   193 |  5390 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   254 |  6822 |     3 | Unit tests for types module           |

**Total:** 50 test files, 14,015 lines, 434,441 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| lib          | main.dart              |    19 | 41128 |  1339 |        - |       - | Flutter app entry point             |
| lib/src/rust | api.dart               |     2 |  3414 |    87 |        - |       - | FRB API bindings (generated)        |
| lib/src/rust | frb_generated.dart     |     6 | 35310 |  1157 |        - |       - | flutter_rust_bridge codegen         |
| lib/src/rust | frb_generated.io.dart  |     3 |  5878 |   232 |        - |       - | FRB IO bindings (generated)         |
| lib/src/rust | frb_generated.web.dart |     2 |  5798 |   232 |        - |       - | FRB web bindings (generated)        |
| lib/src/rust | mobile_api.dart        |     5 |  1464 |    53 |        - |       - | Mobile API bindings (generated)     |
| lib/src/rust | mobile_node.dart       |     5 |  3652 |   144 |        - |       - | Mobile node bindings (generated)    |
| lib/src/rust | types.dart             |     5 |  1083 |    37 |        - |       - | Shared type definitions (generated) |

**Total:** 8 files, 3,281 lines, 97,727 characters

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
