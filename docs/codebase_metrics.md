# Codebase Metrics

## Summary

| Metric                |   Value |
|:----------------------|:--------|
| Total Rust Files      |      46 |
| Total Rust Lines      |  13,949 |
| Total Rust Chars      | 509,499 |
| Avg Lines/Rust File   |     303 |
| Avg Chars/Rust File   |  11,076 |
| Total Dart Files      |      20 |
| Total Dart Lines      |   6,195 |
| Total Dart Chars      | 188,987 |
| Avg Lines/Dart File   |     309 |
| Avg Chars/Dart File   |   9,449 |
| Total Kotlin Files    |       2 |
| Total Kotlin Lines    |     209 |
| Total Kotlin Chars    |   7,161 |
| Avg Lines/Kotlin File |     104 |
| Avg Chars/Kotlin File |   3,580 |

**Grand Total:** 68 files, 20,353 lines, 705,647 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                               |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|--------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        - |       - | Build script                          |
| src                     | behavior.rs          |     6 |  7378 |   206 |        - |       - | Network behavior definitions          |
| src                     | chat.rs              |     5 |  6015 |   169 |        - |       - | TUI chat & DM pane renderer           |
| src                     | connected.rs         |     3 |  2324 |    71 |        - |       - | Connected peers tracking & monitoring |
| src                     | db.rs                |     5 | 24384 |   627 |        - |       - | Database connection & identity mgmt   |
| src                     | fmt.rs               |     4 |  4018 |   129 |        - |       - | Formatting & display utilities        |
| src                     | frb_generated.rs     |     6 | 78844 |  1951 |        - |       - | flutter_rust_bridge codegen           |
| src                     | groups.rs            |     3 | 13193 |   371 |        - |       - | Group management & persistence        |
| src                     | groups.rs            |     4 |  3824 |   102 |        - |       - | TUI groups renderer                   |
| src                     | lib.rs               |     1 |  6122 |   161 |        - |       - | Module declarations & re-exports      |
| src                     | log.rs               |     2 |   907 |    31 |        - |       - | TUI log renderer                      |
| src                     | logging.rs           |     8 | 13114 |   396 |        - |       - | Logging utilities & setup             |
| src                     | messages.rs          |     5 | 11998 |   336 |        - |       - | Message persistence & retrieval       |
| src                     | mobile_api.rs        |     4 | 15629 |   469 |        - |       - | Mobile FRB API surface                |
| src                     | mobile_node.rs       |     5 | 48419 |  1367 |        - |       - | Mobile node lifecycle & swarm         |
| src                     | mod.rs               |     1 |  6844 |   210 |        - |       - | Module declarations                   |
| src                     | mod.rs               |     4 |  6782 |   194 |        - |       - | TUI render chrome & dispatch          |
| src                     | network.rs           |     3 |  2203 |    69 |        - |       - | Network size classification           |
| src                     | nickname.rs          |     5 | 11989 |   319 |        - |       - | Nickname management                   |
| src                     | peer_info.rs         |     3 |  2023 |    62 |        - |       - | TUI peer-info renderer                |
| src                     | peers.rs             |     4 |  9553 |   266 |        - |       - | Peer management & tracking            |
| src                     | peers.rs             |     4 |  3486 |    89 |        - |       - | TUI peers-table renderer              |
| src                     | settings.rs          |     4 |  3322 |    84 |        - |       - | TUI settings renderer                 |
| src                     | swarm_handler.rs     |     8 | 17007 |   450 |        - |       - | Network event translation             |
| src                     | tui_helpers.rs       |     5 | 14994 |   469 |        - |       - | TUI helper functions & utilities      |
| src                     | tui_render_state.rs  |     4 | 18229 |   516 |        - |       - | TUI render state & tab content        |
| src                     | tui_tabs.rs          |     6 | 11965 |   351 |        - |       - | Tab management & navigation           |
| src                     | types.rs             |     2 |  5193 |   165 |        - |       - | Event & command type defs             |
| src/bin                 | p2p_chat.rs          |     7 |  4958 |   134 |        - |       - | CLI chat application                  |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4836 |   124 |        - |       - | Main TUI application entry point      |
| src/bin/tui             | click_handlers.rs    |     7 | 19239 |   471 |        - |       - | Click handlers & index mapping        |
| src/bin/tui             | command_processor.rs |     6 | 16010 |   456 |        - |       - | Event routing & state updates         |
| src/bin/tui             | event_source.rs      |     4 |  1258 |    41 |        - |       - | Terminal event polling (60 FPS)       |
| src/bin/tui             | input_processor.rs   |     5 | 23353 |   614 |        - |       - | Input event routing & processing      |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |        - |       - | Keyboard enhancement probe            |
| src/bin/tui             | main_loop.rs         |     4 | 14040 |   372 |        - |       - | Task orchestration & async            |
| src/bin/tui             | message_handlers.rs  |     5 |  7717 |   229 |        - |       - | Message sending logic                 |
| src/bin/tui             | scroll_handlers.rs   |     5 | 16000 |   419 |        - |       - | Scroll & hover-aware navigation       |
| src/bin/tui             | state.rs             |     5 | 12791 |   313 |        - |       - | Shared application state              |
| src/bin/tui/render_loop | layout.rs            |     3 |  1724 |    50 |        - |       - | UI layout component rendering         |
| src/bin/tui/render_loop | mod.rs               |     6 |  9537 |   283 |        - |       - | Render loop orchestration (60 FPS)    |
| src/generated           | columns.rs           |     1 |  2951 |    71 |        - |       - | Auto-generated column definitions     |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                   |
| src/generated           | models_insertable.rs |     1 |  4842 |   139 |        - |       - | Insertable data models                |
| src/generated           | models_queryable.rs  |     1 |  5946 |   169 |        - |       - | Queryable data models                 |
| src/generated           | schema.rs            |     2 |  3804 |   144 |        - |       - | Database schema (Diesel)              |

