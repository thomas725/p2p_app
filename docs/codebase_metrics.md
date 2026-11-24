# Codebase Metrics

## Summary

| Metric                |                Value |
|:----------------------|:---------------------|
| Total Rust Files      |                   70 |
| Total Rust Lines      |               40,269 |
| Total Rust Chars      |            1,525,079 |
| Avg Lines/Rust File   |                  575 |
| Avg Chars/Rust File   |               21,786 |
| Covered Rust Lines    | 7,884 / 13,206 (60%) |
| Total Dart Files      |                   47 |
| Total Dart Lines      |               21,501 |
| Total Dart Chars      |              689,477 |
| Avg Lines/Dart File   |                  457 |
| Avg Chars/Dart File   |               14,669 |
| Covered Dart Lines    |  2,494 / 6,983 (36%) |
| Total Kotlin Files    |                    4 |
| Total Kotlin Lines    |                  612 |
| Total Kotlin Chars    |               22,099 |
| Avg Lines/Kotlin File |                  153 |
| Avg Chars/Kotlin File |                5,524 |

**Grand Total:** 121 files, 62,382 lines, 2,236,655 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                            |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                                       |
| src                     | behavior.rs          |     6 | 30694 |   757 |       96 |  96.88% | Network behavior definitions                       |
| src                     | blocklist.rs         |     2 |  4185 |   110 |       37 |  97.30% | Source file                                        |
| src                     | call_media.rs        |     3 |  8393 |   236 |       65 | 100.00% | Source file                                        |
| src                     | calls.rs             |     3 |  3221 |    90 |       16 | 100.00% | Call session state machine (signaling)             |
| src                     | chat.rs              |     5 |  5821 |   162 |       80 |  83.75% | TUI chat & DM pane renderer                        |
| src                     | clipboard.rs         |     3 |  3617 |    98 |        7 | 100.00% | Source file                                        |
| src                     | connected.rs         |     3 |  2324 |    71 |       18 | 100.00% | Connected peers tracking & monitoring              |
| src                     | contacts.rs          |     4 | 14829 |   395 |      125 |  97.60% | Canonical contacts-roster model                    |
| src                     | db.rs                |     5 | 27253 |   686 |      109 |  93.58% | Database connection & identity mgmt                |
| src                     | dm_outbox.rs         |     3 |  5631 |   155 |       59 | 100.00% | Source file                                        |
| src                     | e2e.rs               |     4 | 17759 |   443 |      131 |  93.13% | Source file                                        |
| src                     | file_transfer.rs     |     5 | 20140 |   507 |      204 |  37.25% | Source file                                        |
| src                     | files.rs             |     6 | 30295 |   791 |      287 |  94.08% | Content-addressed attachment store                 |
| src                     | fmt.rs               |     4 |  6204 |   192 |       67 | 100.00% | Formatting & display utilities                     |
| src                     | frb_generated.rs     |     6 | 251637 |  6066 |     2855 |   0.00% | flutter_rust_bridge codegen                        |
| src                     | groups.rs            |     3 | 23346 |   618 |      220 |  96.82% | Group management & persistence                     |
| src                     | groups.rs            |     5 |  4846 |   131 |       68 |  45.59% | TUI groups renderer                                |
| src                     | identity.rs          |     4 | 28935 |   763 |      277 |  95.31% | Users, account keys & device bindings              |
| src                     | lib.rs               |     1 |  7558 |   202 |        1 | 100.00% | Module declarations & re-exports                   |
| src                     | log.rs               |     2 |   907 |    31 |       10 | 100.00% | TUI log renderer                                   |
| src                     | logging.rs           |     5 | 13949 |   426 |      123 |  95.93% | Logging utilities & setup                          |
| src                     | messages.rs          |     5 | 22794 |   587 |      191 |  98.95% | Message persistence & retrieval                    |
| src                     | methods.rs           |     4 | 34326 |  1070 |      316 |  67.72% | Web JSON method dispatch                           |
| src                     | mobile_api.rs        |     4 | 36782 |  1049 |       92 |  72.83% | Mobile FRB API surface                             |
| src                     | mobile_node.rs       |     5 | 96435 |  2565 |      505 |  45.15% | Mobile node lifecycle & swarm                      |
| src                     | mod.rs               |     5 | 45421 |  1310 |      261 |  49.43% | FRB API surface                                    |
| src                     | mod.rs               |     4 | 12328 |   319 |      130 |  89.23% | TUI render chrome & dispatch                       |
| src                     | mod.rs               |     7 | 63897 |  1673 |      184 |  79.35% | Web server (axum HTTP+WS)                          |
| src                     | net_diag.rs          |     4 | 21449 |   596 |      209 |  98.56% | Source file                                        |
| src                     | network.rs           |     3 |  1979 |    67 |       14 | 100.00% | Network size classification                        |
| src                     | network_config.rs    |     3 |  7751 |   216 |       68 |  98.53% | Source file                                        |
| src                     | nickname.rs          |     5 | 12838 |   333 |      131 |  99.24% | Nickname management                                |
| src                     | notifications.rs     |     4 |  8213 |   241 |       51 |  86.27% | Source file                                        |
| src                     | peer_info.rs         |     3 |  3104 |    88 |       39 |  92.31% | TUI peer-info renderer                             |
| src                     | peers.rs             |     4 | 10590 |   294 |      101 | 100.00% | Peer management & tracking                         |
| src                     | peers.rs             |     5 |  5890 |   136 |       63 | 100.00% | TUI peers-table renderer                           |
| src                     | reactions.rs         |     5 | 15921 |   419 |      113 |  94.69% | Source file                                        |
| src                     | relay.rs             |     5 |  3764 |    95 |       27 |  96.30% | Source file                                        |
| src                     | reply.rs             |     4 |  5781 |   157 |       46 | 100.00% | Source file                                        |
| src                     | settings.rs          |     3 |  6614 |   168 |       43 |  90.70% | Source file                                        |
| src                     | settings.rs          |     4 |  8302 |   225 |      117 |  97.44% | TUI settings renderer                              |
| src                     | shares.rs            |     6 | 45208 |  1215 |      458 |  92.58% | File shares & folder sync (ACLs, manifests, apply) |
| src                     | shares_json.rs       |     4 |  6336 |   195 |       34 | 100.00% | Source file                                        |
| src                     | swarm_handler.rs     |     7 | 100219 |  2616 |     1006 |  51.19% | Network event translation                          |
| src                     | theme.rs             |     4 |  7223 |   234 |       47 |  89.36% | Source file                                        |
| src                     | tui_helpers.rs       |     5 | 21932 |   663 |      232 |  96.12% | TUI helper functions & utilities                   |
| src                     | tui_render_state.rs  |     4 | 25692 |   690 |      223 |  91.03% | TUI render state & tab content                     |
| src                     | tui_tabs.rs          |     6 | 11711 |   342 |      131 |  99.24% | Tab management & navigation                        |
| src                     | types.rs             |     2 | 18507 |   485 |        8 |  62.50% | Event & command type defs                          |
| src/bin                 | p2p_chat.rs          |     7 |  6917 |   192 |       63 |  11.11% | CLI chat application                               |
| src/bin                 | p2p_chat_server.rs   |     4 |  4836 |   138 |       46 |   0.00% | Headless node + web server entry point             |
| src/bin                 | p2p_chat_tui.rs      |     6 |  9871 |   277 |       52 |  25.00% | Main TUI application entry point                   |
| src/bin/tui             | call_audio.rs        |     6 | 19279 |   540 |      178 |  33.15% | Source file                                        |
| src/bin/tui             | click_handlers.rs    |     7 | 22225 |   533 |      268 |  71.64% | Click handlers & index mapping                     |
| src/bin/tui             | command_processor.rs |     6 | 31796 |   867 |      404 |  78.71% | Event routing & state updates                      |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |       12 |  41.67% | Terminal event polling (60 FPS)                    |
| src/bin/tui             | input_processor.rs   |     7 | 73074 |  1881 |      873 |  76.98% | Input event routing & processing                   |
| src/bin/tui             | key_probe.rs         |     6 |  6272 |   172 |       77 |   0.00% | Keyboard enhancement probe                         |
| src/bin/tui             | main_loop.rs         |     6 | 16099 |   402 |      165 |  22.42% | Task orchestration & async                         |
| src/bin/tui             | message_handlers.rs  |     4 | 17117 |   522 |      249 |  89.96% | Message sending logic                              |
| src/bin/tui             | scroll_handlers.rs   |     6 | 17631 |   470 |      202 |  91.09% | Scroll & hover-aware navigation                    |
| src/bin/tui             | state.rs             |     8 | 63249 |  1494 |      545 |  88.07% | Shared application state                           |
| src/bin/tui/render_loop | layout.rs            |     5 |  3173 |    89 |       46 |  54.35% | UI layout component rendering                      |
| src/bin/tui/render_loop | mod.rs               |     6 | 15105 |   394 |      170 |  60.59% | Render loop orchestration (60 FPS)                 |
| src/generated           | columns.rs           |     1 |  4561 |   108 |        0 |       - | Auto-generated column definitions                  |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                                |
| src/generated           | models_insertable.rs |     1 | 11215 |   317 |        0 |       - | Insertable data models                             |
| src/generated           | models_queryable.rs  |     1 | 14539 |   397 |        0 |       - | Queryable data models                              |
| src/generated           | schema.rs            |     2 |  9786 |   330 |      161 |  93.17% | Database schema (Diesel)                           |

