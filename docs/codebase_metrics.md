# Codebase Metrics

## Summary

| Metric                |                Value |
|:----------------------|:---------------------|
| Total Rust Files      |                   75 |
| Total Rust Lines      |               44,618 |
| Total Rust Chars      |            1,702,533 |
| Avg Lines/Rust File   |                  594 |
| Avg Chars/Rust File   |               22,700 |
| Covered Rust Lines    | 9,997 / 15,525 (64%) |
| Total Dart Files      |                   62 |
| Total Dart Lines      |               27,060 |
| Total Dart Chars      |              872,809 |
| Avg Lines/Dart File   |                  436 |
| Avg Chars/Dart File   |               14,077 |
| Covered Dart Lines    |  3,729 / 8,972 (42%) |
| Total Kotlin Files    |                    5 |
| Total Kotlin Lines    |                1,060 |
| Total Kotlin Chars    |               39,580 |
| Avg Lines/Kotlin File |                  212 |
| Avg Chars/Kotlin File |                7,916 |

**Grand Total:** 142 files, 72,738 lines, 2,614,922 characters

## Rust Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                                            |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|---------------------------------------------------:|
| /                       | build.rs             |     5 |  4068 |   117 |        0 |       - | Build script                                       |
| src                     | behavior.rs          |     6 | 30694 |   757 |       96 |  96.88% | Network behavior definitions                       |
| src                     | blocklist.rs         |     2 |  4185 |   110 |       37 |  97.30% | Source file                                        |
| src                     | call_media.rs        |     3 |  8270 |   233 |       65 | 100.00% | Source file                                        |
| src                     | calls.rs             |     3 |  3221 |    90 |       16 | 100.00% | Call session state machine (signaling)             |
| src                     | clipboard.rs         |     3 |  3617 |    98 |        7 | 100.00% | Source file                                        |
| src                     | connected.rs         |     3 |  2983 |    89 |       24 | 100.00% | Connected peers tracking & monitoring              |
| src                     | contacts.rs          |     4 | 14254 |   399 |      118 |  98.31% | Canonical contacts-roster model                    |
| src                     | db.rs                |     5 | 27253 |   686 |      109 |  93.58% | Database connection & identity mgmt                |
| src                     | dm_outbox.rs         |     3 |  5631 |   155 |       59 | 100.00% | Source file                                        |
| src                     | e2e.rs               |     4 | 17759 |   443 |      131 |  93.89% | Source file                                        |
| src                     | file_transfer.rs     |     5 | 20140 |   507 |      204 |  77.45% | Source file                                        |
| src                     | files.rs             |     6 | 30295 |   791 |      287 |  94.08% | Content-addressed attachment store                 |
| src                     | fmt.rs               |     4 |  6204 |   192 |       67 | 100.00% | Formatting & display utilities                     |
| src                     | frb_generated.rs     |     6 | 284865 |  6851 |     3229 |   0.00% | flutter_rust_bridge codegen                        |
| src                     | groups.rs            |     3 | 24841 |   664 |      230 |  96.96% | Group management & persistence                     |
| src                     | identity.rs          |     4 | 30488 |   797 |      286 |  95.45% | Users, account keys & device bindings              |
| src                     | lib.rs               |     1 |  7604 |   204 |        1 | 100.00% | Module declarations & re-exports                   |
| src                     | location.rs          |     5 | 29490 |   804 |      257 |  89.88% | Source file                                        |
| src                     | logging.rs           |     5 | 13949 |   426 |      123 |  97.56% | Logging utilities & setup                          |
| src                     | messages.rs          |     5 | 25355 |   643 |      220 |  99.09% | Message persistence & retrieval                    |
| src                     | mobile_api.rs        |     4 | 35164 |  1034 |       69 |  89.86% | Mobile FRB API surface                             |
| src                     | mobile_node.rs       |     5 | 69263 |  1770 |      527 |  51.04% | Mobile node lifecycle & swarm                      |
| src                     | mod.rs               |     5 | 51916 |  1452 |      291 |  48.11% | FRB API surface                                    |
| src                     | net_diag.rs          |     4 | 21449 |   596 |      209 |  98.56% | Source file                                        |
| src                     | network.rs           |     3 |  2655 |    84 |       18 | 100.00% | Network size classification                        |
| src                     | network_config.rs    |     3 |  7751 |   216 |       68 |  98.53% | Source file                                        |
| src                     | nickname.rs          |     5 | 13241 |   343 |      131 |  99.24% | Nickname management                                |
| src                     | notifications.rs     |     4 | 10791 |   303 |       57 |  92.98% | Source file                                        |
| src                     | peers.rs             |     4 | 10590 |   294 |      101 | 100.00% | Peer management & tracking                         |
| src                     | reactions.rs         |     5 | 17473 |   463 |      120 |  95.00% | Source file                                        |
| src                     | relay.rs             |     5 |  3764 |    95 |       27 |  96.30% | Source file                                        |
| src                     | reply.rs             |     4 |  7747 |   206 |       57 | 100.00% | Source file                                        |
| src                     | settings.rs          |     4 |  9991 |   251 |       66 | 100.00% | Source file                                        |
| src                     | shares.rs            |     6 | 58668 |  1556 |      569 |  95.08% | File shares & folder sync (ACLs, manifests, apply) |
| src                     | shares_json.rs       |     4 |  6336 |   195 |       34 | 100.00% | Source file                                        |
| src                     | sort.rs              |     3 |  2787 |    74 |       15 | 100.00% | Source file                                        |
| src                     | swarm_handler.rs     |     7 | 101082 |  2630 |     1012 |  53.85% | Network event translation                          |
| src                     | theme.rs             |     4 |  7223 |   234 |       47 |  89.36% | Source file                                        |
| src                     | tui_helpers.rs       |     5 | 21932 |   663 |      232 |  96.12% | TUI helper functions & utilities                   |
| src                     | tui_render_state.rs  |     4 | 31847 |   824 |      259 |  97.68% | TUI render state & tab content                     |
| src                     | tui_tabs.rs          |     6 | 12023 |   348 |      134 |  99.25% | Tab management & navigation                        |
| src                     | types.rs             |     4 | 20251 |   525 |       23 |  30.43% | Event & command type defs                          |
| src/bin                 | p2p_chat.rs          |     7 |  6917 |   192 |       63 |  11.11% | CLI chat application                               |
| src/bin                 | p2p_chat_server.rs   |     4 |  4836 |   138 |       46 |   0.00% | Headless node + web server entry point             |
| src/bin                 | p2p_chat_tui.rs      |     6 |  9927 |   279 |       52 |  25.00% | Main TUI application entry point                   |
| src/bin/tui             | call_audio.rs        |     6 | 19279 |   540 |      178 |  33.15% | Source file                                        |
| src/bin/tui             | click_handlers.rs    |     7 | 26019 |   617 |      318 |  83.65% | Click handlers & index mapping                     |
| src/bin/tui             | command_processor.rs |     6 | 32607 |   881 |      416 |  81.73% | Event routing & state updates                      |
| src/bin/tui             | event_source.rs      |     4 |  1227 |    40 |       12 |  41.67% | Terminal event polling (60 FPS)                    |
| src/bin/tui             | input_processor.rs   |     7 | 81730 |  2088 |      943 |  83.99% | Input event routing & processing                   |
| src/bin/tui             | key_probe.rs         |     6 |  6272 |   172 |       77 |   0.00% | Keyboard enhancement probe                         |
| src/bin/tui             | location_export.rs   |     5 | 16342 |   429 |       77 |  79.22% | Source file                                        |
| src/bin/tui             | main_loop.rs         |     6 | 15979 |   400 |      163 |  22.70% | Task orchestration & async                         |
| src/bin/tui             | message_handlers.rs  |     4 | 17819 |   536 |      260 |  90.38% | Message sending logic                              |
| src/bin/tui             | scroll_handlers.rs   |     4 | 22994 |   621 |      264 |  91.29% | Scroll & hover-aware navigation                    |
| src/bin/tui             | share_actions.rs     |     5 | 29288 |   799 |      368 |  83.70% | Source file                                        |
| src/bin/tui             | state.rs             |     8 | 102408 |  2423 |      923 |  91.77% | Shared application state                           |
| src/bin/tui/render_loop | layout.rs            |     5 |  4411 |   114 |       60 |  81.67% | UI layout component rendering                      |
| src/bin/tui/render_loop | mod.rs               |     6 | 17491 |   438 |      195 |  82.05% | Render loop orchestration (60 FPS)                 |
| src/generated           | columns.rs           |     1 |  4561 |   108 |        0 |       - | Auto-generated column definitions                  |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |       - | Module declarations                                |
| src/generated           | models_insertable.rs |     1 | 11215 |   317 |        0 |       - | Insertable data models                             |
| src/generated           | models_queryable.rs  |     1 | 14539 |   397 |        0 |       - | Queryable data models                              |
| src/generated           | schema.rs            |     2 |  9786 |   330 |      161 |  93.17% | Database schema (Diesel)                           |
| src/tui_render          | chat.rs              |     4 |  7347 |   206 |      100 |  98.00% | TUI chat & DM pane renderer                        |
| src/tui_render          | groups.rs            |     5 |  5201 |   139 |       73 |  94.52% | TUI groups renderer                                |
| src/tui_render          | log.rs               |     2 |   907 |    31 |       10 | 100.00% | TUI log renderer                                   |
| src/tui_render          | mod.rs               |     4 | 14348 |   338 |      135 |  85.19% | TUI render chrome & dispatch                       |
| src/tui_render          | peer_info.rs         |     3 |  3108 |    88 |       39 |  97.44% | TUI peer-info renderer                             |
| src/tui_render          | peers.rs             |     5 |  5890 |   136 |       63 | 100.00% | TUI peers-table renderer                           |
| src/tui_render          | settings.rs          |     5 | 16413 |   469 |      208 |  96.15% | TUI settings renderer                              |
| src/tui_render          | shares.rs            |     4 |  8575 |   230 |      119 | 100.00% | TUI shares renderer                                |
| src/web                 | methods.rs           |     5 | 44877 |  1379 |      386 |  68.91% | Web JSON method dispatch                           |
| src/web                 | mod.rs               |     7 | 20622 |   520 |      184 |  78.80% | Web server (axum HTTP+WS)                          |

