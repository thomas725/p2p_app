# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      36 |
| Total Lines of Code     |   7,100 |
| Total Characters        | 248,604 |
| Average Lines per File  |     197 |
| Average Characters/File |   6,905 |

## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   120 |  4099 |     5 |    -  | Build script                        |
| src                     | behavior.rs          |   181 |  6157 |     4 |   97% | Network behavior definitions        |
| src                     | db.rs                |   401 | 14561 |     6 |   52% | Database connection & identity mgmt |
| src                     | dioxus_app.rs        |   668 | 29738 |    16 |    -  | Source file                         |
| src                     | fmt.rs               |   111 |  3402 |     4 |  100% | Formatting & display utilities      |
| src                     | lib.rs               |   143 |  5467 |     1 |  2.1% | Module declarations & re-exports    |
| src                     | logging.rs           |   243 |  7141 |     4 |  100% | Logging utilities & setup           |
| src                     | messages.rs          |   227 |  8406 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    59 |  1782 |     3 |   88% | Network size classification         |
| src                     | nickname.rs          |   112 |  3900 |     3 |  100% | Nickname management                 |
| src                     | peers.rs             |   182 |  6157 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   270 |  9285 |     7 |   39% | Network event translation           |
| src                     | tui_helpers.rs       |   225 |  6864 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   358 | 11734 |     5 |    -  | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   365 | 11233 |     4 |  0.8% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   205 |  5427 |     5 |  1.5% | Tab management & navigation         |
| src                     | types.rs             |    85 |  2574 |     2 |  100% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   134 |  4927 |     7 |    -  | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   228 |  8833 |     8 |    -  | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   116 |  4516 |     4 |  2.6% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   475 | 16848 |     7 |  0.6% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   327 | 10771 |     6 |  0.9% | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    20 |   648 |     2 |    -  | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    40 |  1232 |     4 |  7.5% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   357 | 12158 |     5 |  0.8% | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   301 | 10677 |     4 |  1.0% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   139 |  4550 |     5 |  2.2% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   251 |  8798 |     5 |  1.2% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   232 |  9100 |     3 |  1.3% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    56 |  1882 |     3 |    -  | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   143 |  4876 |     4 |  2.1% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |    42 |  1652 |     1 |    -  | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |    -  | Module declarations                 |
| src/generated           | models_insertable.rs |    92 |  3256 |     1 |    -  | Insertable data models              |
| src/generated           | models_queryable.rs  |   110 |  3895 |     1 |    -  | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |    -  | Database schema (Diesel)            |

**Total:** 36 files, 7,100 lines, 248,604 characters

## Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| shared | db_test_utils.rs                  |     8 |   209 |     2 | Test file                             |
| shared | logging_test_utils.rs             |    28 |  1109 |     2 | Test file                             |
| shared | tui_test_state.rs                 |   241 |  6963 |     6 | Test file                             |
| tests  | additional_coverage.rs            |   119 |  3892 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   172 |  4860 |     4 | behavior module tests                 |
| tests  | db.rs                             |   206 |  6123 |     2 | database module tests                 |
| tests  | db_selection.rs                   |    59 |  1715 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   291 |  7626 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   311 |  8720 |     3 | logging module tests                  |
| tests  | messages.rs                       |   481 | 14944 |     2 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   388 | 12680 |     3 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1023 | 36428 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   299 |  8696 |     2 | peers module tests                    |
| tests  | swarm_handler.rs                  |   102 |  3282 |     2 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    63 |  1921 |     3 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   284 |  8871 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   788 | 25091 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   661 | 19240 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   437 | 13897 |     3 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   640 | 19695 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   309 |  9373 |     2 | TUI state tests                       |
| tests  | tui_tasks.rs                      |   233 |  7533 |     7 | TUI task tests                        |
| tests  | types.rs                          |   722 | 22240 |     3 | types module tests                    |
| unit   | unit_behavior.rs                  |    58 |  1823 |     2 | Unit tests for behavior module        |
| unit   | unit_bin_tui_click_handlers.rs    |   700 | 23459 |     3 | Unit tests for TUI click handlers     |
| unit   | unit_bin_tui_command_processor.rs |   314 | 10539 |     2 | Unit tests for TUI command processor  |
| unit   | unit_bin_tui_event_source.rs      |    44 |  1347 |     2 | Unit tests for TUI event source       |
| unit   | unit_bin_tui_input_processor.rs   |   245 |  7489 |     3 | Unit tests for TUI input processor    |
| unit   | unit_bin_tui_main_loop.rs         |   236 |  7195 |     3 | Unit tests for TUI main loop          |
| unit   | unit_bin_tui_message_handlers.rs  |   103 |  3410 |     3 | Unit tests for TUI message handlers   |
| unit   | unit_bin_tui_render_loop_mod.rs   |   188 |  5546 |     3 | Unit tests for TUI render loop        |
| unit   | unit_bin_tui_scroll_handlers.rs   |   418 | 12834 |     3 | Unit tests for TUI scroll handlers    |
| unit   | unit_bin_tui_state.rs             |   211 |  5871 |     3 | Unit tests for TUI state              |
| unit   | unit_bin_tui_test_helpers.rs      |    60 |  1764 |     3 | Unit tests for TUI test helpers       |
| unit   | unit_db.rs                        |   295 |  9186 |     3 | Unit tests for database module        |
| unit   | unit_lib.rs                       |   264 |  7191 |     2 | Unit tests for library re-exports/api |
| unit   | unit_logging.rs                   |   205 |  5211 |     4 | Unit tests for logging module         |
| unit   | unit_messages.rs                  |   187 |  6000 |     3 | Unit tests for messages module        |
| unit   | unit_network.rs                   |    54 |  1532 |     2 | Unit tests for network module         |
| unit   | unit_nickname.rs                  |   103 |  3098 |     2 | Unit tests for nickname module        |
| unit   | unit_peers.rs                     |    84 |  2373 |     2 | Unit tests for peers module           |
| unit   | unit_swarm_handler.rs             |    78 |  2258 |     3 | Unit tests for swarm_handler module   |
| unit   | unit_tui_helpers.rs               |   126 |  3876 |     3 | Unit tests for TUI helpers            |
| unit   | unit_tui_render_state.rs          |   256 |  7322 |     2 | Unit tests for TUI render state       |
| unit   | unit_tui_tabs.rs                  |   210 |  5887 |     2 | Unit tests for TUI tabs               |
| unit   | unit_tui_test_state.rs            |    99 |  3073 |     2 | Unit tests for TUI test state         |
| unit   | unit_types.rs                     |   183 |  4960 |     3 | Unit tests for types module           |

**Total:** 50 test files, 12,868 lines, 397,158 characters
