# Codebase Metrics

## Summary

| Metric                |               Value |
|:----------------------|:--------------------|
| Total Rust Files      |                  59 |
| Total Rust Lines      |              28,973 |
| Total Rust Chars      |           1,088,919 |
| Avg Lines/Rust File   |                 491 |
| Avg Chars/Rust File   |              18,456 |
| Covered Rust Lines    | 5,527 / 9,530 (58%) |
| Total Dart Files      |                  32 |
| Total Dart Lines      |              14,419 |
| Total Dart Chars      |             459,465 |
| Avg Lines/Dart File   |                 450 |
| Avg Chars/Dart File   |              14,358 |
| Covered Dart Lines    | 1,896 / 4,982 (38%) |
| Total Kotlin Files    |                   2 |
| Total Kotlin Lines    |                 209 |
| Total Kotlin Chars    |               7,161 |
| Avg Lines/Kotlin File |                 104 |
| Avg Chars/Kotlin File |               3,580 |

**Grand Total:** 93 files, 43,601 lines, 1,555,545 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                           |
| src                     | behavior.rs          |     6 | 22799 |   559 |       75 |  97.33% | Network behavior definitions           |
| src                     | blocklist.rs         |     2 |  3876 |   104 |       35 | 100.00% | Source file                            |
| src                     | calls.rs             |     3 |  3221 |    90 |       16 | 100.00% | Call session state machine (signaling) |
| src                     | chat.rs              |     5 |  6015 |   169 |       85 |  83.53% | TUI chat & DM pane renderer            |
| src                     | connected.rs         |     3 |  2324 |    71 |       18 | 100.00% | Connected peers tracking & monitoring  |
| src                     | contacts.rs          |     4 | 14829 |   395 |      125 |  97.60% | Canonical contacts-roster model        |
| src                     | db.rs                |     5 | 25331 |   652 |      101 |  93.07% | Database connection & identity mgmt    |
| src                     | e2e.rs               |     4 | 17503 |   438 |      131 |  93.13% | Source file                            |
| src                     | files.rs             |     6 | 25734 |   702 |      264 |  93.56% | Content-addressed attachment store     |
| src                     | fmt.rs               |     4 |  5203 |   164 |       60 |  98.33% | Formatting & display utilities         |
| src                     | frb_generated.rs     |     6 | 165005 |  3965 |     1884 |   0.00% | flutter_rust_bridge codegen            |
| src                     | groups.rs            |     3 | 18762 |   508 |      174 |  95.98% | Group management & persistence         |
| src                     | groups.rs            |     4 |  3829 |   102 |       52 |   0.00% | TUI groups renderer                    |
| src                     | identity.rs          |     4 | 22668 |   601 |      220 |  97.73% | Users, account keys & device bindings  |
| src                     | lib.rs               |     1 |  7257 |   190 |        1 | 100.00% | Module declarations & re-exports       |
| src                     | log.rs               |     2 |   907 |    31 |       10 | 100.00% | TUI log renderer                       |
| src                     | logging.rs           |     8 | 13626 |   411 |      119 |  97.48% | Logging utilities & setup              |
| src                     | messages.rs          |     5 | 15008 |   405 |      119 | 100.00% | Message persistence & retrieval        |
| src                     | methods.rs           |     5 | 18754 |   603 |      209 |  66.03% | Web JSON method dispatch               |
| src                     | mobile_api.rs        |     4 | 36782 |  1049 |       92 |  71.74% | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 81592 |  2216 |      463 |  32.83% | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     4 | 21763 |   630 |      107 |  19.63% | FRB API surface                        |
| src                     | mod.rs               |     4 | 10108 |   266 |      115 |  90.43% | TUI render chrome & dispatch           |
| src                     | mod.rs               |     7 | 54977 |  1443 |      173 |  78.61% | Web server (axum HTTP+WS)              |
| src                     | net_diag.rs          |     4 | 21711 |   605 |      214 |  98.60% | Source file                            |
| src                     | network.rs           |     3 |  1979 |    67 |       14 | 100.00% | Network size classification            |
| src                     | network_config.rs    |     3 |  7681 |   214 |       68 |  98.53% | Source file                            |
| src                     | nickname.rs          |     5 | 12519 |   328 |      130 |  99.23% | Nickname management                    |
| src                     | peer_info.rs         |     3 |  3104 |    88 |       39 |  92.31% | TUI peer-info renderer                 |
| src                     | peers.rs             |     4 | 10590 |   294 |      101 | 100.00% | Peer management & tracking             |
| src                     | peers.rs             |     5 |  5259 |   126 |       57 | 100.00% | TUI peers-table renderer               |
| src                     | relay.rs             |     5 |  3823 |   100 |       29 | 100.00% | Source file                            |
| src                     | reply.rs             |     3 |  3626 |   103 |       27 | 100.00% | Source file                            |
| src                     | settings.rs          |     4 |  7218 |   192 |       99 |  98.99% | TUI settings renderer                  |
| src                     | swarm_handler.rs     |     8 | 75740 |  2001 |      798 |  23.81% | Network event translation              |
| src                     | tui_helpers.rs       |     5 | 21932 |   663 |      232 |  96.12% | TUI helper functions & utilities       |
| src                     | tui_render_state.rs  |     4 | 22929 |   627 |      201 |  95.02% | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     6 | 11922 |   348 |      133 |  97.74% | Tab management & navigation            |
| src                     | types.rs             |     2 | 13300 |   355 |        6 |  66.67% | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  5032 |   136 |       62 |   0.00% | CLI chat application                   |
| src/bin                 | p2p_chat_server.rs   |     4 |  4836 |   138 |       46 |   0.00% | Headless node + web server entry point |
| src/bin                 | p2p_chat_tui.rs      |     6 |  9911 |   278 |       52 |  25.00% | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     7 | 21195 |   515 |      263 |  71.10% | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 23239 |   645 |      310 |  76.77% | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |       12 |  41.67% | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     6 | 52599 |  1384 |      636 |  79.72% | Input event routing & processing       |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |       73 |   0.00% | Keyboard enhancement probe             |
| src/bin/tui             | main_loop.rs         |     6 | 15574 |   395 |      163 |  22.70% | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     4 | 15971 |   493 |      242 |  89.67% | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 | 16156 |   425 |      184 |  83.15% | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     8 | 46129 |  1099 |      394 |  90.86% | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     5 |  2612 |    76 |       37 |   0.00% | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     6 | 12700 |   343 |      142 |  65.49% | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  3487 |    83 |        0 |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  8378 |   231 |        0 |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 | 10615 |   284 |        0 |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  7318 |   243 |      118 |  93.22% | Database schema (Diesel)               |