**Total:** 75 files, 44,618 lines, 1,702,533 characters (9997/15525 testable lines covered, 64%)

## Rust Test Files

| Folder | File                               | Lines | Chars | Depth | Description                           |
|:-------|:-----------------------------------|------:|------:|------:|--------------------------------------:|
| shared | logging_test_utils.rs              |    18 |   877 |     2 | Logging test helpers                  |
| shared | tui_test_state.rs                  |   250 |  7360 |     6 | TUI test state builders               |
| tests  | additional_coverage.rs             |   146 |  4516 |     2 | Additional coverage tests             |
| tests  | behavior.rs                        |   252 |  7228 |     5 | behavior module tests                 |
| tests  | db.rs                              |   263 |  8079 |     4 | database module tests                 |
| tests  | db_selection.rs                    |    73 |  2253 |     3 | Database selection tests              |
| tests  | fmt.rs                             |   301 |  8151 |     2 | fmt module tests                      |
| tests  | location_integration.rs            |   303 | 10305 |     4 | Test file                             |
| tests  | logging.rs                         |   335 |  9679 |     3 | logging module tests                  |
| tests  | messages.rs                        |   519 | 16449 |     5 | messages module tests                 |
| tests  | network.rs                         |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                        |   413 | 13419 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                 |  1099 | 39657 |    10 | P2P integration tests                 |
| tests  | peers.rs                           |   317 |  9240 |     4 | peers module tests                    |
| tests  | swarm_handler.rs                   |  1292 | 44723 |     8 | swarm_handler module tests            |
| tests  | test_utils.rs                      |    53 |  1864 |     2 | Test utilities                        |
| tests  | tui_binary_integration.rs          |   327 | 10218 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                        |   741 | 23619 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                     |   437 | 13387 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                 |   520 | 16672 |     4 | TUI integration tests                 |
| tests  | tui_render_integration.rs          |  2254 | 79974 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                       |   269 |  8303 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                       |   245 |  7811 |     7 | TUI task tests                        |
| tests  | types.rs                           |   662 | 20583 |     3 | types module tests                    |
| unit   | unit_behavior.rs                   |   122 |  3894 |     3 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs     |   938 | 34577 |     4 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs  |  1735 | 59173 |     5 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs       |    44 |  1352 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs    |  6962 | 247284 |     5 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs          |   233 |  7007 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs   |   467 | 15073 |     5 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_layout.rs |   122 |  4316 |     2 | Test file                             |
| unit   | unit_bin_tui_render_loop_mod.rs    |   533 | 18586 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs    |   903 | 29318 |     4 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs              |  1752 | 59007 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs       |   237 |  8407 |     5 | Unit tests for TUI test helpers       |
| unit   | unit_blocklist.rs                  |   112 |  3932 |     3 | Test file                             |
| unit   | unit_call_media.rs                 |   148 |  4734 |     3 | Test file                             |
| unit   | unit_calls.rs                      |    90 |  2797 |     1 | calls module tests                    |
| unit   | unit_connected.rs                  |    84 |  2572 |     3 | Unit tests for connected module       |
| unit   | unit_contacts.rs                   |   435 | 14754 |     4 | contacts module tests                 |
| unit   | unit_db.rs                         |   312 | 10040 |     3 | Unit tests for database module        |
| unit   | unit_dm_outbox.rs                  |   169 |  5739 |     4 | Test file                             |
| unit   | unit_e2e.rs                        |   341 | 12479 |     3 | Test file                             |
| unit   | unit_file_transfer.rs              |   276 |  9628 |     4 | Test file                             |
| unit   | unit_files.rs                      |   538 | 20323 |     4 | files module tests                    |
| unit   | unit_groups.rs                     |   649 | 23459 |     5 | Unit tests for groups module          |
| unit   | unit_identity.rs                   |   747 | 29217 |     4 | identity module tests                 |
| unit   | unit_lib.rs                        |   298 |  8790 |     3 | Unit tests for library re-exports/api |
| unit   | unit_location.rs                   |   287 | 10115 |     3 | Test file                             |
| unit   | unit_logging.rs                    |   205 |  5217 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                   |   685 | 25897 |     4 | Unit tests for messages module        |
| unit   | unit_mobile_node.rs                |   912 | 28356 |     4 | Test file                             |
| unit   | unit_net_diag.rs                   |   483 | 15776 |     3 | Test file                             |
| unit   | unit_network.rs                    |    51 |  1650 |     2 | Unit tests for network module         |
| unit   | unit_network_config.rs             |   167 |  5564 |     4 | Test file                             |
| unit   | unit_nickname.rs                   |   236 |  8550 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                      |   113 |  3428 |     3 | Unit tests for peers module           |
| unit   | unit_relay.rs                      |    51 |  1450 |     2 | Test file                             |
| unit   | unit_reply.rs                      |   215 |  7626 |     3 | Test file                             |
| unit   | unit_settings.rs                   |   173 |  5340 |     4 | Test file                             |
| unit   | unit_shares.rs                     |  1662 | 59364 |     6 | shares module tests                   |
| unit   | unit_sort.rs                       |   123 |  2770 |     3 | Test file                             |
| unit   | unit_swarm_handler.rs              |   461 | 15592 |     4 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs                |   694 | 23223 |     5 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs           |   415 | 13085 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                   |   340 | 10961 |     3 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs             |    99 |  3105 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                      |   425 | 12920 |     3 | Unit tests for types module           |
| unit   | unit_web_mod.rs                    |  1304 | 45284 |     9 | Test file                             |

