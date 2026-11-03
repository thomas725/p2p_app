# Codebase Metrics

## Summary

| Metric                |               Value |
|:----------------------|:--------------------|
| Total Rust Files      |                  55 |
| Total Rust Lines      |              25,244 |
| Total Rust Chars      |             941,142 |
| Avg Lines/Rust File   |                 458 |
| Avg Chars/Rust File   |              17,111 |
| Covered Rust Lines    | 4,613 / 8,240 (56%) |
| Total Dart Files      |                  28 |
| Total Dart Lines      |              11,978 |
| Total Dart Chars      |             377,433 |
| Avg Lines/Dart File   |                 427 |
| Avg Chars/Dart File   |              13,479 |
| Covered Dart Lines    |   969 / 4,148 (23%) |
| Total Kotlin Files    |                   2 |
| Total Kotlin Lines    |                 209 |
| Total Kotlin Chars    |               7,161 |
| Avg Lines/Kotlin File |                 104 |
| Avg Chars/Kotlin File |               3,580 |

**Grand Total:** 85 files, 37,431 lines, 1,325,736 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                           |
| src                     | behavior.rs          |     6 | 21368 |   531 |       67 |  97.01% | Network behavior definitions           |
| src                     | calls.rs             |     3 |  3204 |    90 |       16 | 100.00% | Call session state machine (signaling) |
| src                     | chat.rs              |     5 |  6015 |   169 |       85 |  83.53% | TUI chat & DM pane renderer            |
| src                     | connected.rs         |     3 |  2324 |    71 |       18 | 100.00% | Connected peers tracking & monitoring  |
| src                     | contacts.rs          |     4 | 14519 |   391 |      125 |  97.60% | Canonical contacts-roster model        |
| src                     | db.rs                |     5 | 25109 |   647 |      101 |  93.07% | Database connection & identity mgmt    |
| src                     | e2e.rs               |     4 | 17127 |   435 |      123 |  84.55% | Source file                            |
| src                     | files.rs             |     6 | 25734 |   702 |      264 |  93.56% | Content-addressed attachment store     |
| src                     | fmt.rs               |     4 |  3964 |   128 |       46 |  97.83% | Formatting & display utilities         |
| src                     | frb_generated.rs     |     6 | 142537 |  3433 |     1616 |   0.00% | flutter_rust_bridge codegen            |
| src                     | groups.rs            |     3 | 16634 |   459 |      159 |  95.60% | Group management & persistence         |
| src                     | groups.rs            |     4 |  3829 |   102 |       52 |   0.00% | TUI groups renderer                    |
| src                     | identity.rs          |     4 | 22668 |   601 |      220 |  97.73% | Users, account keys & device bindings  |
| src                     | lib.rs               |     1 |  6771 |   178 |        1 |   0.00% | Module declarations & re-exports       |
| src                     | log.rs               |     2 |   907 |    31 |       10 | 100.00% | TUI log renderer                       |
| src                     | logging.rs           |     8 | 13388 |   407 |      122 |  97.54% | Logging utilities & setup              |
| src                     | messages.rs          |     5 | 13384 |   370 |      106 | 100.00% | Message persistence & retrieval        |
| src                     | methods.rs           |     5 | 15493 |   506 |      167 |  62.28% | Web JSON method dispatch               |
| src                     | mobile_api.rs        |     4 | 33732 |   989 |       92 |  71.74% | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 72528 |  2003 |      423 |  33.33% | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     4 | 16928 |   500 |       82 |  14.63% | FRB API surface                        |
| src                     | mod.rs               |     4 |  7291 |   206 |      101 |  92.08% | TUI render chrome & dispatch           |
| src                     | mod.rs               |     7 | 48662 |  1283 |      173 |  78.61% | Web server (axum HTTP+WS)              |
| src                     | net_diag.rs          |     4 | 15732 |   448 |      165 |  98.18% | Source file                            |
| src                     | network.rs           |     3 |  1979 |    67 |       14 | 100.00% | Network size classification            |
| src                     | nickname.rs          |     5 | 12371 |   325 |      130 |  99.23% | Nickname management                    |
| src                     | peer_info.rs         |     3 |  2563 |    77 |       32 |  90.62% | TUI peer-info renderer                 |
| src                     | peers.rs             |     4 | 10590 |   294 |      101 | 100.00% | Peer management & tracking             |
| src                     | peers.rs             |     4 |  4053 |    95 |       42 | 100.00% | TUI peers-table renderer               |
| src                     | settings.rs          |     4 |  5923 |   156 |       83 |  79.52% | TUI settings renderer                  |
| src                     | swarm_handler.rs     |     8 | 58206 |  1530 |      635 |  19.06% | Network event translation              |
| src                     | tui_helpers.rs       |     5 | 21381 |   654 |      232 |  96.12% | TUI helper functions & utilities       |
| src                     | tui_render_state.rs  |     4 | 21915 |   607 |      198 |  94.95% | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     6 | 11922 |   348 |      133 |  97.74% | Tab management & navigation            |
| src                     | types.rs             |     2 | 12190 |   332 |        5 |  80.00% | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  4996 |   135 |       62 |   0.00% | CLI chat application                   |
| src/bin                 | p2p_chat_server.rs   |     5 |  6828 |   182 |       70 |   0.00% | Headless node + web server entry point |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4868 |   125 |       20 |   0.00% | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     7 | 19604 |   474 |      243 |  62.14% | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 22078 |   597 |      286 |  78.32% | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |       12 |  41.67% | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     6 | 41570 |  1102 |      517 |  76.21% | Input event routing & processing       |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |       73 |   0.00% | Keyboard enhancement probe             |
| src/bin/tui             | main_loop.rs         |     4 | 17144 |   446 |      161 |  24.84% | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     4 | 11418 |   364 |      167 |  61.08% | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 | 16078 |   422 |      182 |  84.07% | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     8 | 30825 |   735 |      238 |  92.02% | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     5 |  2276 |    67 |       33 |   0.00% | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     6 | 12358 |   352 |      131 |  62.60% | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  3132 |    75 |        0 |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  7497 |   208 |        0 |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  9183 |   250 |        0 |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  6385 |   215 |      106 |  92.45% | Database schema (Diesel)               |

