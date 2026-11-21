# Codebase Metrics

## Summary

| Metric                |     Value |
|:----------------------|:----------|
| Total Rust Files      |        64 |
| Total Rust Lines      |    33,637 |
| Total Rust Chars      | 1,271,933 |
| Avg Lines/Rust File   |       525 |
| Avg Chars/Rust File   |    19,873 |
| Total Dart Files      |        39 |
| Total Dart Lines      |    17,102 |
| Total Dart Chars      |   550,571 |
| Avg Lines/Dart File   |       438 |
| Avg Chars/Dart File   |    14,117 |
| Total Kotlin Files    |         3 |
| Total Kotlin Lines    |       303 |
| Total Kotlin Chars    |    10,990 |
| Avg Lines/Kotlin File |       101 |
| Avg Chars/Kotlin File |     3,663 |

**Grand Total:** 106 files, 51,042 lines, 1,833,494 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        - |       - | Build script                           |
| src                     | behavior.rs          |     6 | 27519 |   679 |        - |       - | Network behavior definitions           |
| src                     | blocklist.rs         |     2 |  3876 |   104 |        - |       - | Source file                            |
| src                     | calls.rs             |     3 |  3221 |    90 |        - |       - | Call session state machine (signaling) |
| src                     | chat.rs              |     5 |  6324 |   176 |        - |       - | TUI chat & DM pane renderer            |
| src                     | connected.rs         |     3 |  2324 |    71 |        - |       - | Connected peers tracking & monitoring  |
| src                     | contacts.rs          |     4 | 14829 |   395 |        - |       - | Canonical contacts-roster model        |
| src                     | db.rs                |     5 | 25331 |   652 |        - |       - | Database connection & identity mgmt    |
| src                     | dm_outbox.rs         |     3 |  5631 |   155 |        - |       - | Source file                            |
| src                     | e2e.rs               |     4 | 17759 |   443 |        - |       - | Source file                            |
| src                     | files.rs             |     6 | 28869 |   762 |        - |       - | Content-addressed attachment store     |
| src                     | fmt.rs               |     4 |  6204 |   192 |        - |       - | Formatting & display utilities         |
| src                     | frb_generated.rs     |     6 | 193360 |  4652 |        - |       - | flutter_rust_bridge codegen            |
| src                     | groups.rs            |     3 | 22811 |   601 |        - |       - | Group management & persistence         |
| src                     | groups.rs            |     5 |  4661 |   124 |        - |       - | TUI groups renderer                    |
| src                     | identity.rs          |     4 | 28876 |   762 |        - |       - | Users, account keys & device bindings  |
| src                     | lib.rs               |     1 |  7348 |   195 |        - |       - | Module declarations & re-exports       |
| src                     | log.rs               |     2 |   907 |    31 |        - |       - | TUI log renderer                       |
| src                     | logging.rs           |     8 | 13626 |   411 |        - |       - | Logging utilities & setup              |
| src                     | messages.rs          |     5 | 19642 |   512 |        - |       - | Message persistence & retrieval        |
| src                     | methods.rs           |     4 | 21745 |   714 |        - |       - | Web JSON method dispatch               |
| src                     | mobile_api.rs        |     4 | 36782 |  1049 |        - |       - | Mobile FRB API surface                 |
| src                     | mobile_node.rs       |     5 | 91630 |  2455 |        - |       - | Mobile node lifecycle & swarm          |
| src                     | mod.rs               |     4 | 25712 |   741 |        - |       - | FRB API surface                        |
| src                     | mod.rs               |     4 | 10997 |   287 |        - |       - | TUI render chrome & dispatch           |
| src                     | mod.rs               |     7 | 63191 |  1650 |        - |       - | Web server (axum HTTP+WS)              |
| src                     | net_diag.rs          |     4 | 21711 |   605 |        - |       - | Source file                            |
| src                     | network.rs           |     3 |  1979 |    67 |        - |       - | Network size classification            |
| src                     | network_config.rs    |     3 |  7681 |   214 |        - |       - | Source file                            |
| src                     | nickname.rs          |     5 | 12838 |   333 |        - |       - | Nickname management                    |
| src                     | notifications.rs     |     4 |  8213 |   241 |        - |       - | Source file                            |
| src                     | peer_info.rs         |     3 |  3104 |    88 |        - |       - | TUI peer-info renderer                 |
| src                     | peers.rs             |     4 | 10590 |   294 |        - |       - | Peer management & tracking             |
| src                     | peers.rs             |     5 |  5890 |   136 |        - |       - | TUI peers-table renderer               |
| src                     | reactions.rs         |     5 | 15943 |   420 |        - |       - | Source file                            |
| src                     | relay.rs             |     5 |  3965 |   104 |        - |       - | Source file                            |
| src                     | reply.rs             |     3 |  4683 |   129 |        - |       - | Source file                            |
| src                     | settings.rs          |     3 |  5562 |   146 |        - |       - | Source file                            |
| src                     | settings.rs          |     4 |  8302 |   225 |        - |       - | TUI settings renderer                  |
| src                     | swarm_handler.rs     |     7 | 91503 |  2379 |        - |       - | Network event translation              |
| src                     | theme.rs             |     4 |  7223 |   234 |        - |       - | Source file                            |
| src                     | tui_helpers.rs       |     5 | 21932 |   663 |        - |       - | TUI helper functions & utilities       |
| src                     | tui_render_state.rs  |     4 | 25356 |   684 |        - |       - | TUI render state & tab content         |
| src                     | tui_tabs.rs          |     6 | 11711 |   342 |        - |       - | Tab management & navigation            |
| src                     | types.rs             |     2 | 15094 |   394 |        - |       - | Event & command type defs              |
| src/bin                 | p2p_chat.rs          |     7 |  6917 |   192 |        - |       - | CLI chat application                   |
| src/bin                 | p2p_chat_server.rs   |     4 |  4836 |   138 |        - |       - | Headless node + web server entry point |
| src/bin                 | p2p_chat_tui.rs      |     6 |  9911 |   278 |        - |       - | Main TUI application entry point       |
| src/bin/tui             | click_handlers.rs    |     7 | 21862 |   523 |        - |       - | Click handlers & index mapping         |
| src/bin/tui             | command_processor.rs |     6 | 26081 |   725 |        - |       - | Event routing & state updates          |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |        - |       - | Terminal event polling (60 FPS)        |
| src/bin/tui             | input_processor.rs   |     7 | 66214 |  1705 |        - |       - | Input event routing & processing       |
| src/bin/tui             | key_probe.rs         |     8 |  6178 |   162 |        - |       - | Keyboard enhancement probe             |
| src/bin/tui             | main_loop.rs         |     6 | 15810 |   398 |        - |       - | Task orchestration & async             |
| src/bin/tui             | message_handlers.rs  |     4 | 16779 |   516 |        - |       - | Message sending logic                  |
| src/bin/tui             | scroll_handlers.rs   |     5 | 16156 |   425 |        - |       - | Scroll & hover-aware navigation        |
| src/bin/tui             | state.rs             |     8 | 57234 |  1370 |        - |       - | Shared application state               |
| src/bin/tui/render_loop | layout.rs            |     5 |  2923 |    82 |        - |       - | UI layout component rendering          |
| src/bin/tui/render_loop | mod.rs               |     6 | 13810 |   365 |        - |       - | Render loop orchestration (60 FPS)     |
| src/generated           | columns.rs           |     1 |  4107 |    97 |        - |       - | Auto-generated column definitions      |
| src/generated           | mod.rs               |     0 |   488 |    11 |        - |       - | Module declarations                    |
| src/generated           | models_insertable.rs |     1 |  9589 |   267 |        - |       - | Insertable data models                 |
| src/generated           | models_queryable.rs  |     1 | 12527 |   337 |        - |       - | Queryable data models                  |
| src/generated           | schema.rs            |     2 |  8761 |   288 |        - |       - | Database schema (Diesel)               |