**Total:** 70 test files, 38,486 lines, 1,307,736 characters

## Dart Source Files

| Folder       | File                             | Depth | Chars | Lines | Testable | Covered | Purpose                          |
|:-------------|:---------------------------------|------:|------:|------:|---------:|--------:|---------------------------------:|
| lib          | main.dart                        |     5 |  3163 |   108 |       40 |  50.00% | Flutter app entry point          |
| lib          | chat_message_ext.dart            |     3 |  2342 |    60 |       25 | 100.00% | Hand-written message helpers     |
| lib          | frb_node_api.dart                |     5 | 19599 |   672 |      214 |   5.61% | In-process FRB transport         |
| lib          | http_node_api.dart               |     8 | 42905 |  1340 |      581 |  50.95% | WebSocket/HTTP transport         |
| lib          | location_json.dart               |     5 |  2051 |    53 |       33 |   0.00% | Dart source file                 |
| lib          | node_api.dart                    |     4 | 18353 |   523 |        1 | 100.00% | Transport-agnostic node API      |
| lib          | node_transport.dart              |     4 |  3390 |   105 |       27 |   0.00% | Duplex transport abstraction     |
| lib          | shares_json.dart                 |     5 |  3389 |    93 |       57 |  80.70% | Dart source file                 |
| lib/src/rust | api.dart                         |     5 | 41655 |  1175 |      255 |   6.27% | FRB API bindings (generated)     |
| lib/src/rust | frb_generated.dart               |     6 | 224532 |  7180 |     2932 |   0.07% | flutter_rust_bridge codegen      |
| lib/src/rust | frb_generated.io.dart            |     3 | 30227 |  1253 |        5 |   0.00% | FRB IO bindings (generated)      |
| lib/src/rust | frb_generated.web.dart           |     2 | 30147 |  1253 |        - |       - | FRB web bindings (generated)     |
| lib/src/rust | location.dart                    |     5 |  5400 |   173 |       52 |   1.92% | Dart source file                 |
| lib/src/rust | messages.dart                    |     5 |  1067 |    32 |        - |       - | Rust bindings (hand-maintained)  |
| lib/src/rust | mobile_api.dart                  |     5 | 16367 |   502 |      141 |   5.67% | Mobile API bindings (generated)  |
| lib/src/rust | mobile_node.dart                 |     5 | 18150 |   536 |      162 |   4.32% | Mobile node bindings (generated) |
| lib/src/rust | net_diag.dart                    |     5 |  7968 |   256 |       81 |   2.47% | Dart source file                 |
| lib/src/rust | network_config.dart              |     5 |  1182 |    43 |       10 |  10.00% | Dart source file                 |
| lib/src/rust | notifications.dart               |     5 |  2232 |    76 |       23 |   8.70% | Dart source file                 |
| lib/src/rust | reactions.dart                   |     5 |  1113 |    37 |        9 |  11.11% | Dart source file                 |
| lib/src/rust | reply.dart                       |     5 |  1697 |    47 |        9 |  77.78% | Dart source file                 |
| lib/src/rust | settings.dart                    |     5 |  1838 |    56 |       15 |   6.67% | Settings screen                  |
| lib/src/rust | shares.dart                      |     5 |  3924 |   145 |       37 |  48.65% | Dart source file                 |
| lib/src/rust | theme.dart                       |     5 |  1851 |    68 |       21 |   0.00% | Dart source file                 |
| lib          | call_screen.dart                 |    17 | 15735 |   485 |      170 |  91.76% | Call overlay screen              |
| lib          | contact_list.dart                |    15 | 14716 |   428 |      181 |  99.45% | Contacts roster screen           |
| lib          | dm_chat.dart                     |    12 | 26800 |   754 |      354 |  63.84% | Direct-message chat screen       |
| lib          | group_chat.dart                  |     8 | 12409 |   366 |      176 |  50.00% | Group chat screen                |
| lib          | group_list.dart                  |    17 |  7931 |   235 |       99 |  98.99% | Group list screen                |
| lib          | home.dart                        |     9 | 50758 |  1470 |      680 |  43.82% | App shell & broadcast chat       |
| lib          | location_share.dart              |    13 | 14465 |   409 |      180 |  77.78% | Dart source file                 |
| lib          | location_track.dart              |    13 |  7843 |   242 |      101 |  98.02% | Dart source file                 |
| lib          | log_tab.dart                     |    10 |  3493 |   123 |       51 |  43.14% | Log tab screen                   |
| lib          | messages.dart                    |    15 | 48622 |  1430 |      532 |  86.09% | Shared chat widgets              |
| lib          | nav_bar.dart                     |     3 |  1252 |    34 |        5 | 100.00% | Tab navigation bar               |
| lib          | peer_info.dart                   |    11 | 17646 |   518 |      196 |  91.33% | Peer Info screen                 |
| lib          | settings.dart                    |    16 | 42971 |  1241 |      488 |  83.20% | Settings screen                  |
| lib          | share_discovery.dart             |    15 |  9090 |   270 |      101 |  77.23% | Dart source file                 |
| lib          | share_list.dart                  |    17 | 34364 |  1034 |      407 |  93.61% | Dart source file                 |
| lib          | share_network.dart               |    12 |  4947 |   151 |       56 | 100.00% | Dart source file                 |
| lib          | share_publish.dart               |    14 |  7826 |   240 |      100 |  98.00% | Dart source file                 |
| lib          | attachments.dart                 |     4 |  3656 |    92 |       25 |  44.00% | File picker & opener seams       |
| lib          | blocklist.dart                   |     2 |   755 |    19 |        6 | 100.00% | Dart source file                 |
| lib          | call_audio.dart                  |     4 |  4139 |   104 |       27 |  96.30% | Dart source file                 |
| lib          | call_audio_device.dart           |     2 |   436 |    11 |        1 | 100.00% | Dart source file                 |
| lib          | call_audio_platform.dart         |     2 |   850 |    26 |        6 | 100.00% | Dart source file                 |
| lib          | call_audio_platform_android.dart |     4 |  4054 |   106 |       42 |  95.24% | Dart source file                 |
| lib          | call_audio_platform_web.dart     |     5 |  7995 |   216 |        - |       - | Dart source file                 |
| lib          | event_bus.dart                   |     2 |  1813 |    45 |        8 |  75.00% | Node event fan-out               |
| lib          | location.dart                    |     3 |  4274 |   117 |       18 |  66.67% | Dart source file                 |
| lib          | location_device.dart             |     2 |   795 |    17 |        1 | 100.00% | Dart source file                 |
| lib          | location_device_io.dart          |     5 |  4064 |   119 |       40 |  85.00% | Dart source file                 |
| lib          | location_device_web.dart         |     6 |  6872 |   208 |        - |       - | Dart source file                 |
| lib          | location_export.dart             |     2 |  1230 |    35 |        5 |  60.00% | Dart source file                 |
| lib          | location_session.dart            |     7 | 10517 |   328 |      119 |  88.24% | Dart source file                 |
| lib          | notifications.dart               |     3 |  5804 |   162 |       36 |  72.22% | Dart source file                 |
| lib          | notifications_platform.dart      |     0 |   931 |    18 |        3 |   0.00% | Dart source file                 |
| lib          | notifications_platform_web.dart  |     3 |  3244 |    83 |        - |       - | Dart source file                 |
| lib          | reply.dart                       |     3 |  2238 |    44 |       12 |  91.67% | Dart source file                 |
| lib          | share_folders.dart               |     0 |   678 |    16 |        2 |  50.00% | Dart source file                 |
| lib          | theme.dart                       |     2 |  1821 |    49 |        8 |  12.50% | Dart source file                 |
| lib          | ui_utils.dart                    |     2 |  1233 |    29 |        6 |  83.33% | Shared formatting & env helpers  |