**Total:** 55 files, 25,244 lines, 941,142 characters (4613/8240 testable lines covered, 56%)

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   157 |  4819 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    18 |   877 |     2 | Logging test helpers                  |
| shared | tui_test_state.rs                 |   251 |  7394 |     6 | TUI test state builders               |
| tests  | additional_coverage.rs            |   140 |  4372 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   245 |  7060 |     5 | behavior module tests                 |
| tests  | db.rs                             |   281 |  8625 |     4 | database module tests                 |
| tests  | db_selection.rs                   |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   262 |  6901 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   335 |  9679 |     3 | logging module tests                  |
| tests  | messages.rs                       |   485 | 14675 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17397 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1082 | 38457 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   317 |  9240 |     4 | peers module tests                    |
| tests  | swarm_handler.rs                  |   657 | 22087 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   309 |  9533 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23504 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   495 | 15705 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |  1115 | 37961 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8313 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   656 | 20418 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    98 |  2942 |     3 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   536 | 19800 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |  1189 | 40025 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |  1991 | 66811 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   295 |  9119 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   354 | 12314 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   657 | 21507 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   423 | 13228 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    60 |  2017 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_calls.rs                     |    90 |  2797 |     1 | calls module tests                    |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_contacts.rs                  |   357 | 12111 |     4 | contacts module tests                 |
| unit   | unit_db.rs                        |   273 |  8611 |     3 | Unit tests for database module        |
| unit   | unit_e2e.rs                       |   207 |  7559 |     3 | Test file                             |
| unit   | unit_files.rs                     |   400 | 14799 |     4 | files module tests                    |
| unit   | unit_groups.rs                    |   399 | 14179 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                  |   516 | 19595 |     4 | identity module tests                 |
| unit   | unit_lib.rs                       |   253 |  6932 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5217 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   358 | 13183 |     4 | Unit tests for messages module        |
| unit   | unit_net_diag.rs                  |   358 | 11555 |     3 | Test file                             |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   173 |  5791 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |   113 |  3510 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   365 | 11174 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   323 |  9859 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3105 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   348 | 10019 |     3 | Unit tests for types module           |