**Total:** 64 files, 33,637 lines, 1,271,933 characters

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
| tests  | db_selection.rs                   |    73 |  2253 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   301 |  8151 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   335 |  9679 |     3 | logging module tests                  |
| tests  | messages.rs                       |   495 | 15274 |     4 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   516 | 17397 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1099 | 39657 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   317 |  9240 |     4 | peers module tests                    |
| tests  | swarm_handler.rs                  |   808 | 27490 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   319 |  9833 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   739 | 23504 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   495 | 15705 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |  1621 | 56236 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   269 |  8313 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                          |   662 | 20562 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |   122 |  3894 |     3 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   648 | 23912 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |  1228 | 41026 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |  2925 | 100025 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   233 |  7007 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   467 | 15073 |     5 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   408 | 14566 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   657 | 21507 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   889 | 28404 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    66 |  2330 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_blocklist.rs                 |    98 |  3435 |     2 | Test file                             |
| unit   | unit_calls.rs                     |    90 |  2797 |     1 | calls module tests                    |
| unit   | unit_connected.rs                 |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_contacts.rs                  |   357 | 12111 |     4 | contacts module tests                 |
| unit   | unit_db.rs                        |   273 |  8611 |     3 | Unit tests for database module        |
| unit   | unit_dm_outbox.rs                 |   169 |  5739 |     4 | Test file                             |
| unit   | unit_e2e.rs                       |   341 | 12479 |     3 | Test file                             |
| unit   | unit_files.rs                     |   503 | 18713 |     4 | files module tests                    |
| unit   | unit_groups.rs                    |   624 | 22507 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                  |   747 | 29220 |     4 | identity module tests                 |
| unit   | unit_lib.rs                       |   255 |  6980 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5217 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   580 | 21994 |     4 | Unit tests for messages module        |
| unit   | unit_net_diag.rs                  |   479 | 15624 |     3 | Test file                             |
| unit   | unit_network.rs                   |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_network_config.rs            |   153 |  5151 |     4 | Test file                             |
| unit   | unit_nickname.rs                  |   173 |  5791 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_relay.rs                     |    62 |  1806 |     2 | Test file                             |
| unit   | unit_reply.rs                     |    74 |  2384 |     2 | Test file                             |
| unit   | unit_settings.rs                  |    88 |  2766 |     3 | Test file                             |
| unit   | unit_swarm_handler.rs             |   409 | 14000 |     4 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   365 | 11174 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   323 |  9859 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3105 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   367 | 10657 |     3 | Unit tests for types module           |

