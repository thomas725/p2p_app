# Codebase Metrics

## Summary

| Metric                |               Value |
|:----------------------|:--------------------|
| Total Rust Files      |                  46 |
| Total Rust Lines      |              13,426 |
| Total Rust Chars      |             486,708 |
| Avg Lines/Rust File   |                 291 |
| Avg Chars/Rust File   |              10,580 |
| Covered Rust Lines    | 2,592 / 4,516 (57%) |
| Total Dart Files      |                  21 |
| Total Dart Lines      |               6,141 |
| Total Dart Chars      |             188,602 |
| Avg Lines/Dart File   |                 292 |
| Avg Chars/Dart File   |               8,981 |
| Covered Dart Lines    |    193 / 2,250 (9%) |
| Total Kotlin Files    |                   2 |
| Total Kotlin Lines    |                 201 |
| Total Kotlin Chars    |               6,701 |
| Avg Lines/Kotlin File |                 100 |
| Avg Chars/Kotlin File |               3,350 |

**Grand Total:** 69 files, 19,768 lines, 682,011 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                        |
| src                     | behavior.rs          |     6 |  7262 |   204 |       38 |  94.74% | Network behavior definitions        |
| src                     | chat.rs              |     5 |  6343 |   176 |       89 |  83.15% | Source file                         |
| src                     | connected.rs         |     3 |  2324 |    71 |       18 | 100.00% | Source file                         |
| src                     | db.rs                |     5 | 23496 |   604 |       88 |  92.05% | Database connection & identity mgmt |
| src                     | fmt.rs               |     4 |  3906 |   129 |       43 |  97.67% | Formatting & display utilities      |
| src                     | frb_generated.rs     |     6 | 76082 |  1882 |      834 |   0.00% | flutter_rust_bridge codegen         |
| src                     | groups.rs            |     3 | 11112 |   314 |      112 |  99.11% | Source file                         |
| src                     | groups.rs            |     4 |  3824 |   102 |       52 |   0.00% | Source file                         |
| src                     | lib.rs               |     1 |  6058 |   159 |        1 | 100.00% | Module declarations & re-exports    |
| src                     | log.rs               |     2 |   907 |    31 |       10 | 100.00% | Source file                         |
| src                     | logging.rs           |     4 | 11635 |   359 |      110 |  91.82% | Logging utilities & setup           |
| src                     | messages.rs          |     5 | 12171 |   351 |      101 |  90.10% | Message persistence & retrieval     |
| src                     | mobile_api.rs        |     4 | 15629 |   469 |       37 | 100.00% | Mobile FRB API surface              |
| src                     | mobile_node.rs       |     5 | 43881 |  1265 |      242 |  38.43% | Mobile node lifecycle & swarm       |
| src                     | mod.rs               |     1 |  6281 |   194 |       40 |   0.00% | Module declarations                 |
| src                     | mod.rs               |     4 |  6782 |   194 |       90 |  91.11% | Module declarations                 |
| src                     | network.rs           |     3 |  2203 |    69 |       13 | 100.00% | Network size classification         |
| src                     | nickname.rs          |     5 | 11658 |   310 |      125 |  99.20% | Nickname management                 |
| src                     | peer_info.rs         |     3 |  2088 |    63 |       27 |   0.00% | Source file                         |
| src                     | peers.rs             |     4 |  8137 |   234 |       73 |  95.89% | Peer management & tracking          |
| src                     | peers.rs             |     4 |  3486 |    89 |       40 | 100.00% | Peer management & tracking          |
| src                     | settings.rs          |     5 |  3453 |    86 |       45 |  91.11% | Source file                         |
| src                     | swarm_handler.rs     |     7 | 14316 |   395 |      176 |  59.66% | Network event translation           |
| src                     | tui_helpers.rs       |     5 | 15055 |   470 |      171 |  98.83% | TUI helper functions & utilities    |
| src                     | tui_render_state.rs  |     4 | 17443 |   495 |      155 |  94.84% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |     6 | 12037 |   351 |      133 |  97.74% | Tab management & navigation         |
| src                     | types.rs             |     2 |  4928 |   158 |        2 | 100.00% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |     7 |  4958 |   134 |       62 |   0.00% | CLI chat application                |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4836 |   124 |       20 |   0.00% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |     7 | 18021 |   448 |      238 |  61.76% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |     6 | 14559 |   423 |      215 |  82.79% | Event routing & state updates       |
| src/bin/tui             | event_source.rs      |     4 |  1258 |    41 |       12 |  41.67% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |     5 | 23642 |   619 |      306 |  68.63% | Input event routing & processing    |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |       73 |   0.00% | Source file                         |
| src/bin/tui             | main_loop.rs         |     4 | 14040 |   372 |      169 |  23.67% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |     5 |  7717 |   229 |      121 |  63.64% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |     5 | 12692 |   349 |      160 |  82.50% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |     5 | 12930 |   316 |       74 |  86.49% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |     3 |  1724 |    50 |       23 |   0.00% | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |     6 |  9593 |   284 |      104 |  53.85% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |     1 |  2951 |    71 |        0 |       - | Auto-generated column definitions   |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                 |
| src/generated           | models_insertable.rs |     1 |  4824 |   139 |        0 |       - | Insertable data models              |
| src/generated           | models_queryable.rs  |     1 |  5928 |   169 |        0 |       - | Queryable data models               |
| src/generated           | schema.rs            |     2 |  3804 |   144 |       74 |  87.84% | Database schema (Diesel)            |