**Total:** 70 files, 40,269 lines, 1,525,079 characters (7884/13206 testable lines covered, 60%)

## Rust Test Files

| Folder | File                               | Lines | Chars | Depth | Description                           |
|:-------|:-----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs                |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                 |   157 |  4819 |     3 | Diesel queryable model tests          |
| shared | logging_test_utils.rs              |    18 |   877 |     2 | Logging test helpers                  |
| shared | tui_test_state.rs                  |   250 |  7360 |     6 | TUI test state builders               |
| tests  | additional_coverage.rs             |   146 |  4516 |     2 | Additional coverage tests             |
| tests  | behavior.rs                        |   252 |  7228 |     5 | behavior module tests                 |
| tests  | db.rs                              |   281 |  8625 |     4 | database module tests                 |
| tests  | db_selection.rs                    |    73 |  2253 |     3 | Database selection tests              |
| tests  | fmt.rs                             |   301 |  8151 |     2 | fmt module tests                      |
| tests  | logging.rs                         |   335 |  9679 |     3 | logging module tests                  |
| tests  | messages.rs                        |   495 | 15274 |     4 | messages module tests                 |
| tests  | network.rs                         |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                        |   516 | 17397 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                 |  1099 | 39657 |    10 | P2P integration tests                 |
| tests  | peers.rs                           |   317 |  9240 |     4 | peers module tests                    |
| tests  | swarm_handler.rs                   |  1292 | 44723 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                      |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs          |   327 | 10202 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                        |   739 | 23504 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                     |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                 |   495 | 15705 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs          |  1762 | 61077 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                       |   269 |  8313 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                       |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                           |   662 | 20562 |     3 | types module tests                    |
| unit   | unit_behavior.rs                   |   122 |  3894 |     3 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs     |   658 | 24426 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs  |  1377 | 46839 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs       |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs    |  3519 | 121279 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs          |   233 |  7007 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs   |   467 | 15073 |     5 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_layout.rs |    70 |  2251 |     2 | Test file                             |
| unit   | unit_bin_tui_render_loop_mod.rs    |   408 | 14566 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs    |   787 | 25481 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs              |  1120 | 37039 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs       |    66 |  2330 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_blocklist.rs                  |   112 |  3932 |     3 | Test file                             |
| unit   | unit_call_media.rs                 |   148 |  4734 |     3 | Test file                             |
| unit   | unit_calls.rs                      |    90 |  2797 |     1 | calls module tests                    |
| unit   | unit_connected.rs                  |    65 |  2019 |     1 | Unit tests for connected module       |
| unit   | unit_contacts.rs                   |   357 | 12111 |     4 | contacts module tests                 |
| unit   | unit_db.rs                         |   312 | 10040 |     3 | Unit tests for database module        |
| unit   | unit_dm_outbox.rs                  |   169 |  5739 |     4 | Test file                             |
| unit   | unit_e2e.rs                        |   341 | 12479 |     3 | Test file                             |
| unit   | unit_file_transfer.rs              |   276 |  9628 |     4 | Test file                             |
| unit   | unit_files.rs                      |   538 | 20323 |     4 | files module tests                    |
| unit   | unit_groups.rs                     |   624 | 22507 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                   |   747 | 29217 |     4 | identity module tests                 |
| unit   | unit_lib.rs                        |   278 |  7955 |     3 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                    |   205 |  5217 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                   |   664 | 25132 |     4 | Unit tests for messages module        |
| unit   | unit_net_diag.rs                   |   483 | 15776 |     3 | Test file                             |
| unit   | unit_network.rs                    |    38 |  1098 |     2 | Unit tests for network module         |
| unit   | unit_network_config.rs             |   167 |  5564 |     4 | Test file                             |
| unit   | unit_nickname.rs                   |   173 |  5791 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                      |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_relay.rs                      |    51 |  1450 |     2 | Test file                             |
| unit   | unit_reply.rs                      |   134 |  4907 |     3 | Test file                             |
| unit   | unit_settings.rs                   |   104 |  3289 |     3 | Test file                             |
| unit   | unit_shares.rs                     |   973 | 34810 |     4 | shares module tests                   |
| unit   | unit_swarm_handler.rs              |   422 | 14373 |     4 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs                |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs           |   365 | 11174 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                   |   330 | 10365 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs             |    99 |  3105 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                      |   400 | 12098 |     3 | Unit tests for types module           |