**Total:** 63 test files, 25,445 lines, 844,774 characters

## Dart Source Files

| Folder       | File                            | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:--------------------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart                       |     5 |  3163 |   108 |        - |       - | Flutter app entry point          |
| lib          | chat_message_ext.dart           |     2 |  2638 |    81 |        - |       - | Hand-written message helpers     |
| lib          | frb_node_api.dart               |     5 | 12174 |   408 |        - |       - | In-process FRB transport         |
| lib          | http_node_api.dart              |     7 | 30758 |   929 |        - |       - | WebSocket/HTTP transport         |
| lib          | node_api.dart                   |     2 | 10774 |   308 |        - |       - | Transport-agnostic node API      |
| lib          | node_transport.dart             |     4 |  3396 |   106 |        - |       - | Duplex transport abstraction     |
| lib/src/rust | api.dart                        |     3 | 21606 |   593 |        - |       - | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart              |     6 | 150342 |  4768 |        - |       - | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart           |     3 | 19720 |   814 |        - |       - | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart          |     2 | 19640 |   814 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart                   |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart                 |     5 | 16684 |   519 |        - |       - | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart                |     5 | 16515 |   500 |        - |       - | Mobile node bindings (generated) |
| lib/src/rust | net_diag.dart                   |     5 |  7968 |   256 |        - |       - | Dart source file                 |
| lib/src/rust | network_config.dart             |     5 |  1182 |    43 |        - |       - | Dart source file                 |
| lib/src/rust | notifications.dart              |     5 |  2232 |    76 |        - |       - | Dart source file                 |
| lib/src/rust | reactions.dart                  |     5 |  1113 |    37 |        - |       - | Dart source file                 |
| lib/src/rust | reply.dart                      |     5 |  1313 |    41 |        - |       - | Dart source file                 |
| lib/src/rust | theme.dart                      |     5 |  1851 |    68 |        - |       - | Dart source file                 |
| lib          | call_screen.dart                |     8 |  9650 |   323 |        - |       - | Call overlay screen              |
| lib          | contact_list.dart               |    15 | 14857 |   429 |        - |       - | Contacts roster screen           |
| lib          | dm_chat.dart                    |     8 | 14742 |   438 |        - |       - | Direct-message chat screen       |
| lib          | group_chat.dart                 |     8 | 12371 |   367 |        - |       - | Group chat screen                |
| lib          | group_list.dart                 |    17 |  7931 |   235 |        - |       - | Group list screen                |
| lib          | home.dart                       |     7 | 41038 |  1224 |        - |       - | App shell & broadcast chat       |
| lib          | log_tab.dart                    |    10 |  3493 |   123 |        - |       - | Log tab screen                   |
| lib          | messages.dart                   |    15 | 43068 |  1251 |        - |       - | Shared chat widgets              |
| lib          | nav_bar.dart                    |     3 |  1252 |    34 |        - |       - | Tab navigation bar               |
| lib          | peer_info.dart                  |    11 | 17257 |   500 |        - |       - | Peer Info screen                 |
| lib          | settings.dart                   |    16 | 39169 |  1140 |        - |       - | Settings screen                  |
| lib          | attachments.dart                |     4 |  3656 |    92 |        - |       - | File picker & opener seams       |
| lib          | blocklist.dart                  |     2 |   755 |    19 |        - |       - | Dart source file                 |
| lib          | event_bus.dart                  |     2 |  1813 |    45 |        - |       - | Node event fan-out               |
| lib          | notifications.dart              |     3 |  4511 |   121 |        - |       - | Dart source file                 |
| lib          | notifications_platform.dart     |     0 |   931 |    18 |        - |       - | Dart source file                 |
| lib          | notifications_platform_web.dart |     3 |  3244 |    83 |        - |       - | Dart source file                 |
| lib          | reply.dart                      |     3 |  3273 |    73 |        - |       - | Dart source file                 |
| lib          | theme.dart                      |     2 |  1821 |    49 |        - |       - | Dart source file                 |
| lib          | ui_utils.dart                   |     2 |  1603 |    37 |        - |       - | Shared formatting & env helpers  |