**Total:** 62 files, 27,060 lines, 872,809 characters (3729/8972 testable lines covered, 42%)

## Dart Test Files

| Folder  | File                             | Lines | Chars | Depth | Description                           |
|:--------|:---------------------------------|------:|------:|------:|--------------------------------------:|
| helpers | fake_node_api.dart               |  1139 | 35537 |     4 | In-memory node fake                   |
| unit    | api_test.dart                    |    61 |  2389 |     4 | Dart API layer unit tests             |
| unit    | attachments_test.dart            |   132 |  4053 |     5 | Attachment helper tests               |
| unit    | blocklist_test.dart              |    47 |  1429 |     4 | Dart test file                        |
| unit    | call_audio_android_test.dart     |   167 |  5393 |     5 | Dart test file                        |
| unit    | call_audio_test.dart             |    93 |  3466 |     5 | Dart test file                        |
| unit    | http_node_api_test.dart          |   982 | 32370 |    10 | Web transport wire-contract tests     |
| unit    | location_session_test.dart       |   329 | 10019 |     6 | Dart test file                        |
| unit    | location_test.dart               |   186 |  6323 |     6 | Dart test file                        |
| unit    | notifications_test.dart          |   120 |  4374 |     3 | Dart test file                        |
| widget  | attachment_chip_test.dart        |    91 |  2630 |     7 | Attachment chip widget tests          |
| widget  | call_screen_test.dart            |   593 | 18895 |    12 | Call screen widget tests              |
| widget  | contact_list_test.dart           |   673 | 22593 |     7 | Contacts roster widget tests          |
| widget  | device_binding_test.dart         |    72 |  1990 |     5 | Device-binding prompt tests           |
| widget  | group_list_test.dart             |   233 |  7128 |     5 | Group list widget tests               |
| widget  | home_last_connection_test.dart   |    59 |  2033 |     4 | Dart test file                        |
| widget  | home_share_delete_test.dart      |    85 |  2842 |     5 | Dart test file                        |
| widget  | home_share_folder_test.dart      |   120 |  4433 |     7 | Dart test file                        |
| widget  | location_share_test.dart         |   188 |  5999 |     4 | Dart test file                        |
| widget  | location_track_test.dart         |   149 |  4523 |     4 | Dart test file                        |
| widget  | messages_test.dart               |  1584 | 46553 |    10 | Chat widget tests                     |
| widget  | peer_info_test.dart              |   344 | 10585 |     6 | Peer Info widget tests                |
| widget  | settings_network_peers_test.dart |   128 |  4028 |     3 | Dart test file                        |
| widget  | settings_test.dart               |   514 | 15959 |     6 | Settings widget tests                 |
| widget  | share_discovery_test.dart        |   231 |  7265 |     4 | Dart test file                        |
| widget  | share_list_test.dart             |   968 | 29660 |     9 | Dart test file                        |
| widget  | share_network_test.dart          |   168 |  4841 |     7 | Dart test file                        |
| widget  | share_publish_test.dart          |   321 |  9700 |     6 | Dart test file                        |
| test    | widget_test.dart                 |    21 |   624 |     2 | Widget smoke tests                    |