**Total:** 67 test files, 28,990 lines, 972,051 characters

## Dart Source Files

| Folder       | File                             | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:---------------------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart                        |     5 |  3163 |   108 |       40 |  50.00% | Flutter app entry point          |
| lib          | chat_message_ext.dart            |     3 |  2342 |    60 |       25 | 100.00% | Hand-written message helpers     |
| lib          | frb_node_api.dart                |     5 | 15668 |   525 |      176 |   5.68% | In-process FRB transport         |
| lib          | http_node_api.dart               |     7 | 36096 |  1112 |      498 |  56.02% | WebSocket/HTTP transport         |
| lib          | node_api.dart                    |     2 | 13989 |   407 |        1 | 100.00% | Transport-agnostic node API      |
| lib          | node_transport.dart              |     4 |  3396 |   106 |       27 |   0.00% | Duplex transport abstraction     |
| lib          | shares_json.dart                 |     5 |  3082 |    90 |       57 |  77.19% | Dart source file                 |
| lib/src/rust | api.dart                         |     5 | 34760 |   992 |      219 |   5.02% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart               |     6 | 197258 |  6302 |     2594 |   0.08% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart            |     3 | 26933 |  1121 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart           |     2 | 26853 |  1121 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | messages.dart                    |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart                  |     5 | 16684 |   519 |      152 |   4.61% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart                 |     5 | 17661 |   526 |      160 |   4.38% | Mobile node bindings (generated) |
| lib/src/rust | net_diag.dart                    |     5 |  7968 |   256 |       81 |   2.47% | Dart source file                 |
| lib/src/rust | network_config.dart              |     5 |  1182 |    43 |       10 |  10.00% | Dart source file                 |
| lib/src/rust | notifications.dart               |     5 |  2232 |    76 |       23 |   8.70% | Dart source file                 |
| lib/src/rust | reactions.dart                   |     5 |  1113 |    37 |        9 |  11.11% | Dart source file                 |
| lib/src/rust | reply.dart                       |     5 |  1313 |    41 |        9 |  77.78% | Dart source file                 |
| lib/src/rust | shares.dart                      |     5 |  3924 |   145 |       37 |  27.03% | Dart source file                 |
| lib/src/rust | theme.dart                       |     5 |  1851 |    68 |       21 |   0.00% | Dart source file                 |
| lib          | call_screen.dart                 |    17 | 15735 |   485 |      170 |  91.76% | Call overlay screen              |
| lib          | contact_list.dart                |    15 | 14857 |   429 |      181 |  99.45% | Contacts roster screen           |
| lib          | dm_chat.dart                     |     8 | 20097 |   574 |      269 |  77.32% | Direct-message chat screen       |
| lib          | group_chat.dart                  |     8 | 12394 |   366 |      176 |  50.00% | Group chat screen                |
| lib          | group_list.dart                  |    17 |  7931 |   235 |       99 |  98.99% | Group list screen                |
| lib          | home.dart                        |     7 | 42833 |  1277 |      568 |  35.56% | App shell & broadcast chat       |
| lib          | log_tab.dart                     |    10 |  3493 |   123 |       51 |  39.22% | Log tab screen                   |
| lib          | messages.dart                    |    15 | 45724 |  1320 |      491 |  87.78% | Shared chat widgets              |
| lib          | nav_bar.dart                     |     3 |  1252 |    34 |        5 | 100.00% | Tab navigation bar               |
| lib          | peer_info.dart                   |    11 | 17257 |   500 |      190 |  91.58% | Peer Info screen                 |
| lib          | settings.dart                    |    16 | 40337 |  1170 |      456 |  79.82% | Settings screen                  |
| lib          | share_list.dart                  |    17 |  9951 |   301 |        - |       - | Dart source file                 |
| lib          | attachments.dart                 |     4 |  3656 |    92 |       25 |  44.00% | File picker & opener seams       |
| lib          | blocklist.dart                   |     2 |   755 |    19 |        6 | 100.00% | Dart source file                 |
| lib          | call_audio.dart                  |     4 |  4139 |   104 |       27 |  96.30% | Dart source file                 |
| lib          | call_audio_device.dart           |     2 |   436 |    11 |        1 | 100.00% | Dart source file                 |
| lib          | call_audio_platform.dart         |     2 |   850 |    26 |        6 | 100.00% | Dart source file                 |
| lib          | call_audio_platform_android.dart |     4 |  4054 |   106 |       42 |  95.24% | Dart source file                 |
| lib          | call_audio_platform_web.dart     |     5 |  7995 |   216 |        - |       - | Dart source file                 |
| lib          | event_bus.dart                   |     2 |  1813 |    45 |        8 |  75.00% | Node event fan-out               |
| lib          | notifications.dart               |     3 |  4511 |   121 |       22 |  54.55% | Dart source file                 |
| lib          | notifications_platform.dart      |     0 |   931 |    18 |        3 |   0.00% | Dart source file                 |
| lib          | notifications_platform_web.dart  |     3 |  3244 |    83 |        - |       - | Dart source file                 |
| lib          | reply.dart                       |     3 |  3273 |    73 |       25 |  96.00% | Dart source file                 |
| lib          | theme.dart                       |     2 |  1821 |    49 |        8 |  12.50% | Dart source file                 |
| lib          | ui_utils.dart                    |     2 |  1603 |    37 |       10 |  60.00% | Shared formatting & env helpers  |