**Total:** 59 files, 28,973 lines, 1,088,919 characters (5527/9530 testable lines covered, 58%)

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   157 |  4819 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    18 |   877 |     2 | Logging test helpers                  |
| shared | tui_test_state.rs                 |   250 |  7360 |     6 | TUI test state builders               |
| tests  | additional_coverage.rs            |   146 |  4516 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   252 |  7228 |     5 | behavior module tests                 |
| tests  | db.rs                             |   281 |  8625 |     4 | database module tests                 |
| tests  | db_selection.rs                   |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   281 |  7503 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   335 |  9679 |     3 | logging module tests                  |
| tests  | messages.rs                       |   492 | 14988 |     4 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17397 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1087 | 38661 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   317 |  9240 |     4 | peers module tests                    |
| tests  | swarm_handler.rs                  |   706 | 24041 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   317 |  9747 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23504 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   495 | 15705 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |  1493 | 51771 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8313 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   662 | 20562 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    99 |  2966 |     3 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   625 | 22816 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |  1228 | 41026 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |  2658 | 89982 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   233 |  7007 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   435 | 13918 |     5 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   408 | 14566 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   657 | 21507 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   705 | 22683 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    66 |  2330 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_blocklist.rs                 |    98 |  3435 |     2 | Test file                             |
| unit   | unit_calls.rs                     |    90 |  2797 |     1 | calls module tests                    |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_contacts.rs                  |   357 | 12111 |     4 | contacts module tests                 |
| unit   | unit_db.rs                        |   273 |  8611 |     3 | Unit tests for database module        |
| unit   | unit_e2e.rs                       |   312 | 11457 |     3 | Test file                             |
| unit   | unit_files.rs                     |   401 | 14828 |     4 | files module tests                    |
| unit   | unit_groups.rs                    |   504 | 18172 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                  |   516 | 19595 |     4 | identity module tests                 |
| unit   | unit_lib.rs                       |   255 |  6980 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5217 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   516 | 19266 |     4 | Unit tests for messages module        |
| unit   | unit_net_diag.rs                  |   479 | 15624 |     3 | Test file                             |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_network_config.rs            |   153 |  5151 |     4 | Test file                             |
| unit   | unit_nickname.rs                  |   173 |  5791 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_relay.rs                     |    62 |  1806 |     2 | Test file                             |
| unit   | unit_reply.rs                     |    51 |  1659 |     2 | Test file                             |
| unit   | unit_swarm_handler.rs             |   133 |  4227 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   365 | 11174 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   323 |  9859 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3105 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   361 | 10399 |     3 | Unit tests for types module           |