**Total:** 39 files, 17,102 lines, 550,571 characters

## Dart Test Files

| Folder  | File                             | Lines | Chars | Depth | Description                           |
|:--------|:---------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart               |   658 | 20111 |     4 | In-memory node fake                   |
| unit    | api_test.dart                    |    61 |  2389 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart            |   134 |  3997 |     5 | Attachment helper tests               |
| unit    | blocklist_test.dart              |    47 |  1429 |     4 | Dart test file                        |
| unit    | http_node_api_test.dart          |   744 | 24284 |    10 | Web transport wire-contract tests     |
| unit    | notifications_test.dart          |    99 |  3572 |     3 | Dart test file                        |
| widget  | attachment_chip_test.dart        |    92 |  2611 |     7 | Attachment chip widget tests          |
| widget  | call_screen_test.dart            |   313 | 10264 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart           |   661 | 22484 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart         |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart             |   233 |  7128 |     5 | Group list widget tests               |
| widget  | messages_test.dart               |  1077 | 31615 |    10 | Chat widget tests                     |
| widget  | peer_info_test.dart              |   306 |  9464 |     6 | Peer Info widget tests                |
| widget  | settings_network_peers_test.dart |   128 |  4028 |     3 | Dart test file                        |
| widget  | settings_test.dart               |   285 |  9080 |     6 | Settings widget tests                 |
| test    | widget_test.dart                 |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 16 test files, 4,931 lines, 155,070 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  3277 |    91 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | MessageNotifications.kt |     4 |  3431 |    85 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  4282 |   127 |        - |       - | Foreground service for background networking |

**Total:** 3 files, 303 lines, 10,990 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | MainActivityTest.kt            |    80 |  2878 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 2 test files, 179 lines, 5,971 characters