**Total:** 46 files, 13,949 lines, 509,499 characters

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   251 |  7367 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   131 |  4139 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   222 |  6306 |     5 | behavior module tests                 |
| tests  | db.rs                             |   281 |  8650 |     4 | database module tests                 |
| tests  | db_selection.rs                   |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   262 |  6939 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   335 |  9678 |     3 | logging module tests                  |
| tests  | messages.rs                       |   477 | 14302 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17422 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1033 | 36651 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   257 |  7305 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   584 | 19643 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    49 |  1818 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   293 |  9009 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23473 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   496 | 15742 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   867 | 28074 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8301 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   647 | 20154 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    59 |  1847 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   518 | 18885 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   908 | 29970 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   992 | 34077 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   248 |  8189 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   230 |  7114 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   657 | 21474 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   227 |  6483 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    61 |  1855 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_db.rs                        |   273 |  8621 |     3 | Unit tests for database module        |
| unit   | unit_groups.rs                    |   276 |  9724 |     5 | Unit tests for groups module          |
| unit   | unit_lib.rs                       |   250 |  6860 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   232 |  8907 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   147 |  4848 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |   115 |  3450 |     3 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |   104 |  3173 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   477 | 16450 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   365 | 11159 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   320 |  9819 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   326 |  9276 |     3 | Unit tests for types module           |

**Total:** 51 test files, 16,251 lines, 520,355 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |     5 |  1565 |    62 |        - |       - | Flutter app entry point          |
| lib/src/rust | api.dart               |     2 |  6625 |   186 |        - |       - | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 61012 |  1964 |        - |       - | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 |  9248 |   372 |        - |       - | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 |  9168 |   372 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        - |       - | Dart source file                 |
| lib/src/rust | mobile_api.dart        |     5 |  6554 |   200 |        - |       - | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 |  6845 |   238 |        - |       - | Mobile node bindings (generated) |
| lib          | dm_chat.dart           |     9 |  6004 |   199 |        - |       - | Dart source file                 |
| lib          | group_chat.dart        |     8 |  6932 |   220 |        - |       - | Dart source file                 |
| lib          | group_list.dart        |    12 |  5671 |   175 |        - |       - | Dart source file                 |
| lib          | home.dart              |     7 | 19494 |   660 |        - |       - | Dart source file                 |
| lib          | log_tab.dart           |    10 |  3395 |   121 |        - |       - | Dart source file                 |
| lib          | messages.dart          |    14 | 17453 |   545 |        - |       - | Dart source file                 |
| lib          | nav_bar.dart           |     3 |  1246 |    34 |        - |       - | Dart source file                 |
| lib          | peer_info.dart         |    11 |  5641 |   172 |        - |       - | Dart source file                 |
| lib          | peer_list.dart         |    18 |  6135 |   178 |        - |       - | Dart source file                 |
| lib          | settings.dart          |    13 | 11923 |   382 |        - |       - | Dart source file                 |
| lib          | event_bus.dart         |     4 |  1684 |    50 |        - |       - | Dart source file                 |
| lib          | ui_utils.dart          |     2 |  1325 |    33 |        - |       - | Dart source file                 |

**Total:** 20 files, 6,195 lines, 188,987 characters

## Dart Test Files

| Folder | File                           | Lines | Chars | Depth | Description                           |
|:-------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| unit   | api_test.dart                  |    53 |  2156 |     4 | Dart API layer unit tests             |
| widget | group_list_test.dart           |   138 |  4205 |     5 | Dart test file                        |
| widget | peer_list_test.dart            |   289 |  9869 |     5 | Dart test file                        |
| test   | widget_test.dart               |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 4 test files, 501 lines, 16,854 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  2879 |    82 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  4282 |   127 |        - |       - | Foreground service for background networking |

**Total:** 2 files, 209 lines, 7,161 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | MainActivityTest.kt            |    67 |  2326 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 2 test files, 166 lines, 5,419 characters
