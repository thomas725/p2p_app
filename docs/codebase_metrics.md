# Codebase Metrics

## Summary

| Metric                |   Value |
|:----------------------|:--------|
| Total Rust Files      |      53 |
| Total Rust Lines      |  22,911 |
| Total Rust Chars      | 850,892 |
| Avg Lines/Rust File   |     432 |
| Avg Chars/Rust File   |  16,054 |
| Total Dart Files      |      27 |
| Total Dart Lines      |  11,206 |
| Total Dart Chars      | 351,566 |
| Avg Lines/Dart File   |     415 |
| Avg Chars/Dart File   |  13,020 |
| Total Kotlin Files    |       2 |
| Total Kotlin Lines    |     209 |
| Total Kotlin Chars    |   7,161 |
| Avg Lines/Kotlin File |     104 |
| Avg Chars/Kotlin File |   3,580 |

**Grand Total:** 82 files, 34,326 lines, 1,209,619 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        - |       - | Build script                           |
| src                     | behavior.rs          |     6 | 17974 |   449 |        - |       - | Network behavior definitions           |
| src                     | calls.rs             |     3 |  3204 |    90 |        - |       - | Call session state machine (signaling) |
| src                     | chat.rs              |     5 |  6015 |   169 |        - |       - | TUI chat & DM pane renderer            |
| src                     | connected.rs         |     3 |  2324 |    71 |        - |       - | Connected peers tracking & monitoring  |
| src                     | contacts.rs          |     4 | 14519 |   391 |        - |       - | Canonical contacts-roster model        |
| src                     | db.rs                |     5 | 24411 |   628 |        - |       - | Database connection & identity mgmt    |
| src                     | files.rs             |     6 | 25746 |   700 |        - |       - | Content-addressed attachment store     |
| src                     | fmt.rs               |     4 |  3964 |   128 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     6 | 128798 |  3132 |        - |       - | flutter_rust_bridge codegen            |
| src                     | groups.rs            |     3 | 16634 |   459 |        - |       - | Group management & persistence         |
| src                     | groups.rs            |     4 |  3829 |   102 |        - |       - | TUI groups renderer                    |
| src                     | identity.rs          |     4 | 20512 |   541 |        - |       - | Users, account keys & device bindings  |
| src                     | lib.rs               |     1 |  6590 |   172 |        - |       - | Module declarations & re-exports       |
| src                     | log.rs               |     2 |   907 |    31 |        - |       - | TUI log renderer                       |
| src                     | logging.rs           |     8 | 13383 |   407 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     5 | 13194 |   366 |        - |       - | Message persistence & retrieval        |
| src                     | methods.rs           |     5 | 15146 |   493 |        - |       - | Web JSON method dispatch               |
| src                     | mobile_api.rs        |     4 | 33277 |   971 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 70068 |  1933 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     4 | 16323 |   483 |        - |       - | FRB API surface                        |
| src                     | mod.rs               |     4 |  7176 |   203 |        - |       - | TUI render chrome & dispatch           |
| src                     | mod.rs               |     7 | 35969 |   963 |        - |       - | Web server (axum HTTP+WS)              |
| src                     | network.rs           |     3 |  1979 |    67 |        - |       - | Network size classification            |
| src                     | nickname.rs          |     5 | 12371 |   325 |        - |       - | Nickname management                    |
| src                     | peer_info.rs         |     3 |  2563 |    77 |        - |       - | TUI peer-info renderer                 |
| src                     | peers.rs             |     4 |  9500 |   264 |        - |       - | Peer management & tracking             |
| src                     | peers.rs             |     4 |  4053 |    95 |        - |       - | TUI peers-table renderer               |
| src                     | settings.rs          |     4 |  3823 |    95 |        - |       - | TUI settings renderer                  |
| src                     | swarm_handler.rs     |     8 | 48376 |  1285 |        - |       - | Network event translation              |
| src                     | tui_helpers.rs       |     5 | 21381 |   654 |        - |       - | TUI helper functions & utilities       |
| src                     | tui_render_state.rs  |     4 | 21582 |   600 |        - |       - | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     6 | 11922 |   348 |        - |       - | Tab management & navigation            |
| src                     | types.rs             |     2 | 11215 |   308 |        - |       - | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  4996 |   135 |        - |       - | CLI chat application                   |
| src/bin                 | p2p_chat_server.rs   |     5 |  4899 |   136 |        - |       - | Headless node + web server entry point |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4836 |   124 |        - |       - | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     7 | 19604 |   474 |        - |       - | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 21211 |   573 |        - |       - | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |        - |       - | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     6 | 40044 |  1056 |        - |       - | Input event routing & processing       |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |        - |       - | Keyboard enhancement probe             |
| src/bin/tui             | main_loop.rs         |     4 | 17144 |   446 |        - |       - | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     4 | 10837 |   345 |        - |       - | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 | 16078 |   422 |        - |       - | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     8 | 30199 |   716 |        - |       - | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     5 |  2110 |    61 |        - |       - | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     6 | 12310 |   351 |        - |       - | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  3090 |    74 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  7402 |   206 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 |  9088 |   248 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  6355 |   214 |        - |       - | Database schema (Diesel)               |

**Total:** 53 files, 22,911 lines, 850,892 characters

