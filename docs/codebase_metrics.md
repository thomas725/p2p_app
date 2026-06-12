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

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   118 |  3949 |     5 |     - | Build script                        |
| src                     | behavior.rs          |   181 |  6157 |     4 |   64% | Network behavior definitions        |
| src                     | db.rs                |   401 | 14561 |     6 |   91% | Database connection & identity mgmt |
| src                     | dioxus_app.rs        |   537 | 19052 |    11 |  0.0% | Source file                         |
| src                     | dioxus_styles.rs     |    47 |  3753 |     0 |     - | Source file                         |
| src                     | dioxus_swarm.rs      |   160 |  6079 |     5 |  0.0% | Source file                         |
| src                     | fmt.rs               |   117 |  3547 |     4 |   95% | Formatting & display utilities      |
| src                     | lib.rs               |   147 |  5586 |     1 |  100% | Module declarations & re-exports    |
| src                     | logging.rs           |   238 |  7042 |     4 |  100% | Logging utilities & setup           |
| src                     | messages.rs          |   227 |  8406 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    61 |  1851 |     3 |  100% | Network size classification         |
| src                     | nickname.rs          |    98 |  3445 |     5 |  100% | Nickname management                 |
| src                     | peers.rs             |   182 |  6171 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   272 |  9362 |     7 |  5.1% | Network event translation           |
| src                     | tui_helpers.rs       |   226 |  6896 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   363 | 11816 |     5 |   88% | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   372 | 11457 |     4 |   98% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   168 |  4793 |     5 |   98% | Tab management & navigation         |
| src                     | types.rs             |   110 |  3279 |     2 |   67% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   134 |  4936 |     7 |  0.0% | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   215 |  8206 |     8 |  0.0% | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   115 |  4511 |     4 |  0.0% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   137 |  5185 |     5 |  100% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   321 | 10698 |     6 |  100% | Event routing & state updates       |
| src/bin/tui             | event_source.rs      |    40 |  1224 |     4 |   42% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   356 | 12090 |     5 |   82% | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   305 | 10740 |     4 |   28% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   138 |  4473 |     5 |  100% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   252 |  8766 |     5 |   92% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   226 |  8341 |     3 |   95% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    56 |  1882 |     3 |  0.0% | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   151 |  5074 |     4 |   48% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |    42 |  1653 |     1 |     - | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |     - | Module declarations                 |
| src/generated           | models_insertable.rs |    92 |  3256 |     1 |     - | Insertable data models              |
| src/generated           | models_queryable.rs  |   110 |  3895 |     1 |     - | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |   98% | Database schema (Diesel)            |

**Total:** 37 files, 6,797 lines, 234,190 characters (1636/2474 testable lines covered, 66%)

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
| unit   | unit_bin_tui_input_processor.rs   |   504 | 15001 |     3 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   236 |  7238 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   248 |  7445 |     3 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   193 |  5783 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   457 | 14257 |     3 | Unit tests for TUI scroll handlers    |
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

**Total:** 50 test files, 13,243 lines, 405,715 characters