**Total:** 61 test files, 23,533 lines, 774,563 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |     5 |  2205 |    74 |       23 |  30.43% | Flutter app entry point          |
| lib          | chat_message_ext.dart  |     2 |  1176 |    32 |       15 | 100.00% | Hand-written message helpers     |
| lib          | frb_node_api.dart      |     5 | 10636 |   356 |      109 |   5.50% | In-process FRB transport         |
| lib          | http_node_api.dart     |     6 | 26697 |   802 |      379 |  65.44% | WebSocket/HTTP transport         |
| lib          | node_api.dart          |     2 |  8668 |   255 |        1 | 100.00% | Transport-agnostic node API      |
| lib          | node_transport.dart    |     4 |  3396 |   106 |       27 |   0.00% | Duplex transport abstraction     |
| lib/src/rust | api.dart               |     3 | 17793 |   496 |       90 |   6.67% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 129216 |  4072 |     1689 |   0.12% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 | 17113 |   702 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 | 17033 |   702 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart        |     5 | 16684 |   519 |      152 |   4.61% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 | 14676 |   457 |      140 |   4.29% | Mobile node bindings (generated) |
| lib/src/rust | net_diag.dart          |     5 |  7968 |   256 |       81 |   2.47% | Dart source file                 |
| lib/src/rust | network_config.dart    |     5 |  1182 |    43 |       10 |  10.00% | Dart source file                 |
| lib/src/rust | reply.dart             |     5 |  1313 |    41 |        9 |  77.78% | Dart source file                 |
| lib          | call_screen.dart       |     8 |  9678 |   323 |      101 |  89.11% | Call overlay screen              |
| lib          | contact_list.dart      |    15 | 14857 |   429 |      181 |  99.45% | Contacts roster screen           |
| lib          | dm_chat.dart           |     9 |  9242 |   293 |      147 |  70.75% | Direct-message chat screen       |
| lib          | group_chat.dart        |     8 |  9487 |   302 |      148 |  52.03% | Group chat screen                |
| lib          | group_list.dart        |    17 |  7931 |   235 |       99 |  98.99% | Group list screen                |
| lib          | home.dart              |     7 | 36967 |  1131 |      526 |  35.55% | App shell & broadcast chat       |
| lib          | log_tab.dart           |    10 |  3493 |   123 |       51 |  39.22% | Log tab screen                   |
| lib          | messages.dart          |    14 | 32186 |   961 |      369 |  84.01% | Shared chat widgets              |
| lib          | nav_bar.dart           |     3 |  1252 |    34 |        5 | 100.00% | Tab navigation bar               |
| lib          | peer_info.dart         |    11 | 16379 |   481 |      186 |  91.94% | Peer Info screen                 |
| lib          | settings.dart          |    16 | 30922 |   911 |      367 |  80.93% | Settings screen                  |
| lib          | attachments.dart       |     4 |  3656 |    92 |       25 |  44.00% | File picker & opener seams       |
| lib          | blocklist.dart         |     2 |   755 |    19 |        6 | 100.00% | Dart source file                 |
| lib          | event_bus.dart         |     1 |  1483 |    38 |        6 |  66.67% | Node event fan-out               |
| lib          | reply.dart             |     3 |  2751 |    65 |       25 |  88.00% | Dart source file                 |
| lib          | ui_utils.dart          |     2 |  1603 |    37 |       10 |  60.00% | Shared formatting & env helpers  |

**Total:** 32 files, 14,419 lines, 459,465 characters (1896/4982 testable lines covered, 38%)

## Dart Test Files

| Folder  | File                             | Lines | Chars | Depth | Description                           |
|:--------|:---------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart               |   527 | 15971 |     4 | In-memory node fake                   |
| unit    | api_test.dart                    |    61 |  2389 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart            |   130 |  3905 |     5 | Attachment helper tests               |
| unit    | blocklist_test.dart              |    47 |  1429 |     4 | Dart test file                        |
| unit    | http_node_api_test.dart          |   744 | 24284 |    10 | Web transport wire-contract tests     |
| widget  | attachment_chip_test.dart        |    90 |  2547 |     7 | Attachment chip widget tests          |
| widget  | call_screen_test.dart            |   313 | 10264 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart           |   661 | 22484 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart         |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart             |   233 |  7128 |     5 | Group list widget tests               |
| widget  | messages_test.dart               |   719 | 20859 |    10 | Chat widget tests                     |
| widget  | peer_info_test.dart              |   280 |  8730 |     6 | Peer Info widget tests                |
| widget  | settings_network_peers_test.dart |   128 |  4028 |     3 | Dart test file                        |
| widget  | settings_test.dart               |   215 |  6669 |     5 | Settings widget tests                 |
| test    | widget_test.dart                 |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 15 test files, 4,241 lines, 133,301 characters

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