**Total:** 29 test files, 9,798 lines, 307,634 characters

## Kotlin Source Files

| Folder                      | File                    | Depth | Chars | Lines | Testable | Covered | Purpose                                      |
|:----------------------------|:------------------------|------:|------:|------:|---------:|--------:|---------------------------------------------:|
| com/example/p2p_app_flutter | CallAudio.kt            |     6 | 10250 |   286 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | LocationSensor.kt       |     7 | 16923 |   436 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | MainActivity.kt         |     5 |  4694 |   126 |        - |       - | Flutter activity & method channel bridge     |
| com/example/p2p_app_flutter | MessageNotifications.kt |     4 |  3431 |    85 |        - |       - | Kotlin source file                           |
| com/example/p2p_app_flutter | P2pForegroundService.kt |     4 |  4282 |   127 |        - |       - | Foreground service for background networking |

**Total:** 5 files, 1,060 lines, 39,580 characters

## Kotlin Test Files

| Folder                      | File                           | Lines | Chars | Depth | Description                           |
|:----------------------------|:-------------------------------|------:|------:|------:|--------------------------------------:|
| com/example/p2p_app_flutter | CallAudioTest.kt               |    51 |  1765 |     3 | Kotlin test file                      |
| com/example/p2p_app_flutter | LocationSensorTest.kt          |   105 |  3771 |     5 | Kotlin test file                      |
| com/example/p2p_app_flutter | MainActivityTest.kt            |    80 |  2878 |     3 | MainActivity unit tests               |
| com/example/p2p_app_flutter | P2pForegroundServiceTest.kt    |    99 |  3093 |     3 | ForegroundService unit tests          |

**Total:** 4 test files, 335 lines, 11,507 characters