**Total:** 57 test files, 20,898 lines, 682,296 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |     5 |  2205 |    74 |       23 |  30.43% | Flutter app entry point          |
| lib          | chat_message_ext.dart  |     2 |   938 |    27 |       13 |   0.00% | Hand-written message helpers     |
| lib          | frb_node_api.dart      |     5 |  9047 |   307 |       92 |   5.43% | In-process FRB transport         |
| lib          | http_node_api.dart     |     5 | 22973 |   690 |      322 |  63.66% | WebSocket/HTTP transport         |
| lib          | node_api.dart          |     2 |  6573 |   205 |        1 | 100.00% | Transport-agnostic node API      |
| lib          | node_transport.dart    |     4 |  3396 |   106 |       27 |   0.00% | Duplex transport abstraction     |
| lib/src/rust | api.dart               |     3 | 13932 |   394 |       72 |   5.56% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 112229 |  3529 |     1456 |   0.14% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 | 15523 |   637 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 | 15443 |   637 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart        |     5 | 14480 |   446 |      152 |   4.61% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 | 10986 |   344 |      130 |   3.85% | Mobile node bindings (generated) |
| lib/src/rust | net_diag.dart          |     5 |  6025 |   193 |       62 |   0.00% | Dart source file                 |
| lib          | call_screen.dart       |     8 |  9678 |   323 |      101 |  89.11% | Call overlay screen              |
| lib          | contact_list.dart      |    15 | 13574 |   398 |      171 |  95.91% | Contacts roster screen           |
| lib          | dm_chat.dart           |     9 |  8088 |   256 |      127 |   0.00% | Direct-message chat screen       |
| lib          | group_chat.dart        |     8 |  8551 |   265 |      129 |   0.00% | Group chat screen                |
| lib          | group_list.dart        |    17 |  7931 |   235 |       99 |  98.99% | Group list screen                |
| lib          | home.dart              |     7 | 32496 |   998 |      465 |   9.03% | App shell & broadcast chat       |
| lib          | log_tab.dart           |    10 |  3493 |   123 |       51 |   1.96% | Log tab screen                   |
| lib          | messages.dart          |    14 | 23573 |   728 |      276 |  27.90% | Shared chat widgets              |
| lib          | nav_bar.dart           |     3 |  1252 |    34 |        5 | 100.00% | Tab navigation bar               |
| lib          | peer_info.dart         |    11 | 11858 |   350 |      134 |  90.30% | Peer Info screen                 |
| lib          | settings.dart          |    16 | 15380 |   480 |      194 |  65.46% | Settings screen                  |
| lib          | attachments.dart       |     4 |  3656 |    92 |       25 |   0.00% | File picker & opener seams       |
| lib          | event_bus.dart         |     1 |  1483 |    38 |        6 |  33.33% | Node event fan-out               |
| lib          | ui_utils.dart          |     2 |  1603 |    37 |       10 |  60.00% | Shared formatting & env helpers  |

**Total:** 28 files, 11,978 lines, 377,433 characters (969/4148 testable lines covered, 23%)

## Dart Test Files

| Folder  | File                           | Lines | Chars | Depth | Description                           |
|:--------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart             |   315 |  9435 |     4 | In-memory node fake                   |
| unit    | api_test.dart                  |    53 |  2156 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart          |   129 |  3868 |     5 | Attachment helper tests               |
| unit    | http_node_api_test.dart        |   569 | 19213 |     8 | Web transport wire-contract tests     |
| widget  | attachment_chip_test.dart      |    67 |  2047 |     9 | Attachment chip widget tests          |
| widget  | call_screen_test.dart          |   311 | 10216 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart         |   584 | 20382 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart       |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart           |   233 |  7128 |     5 | Group list widget tests               |
| widget  | messages_test.dart             |   231 |  7055 |     9 | Chat widget tests                     |
| widget  | peer_info_test.dart            |   165 |  5263 |     6 | Peer Info widget tests                |
| widget  | settings_test.dart             |    78 |  2272 |     3 | Settings widget tests                 |
| test    | widget_test.dart               |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 13 test files, 2,828 lines, 91,649 characters

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