## Rust Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs             |    18 |   877 |     2 | Logging test helpers                  |
| shared | tui_test_state.rs                 |   251 |  7394 |     6 | TUI test state builders               |
| tests  | additional_coverage.rs            |   137 |  4295 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   229 |  6488 |     5 | behavior module tests                 |
| tests  | db.rs                             |   281 |  8650 |     4 | database module tests                 |
| tests  | db_selection.rs                   |    65 |  1771 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   262 |  6939 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   335 |  9678 |     3 | logging module tests                  |
| tests  | messages.rs                       |   483 | 14619 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17422 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1082 | 38457 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   257 |  7305 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   595 | 19963 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   304 |  9402 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23494 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   495 | 15709 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |  1115 | 37965 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8313 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   653 | 20345 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    85 |  2613 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   535 | 19778 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |  1164 | 39332 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |  1991 | 66823 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   249 |  7558 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   264 |  8274 |     4 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   358 | 12352 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   657 | 21507 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   422 | 13210 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    60 |  2017 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_calls.rs                     |    90 |  2802 |     1 | calls module tests                    |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_contacts.rs                  |   313 | 10517 |     4 | contacts module tests                 |
| unit   | unit_db.rs                        |   273 |  8621 |     3 | Unit tests for database module        |
| unit   | unit_files.rs                     |   399 | 14777 |     4 | files module tests                    |
| unit   | unit_groups.rs                    |   399 | 14179 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                  |   320 | 11965 |     4 | identity module tests                 |
| unit   | unit_lib.rs                       |   252 |  6912 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5222 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   357 | 13153 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   173 |  5796 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |   111 |  3442 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   365 | 11173 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   323 |  9859 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3079 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   342 |  9837 |     3 | Unit tests for types module           |

**Total:** 55 test files, 19,863 lines, 646,872 characters

## Dart Source Files

| Folder       | File                   | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:-----------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart              |     5 |  2107 |    73 |        - |       - | Flutter app entry point          |
| lib          | chat_message_ext.dart  |     2 |   912 |    26 |        - |       - | Hand-written message helpers     |
| lib          | frb_node_api.dart      |     5 |  8793 |   299 |        - |       - | In-process FRB transport         |
| lib          | http_node_api.dart     |     5 | 20760 |   637 |        - |       - | WebSocket/HTTP transport         |
| lib          | node_api.dart          |     2 |  6196 |   197 |        - |       - | Transport-agnostic node API      |
| lib          | node_transport.dart    |     4 |  3565 |   117 |        - |       - | Duplex transport abstraction     |
| lib/src/rust | api.dart               |     3 | 13318 |   379 |        - |       - | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart     |     6 | 100559 |  3210 |        - |       - | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart  |     3 | 13864 |   567 |        - |       - | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart |     2 | 13784 |   567 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart          |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart        |     5 | 14457 |   446 |        - |       - | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart       |     5 | 10673 |   337 |        - |       - | Mobile node bindings (generated) |
| lib          | call_screen.dart       |     8 |  9678 |   323 |        - |       - | Call overlay screen              |
| lib          | contact_list.dart      |    15 | 13574 |   398 |        - |       - | Contacts roster screen           |
| lib          | dm_chat.dart           |     9 |  8088 |   256 |        - |       - | Direct-message chat screen       |
| lib          | group_chat.dart        |     8 |  8457 |   263 |        - |       - | Group chat screen                |
| lib          | group_list.dart        |    17 |  7931 |   235 |        - |       - | Group list screen                |
| lib          | home.dart              |     7 | 32496 |   998 |        - |       - | App shell & broadcast chat       |
| lib          | log_tab.dart           |    10 |  3493 |   123 |        - |       - | Log tab screen                   |
| lib          | messages.dart          |    14 | 23573 |   728 |        - |       - | Shared chat widgets              |
| lib          | nav_bar.dart           |     3 |  1252 |    34 |        - |       - | Tab navigation bar               |
| lib          | peer_info.dart         |    11 | 11858 |   350 |        - |       - | Peer Info screen                 |
| lib          | settings.dart          |    16 | 14067 |   436 |        - |       - | Settings screen                  |
| lib          | attachments.dart       |     4 |  3656 |    92 |        - |       - | File picker & opener seams       |
| lib          | event_bus.dart         |     1 |  1483 |    38 |        - |       - | Node event fan-out               |
| lib          | ui_utils.dart          |     2 |  1905 |    45 |        - |       - | Shared formatting & env helpers  |

**Total:** 27 files, 11,206 lines, 351,566 characters

## Dart Test Files

| Folder  | File                           | Lines | Chars | Depth | Description                           |
|:--------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart             |   312 |  9362 |     4 | In-memory node fake                   |
| unit    | api_test.dart                  |    53 |  2156 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart          |   127 |  3822 |     5 | Attachment helper tests               |
| unit    | http_node_api_test.dart        |   569 | 19213 |     8 | Web transport wire-contract tests     |
| widget  | attachment_chip_test.dart      |    66 |  2011 |     9 | Attachment chip widget tests          |
| widget  | call_screen_test.dart          |   311 | 10216 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart         |   584 | 20382 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart       |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart           |   233 |  7128 |     5 | Group list widget tests               |
| widget  | messages_test.dart             |   229 |  7005 |     9 | Chat widget tests                     |
| widget  | peer_info_test.dart            |   165 |  5263 |     6 | Peer Info widget tests                |
| widget  | settings_test.dart             |    78 |  2272 |     3 | Settings widget tests                 |
| test    | widget_test.dart               |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 13 test files, 2,820 lines, 91,444 characters

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