**Total:** 47 files, 21,501 lines, 689,477 characters (2494/6983 testable lines covered, 36%)

## Dart Test Files

| Folder  | File                             | Lines | Chars | Depth | Description                           |
|:--------|:---------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart               |   798 | 24166 |     4 | In-memory node fake                   |
| unit    | api_test.dart                    |    61 |  2389 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart            |   132 |  4053 |     5 | Attachment helper tests               |
| unit    | blocklist_test.dart              |    47 |  1429 |     4 | Dart test file                        |
| unit    | call_audio_android_test.dart     |   167 |  5393 |     5 | Dart test file                        |
| unit    | call_audio_test.dart             |    93 |  3466 |     5 | Dart test file                        |
| unit    | http_node_api_test.dart          |   897 | 29428 |    10 | Web transport wire-contract tests     |
| unit    | notifications_test.dart          |    99 |  3572 |     3 | Dart test file                        |
| widget  | attachment_chip_test.dart        |    91 |  2630 |     7 | Attachment chip widget tests          |
| widget  | call_screen_test.dart            |   593 | 18895 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart           |   661 | 22484 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart         |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart             |   233 |  7128 |     5 | Group list widget tests               |
| widget  | messages_test.dart               |  1352 | 40012 |    10 | Chat widget tests                     |
| widget  | peer_info_test.dart              |   306 |  9464 |     6 | Peer Info widget tests                |
| widget  | settings_network_peers_test.dart |   128 |  4028 |     3 | Dart test file                        |
| widget  | settings_test.dart               |   285 |  9080 |     6 | Settings widget tests                 |
| widget  | share_list_test.dart             |   174 |  5157 |     6 | Dart test file                        |
| test    | widget_test.dart                 |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 19 test files, 6,210 lines, 195,388 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | CallAudio.kt            |     6 | 10250 |   286 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  4136 |   114 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | MessageNotifications.kt |     4 |  3431 |    85 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  4282 |   127 |        - |       - | Foreground service for background networking |

**Total:** 4 files, 612 lines, 22,099 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | CallAudioTest.kt               |    51 |  1765 |     3 | Kotlin test file                      |
| com/example/p2p_app_flutter | MainActivityTest.kt            |    80 |  2878 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 3 test files, 230 lines, 7,736 characters
