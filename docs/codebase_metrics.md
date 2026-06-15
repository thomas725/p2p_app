# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      37 |
| Total Lines of Code     |   6,797 |
| Total Characters        | 234,190 |
| Average Lines per File  |     183 |
| Average Characters/File |   6,329 |

## All Source Files

| Folder                  | File                 | Depth | Chars | Lines | Testable | Covered | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|---------:|--------:|------------------------------------:|
| /                       | build.rs             |     5 |  3949 |   118 |        0 |     - | Build script                        |
| src                     | behavior.rs          |     4 |  6157 |   181 |       25 |   64% | Network behavior definitions        |
| src                     | db.rs                |     6 | 14561 |   401 |      149 |   93% | Database connection & identity mgmt |
| src                     | dioxus_app.rs        |    11 | 19052 |   537 |      178 |  0.0% | Web UI app shell & components (Dio… |
| src                     | dioxus_styles.rs     |     0 |  3753 |    47 |        0 |     - | Web UI CSS styles (Dioxus)          |
| src                     | dioxus_swarm.rs      |     5 |  6079 |   160 |       98 |  0.0% | Web UI swarm event handling (Dioxu… |
| src                     | fmt.rs               |     4 |  3547 |   117 |       39 |   95% | Formatting & display utilities      |
| src                     | lib.rs               |     1 |  5586 |   147 |        1 |  100% | Module declarations & re-exports    |
| src                     | logging.rs           |     4 |  7042 |   238 |       86 |  100% | Logging utilities & setup           |
| src                     | messages.rs          |     4 |  8406 |   227 |       84 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |     3 |  1851 |    61 |       13 |  100% | Network size classification         |
| src                     | nickname.rs          |     5 |  3445 |    98 |       49 |  100% | Nickname management                 |
| src                     | peers.rs             |     3 |  6171 |   182 |       64 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |     7 |  9362 |   272 |      117 |  5.1% | Network event translation           |
| src                     | tui_helpers.rs       |     3 |  6896 |   226 |       91 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |     5 | 11816 |   363 |      185 |   88% | TUI rendering & state management    |
| src                     | tui_render_state.rs  |     4 | 11457 |   372 |      136 |   98% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |     5 |  4793 |   168 |       52 |   98% | Tab management & navigation         |
| src                     | types.rs             |     2 |  3279 |   110 |        3 |   67% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |     7 |  4936 |   134 |       62 |  0.0% | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |     8 |  8206 |   215 |      112 |  0.0% | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |     4 |  4511 |   115 |       18 |  0.0% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |     5 |  5185 |   137 |       77 |  100% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |     6 | 10698 |   321 |      158 |  100% | Event routing & state updates       |
| src/bin/tui             | event_source.rs      |     4 |  1224 |    40 |       12 |   42% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |     5 | 12090 |   356 |      174 |   82% | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |     4 | 10740 |   305 |      143 |   28% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |     5 |  4473 |   138 |       73 |  100% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |     5 |  8766 |   252 |      110 |   98% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |     3 |  8341 |   226 |       40 |   95% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |     3 |  1882 |    56 |       23 |  0.0% | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |     4 |  5074 |   151 |       62 |   48% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |     1 |  1653 |    42 |        0 |     - | Auto-generated column definitions   |
| src/generated           | mod.rs               |     0 |   488 |    11 |        0 |     - | Module declarations                 |
| src/generated           | models_insertable.rs |     1 |  3256 |    92 |        0 |     - | Insertable data models              |
| src/generated           | models_queryable.rs  |     1 |  3895 |   110 |        0 |     - | Queryable data models               |
| src/generated           | schema.rs            |     2 |  1570 |    71 |       40 |   98% | Database schema (Diesel)            |

**Total:** 37 files, 6,797 lines, 234,190 characters (1645/2474 testable lines covered, 66%)

## Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | db_test_utils.rs                  |     8 |   209 |     2 | Test file                             |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   241 |  6963 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   119 |  3892 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   176 |  4967 |     5 | behavior module tests                 |
| tests  | db.rs                             |   188 |  5305 |     3 | database module tests                 |
| tests  | db_selection.rs                   |    57 |  1737 |     4 | Database selection tests              |
| tests  | fmt.rs                            |   291 |  7626 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   296 |  8328 |     3 | logging module tests                  |
| tests  | messages.rs                       |   496 | 14906 |     3 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   398 | 12478 |     4 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1021 | 36271 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   322 |  8933 |     3 | peers module tests                    |
| tests  | swarm_handler.rs                  |   102 |  3282 |     2 | swarm_handler module tests            |
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
| unit   | unit_bin_tui_click_handlers.rs    |   205 |  7118 |     2 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   751 | 23456 |     4 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   565 | 16610 |     3 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   236 |  7238 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   250 |  7437 |     3 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   193 |  5783 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   497 | 15673 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   223 |  6335 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    62 |  1896 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_db.rs                        |   281 |  8768 |     4 | Unit tests for database module        |
| unit   | unit_lib.rs                       |   264 |  7191 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   203 |  5147 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   190 |  6084 |     4 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    46 |  1371 |     3 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   100 |  3032 |     3 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |    80 |  2260 |     3 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |    98 |  2844 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   126 |  3954 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   256 |  7337 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   193 |  5390 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   254 |  6822 |     3 | Unit tests for types module           |

**Total:** 50 test files, 13,346 lines, 408,732 characters