**Total:** 46 files, 13,426 lines, 486,708 characters (2592/4516 testable lines covered, 57%)

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   251 |  7367 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   130 |  4115 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   218 |  6210 |     5 | behavior module tests                 |
| tests  | db.rs                             |   281 |  8650 |     4 | database module tests                 |
| tests  | db_selection.rs                   |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   262 |  6939 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   296 |  8311 |     3 | logging module tests                  |
| tests  | messages.rs                       |   477 | 14302 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17422 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1033 | 36651 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   257 |  7305 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   542 | 18172 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    49 |  1818 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   292 |  8985 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23473 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   496 | 15742 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   875 | 28336 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8301 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   646 | 20130 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   438 | 15793 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   908 | 29970 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   992 | 34077 |     4 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   248 |  8189 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   232 |  7204 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   600 | 19274 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   228 |  6532 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    61 |  1855 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Test file                             |
| unit   | unit_db.rs                        |   273 |  8621 |     3 | Unit tests for database module        |
| unit   | unit_groups.rs                    |   221 |  7650 |     5 | Test file                             |
| unit   | unit_lib.rs                       |   249 |  6836 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   204 |  7532 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   147 |  4848 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |    63 |  1805 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |   104 |  3173 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   477 | 16450 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   331 |  9991 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   320 |  9819 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   326 |  9276 |     3 | Unit tests for types module           |

**Total:** 51 test files, 15,866 lines, 506,148 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |     5 |  1560 |    62 |       22 |  31.82% | Flutter app entry point          |
| lib/src/rust | api.dart               |     2 |  6082 |   174 |       36 |   5.56% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 59331 |  1903 |      765 |   0.26% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 |  9248 |   372 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 |  9168 |   372 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        8 |   0.00% | Dart source file                 |
| lib/src/rust | mobile_api.dart        |     5 |  6554 |   200 |       65 |   1.54% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 |  6797 |   238 |       99 |   2.02% | Mobile node bindings (generated) |
| lib          | dm_chat.dart           |    17 | 11855 |   373 |      185 |   0.00% | Dart source file                 |
| lib          | group_chat.dart        |    17 | 12384 |   380 |      190 |   0.00% | Dart source file                 |
| lib          | group_list.dart        |    12 |  5670 |   175 |       74 |  98.65% | Dart source file                 |
| lib          | home.dart              |    10 | 18066 |   628 |      330 |   8.79% | Dart source file                 |
| lib          | log_tab.dart           |    10 |  2662 |    97 |       38 |   2.63% | Dart source file                 |
| lib          | messages.dart          |    16 | 10818 |   303 |      117 |   0.00% | Dart source file                 |
| lib          | nav_bar.dart           |     3 |  1246 |    34 |        5 |   0.00% | Dart source file                 |
| lib          | peer_info.dart         |    11 |  5640 |   172 |       63 |   0.00% | Dart source file                 |
| lib          | peer_list.dart         |    18 |  6134 |   178 |       77 |  89.61% | Dart source file                 |
| lib          | settings.dart          |    13 | 11922 |   382 |      153 |   0.65% | Dart source file                 |
| lib          | env.dart               |     2 |   624 |    17 |        2 |  50.00% | Dart source file                 |
| lib          | event_bus.dart         |     3 |  1008 |    32 |        8 |  12.50% | Dart source file                 |
| lib          | formats.dart           |     1 |   766 |    17 |        8 |  50.00% | Dart source file                 |

**Total:** 21 files, 6,141 lines, 188,602 characters (193/2250 testable lines covered, 9%)

## Dart Test Files

| Folder  | File                           | Lines | Chars | Depth | Description                           |
|:--------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | test_helpers.dart              |    46 |  1422 |     4 | Test utilities & helpers              |
| unit    | api_test.dart                  |    54 |  2171 |     5 | Dart API layer unit tests             |
| widget  | group_list_test.dart           |   130 |  4147 |     5 | Dart test file                        |
| widget  | peer_list_test.dart            |   289 |  9869 |     5 | Dart test file                        |
| test    | widget_test.dart               |    19 |   615 |     2 | Widget smoke tests                    |

**Total:** 5 test files, 538 lines, 18,224 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  2879 |    82 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  3822 |   119 |        - |       - | Foreground service for background networking |

**Total:** 2 files, 201 lines, 6,701 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | MainActivityTest.kt            |    67 |  2326 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 2 test files, 166 lines, 5,419 characters
