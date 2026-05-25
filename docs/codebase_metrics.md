# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      37 |
| Total Lines of Code     |   7,527 |
| Total Characters        | 261,208 |
| Average Lines per File  |     203 |
| Average Characters/File |   7,059 |

## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   120 |  4133 |     5 |    -  | Build script                        |
| src                     | behavior.rs          |   146 |  4999 |     4 |  100% | Network behavior definitions        |
| src                     | db.rs                |   413 | 14826 |     6 |   51% | Database connection & identity mgmt |
| src                     | dioxus_app.rs        |   675 | 29856 |    16 |    -  | Source file                         |
| src                     | fmt.rs               |   111 |  3402 |     4 |  100% | Formatting & display utilities      |
| src                     | lib.rs               |   122 |  4738 |     1 |  2.5% | Module declarations & re-exports    |
| src                     | logging.rs           |   238 |  7319 |     4 |  100% | Logging utilities & setup           |
| src                     | messages.rs          |   228 |  8438 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    59 |  1797 |     3 |   88% | Network size classification         |
| src                     | nickname.rs          |   142 |  5037 |     3 |  100% | Nickname management                 |
| src                     | peers.rs             |   182 |  6152 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   272 |  9322 |     7 |   39% | Network event translation           |
| src                     | tui_helpers.rs       |   272 |  8099 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   347 | 11389 |     5 |    -  | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   348 | 10478 |     4 |  0.9% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   205 |  5401 |     5 |  1.5% | Tab management & navigation         |
| src                     | tui_test_state.rs    |   241 |  6964 |     6 |  1.2% | TUI test state & mouse handling     |
| src                     | types.rs             |    91 |  2794 |     2 |  100% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   158 |  5800 |     7 |    -  | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   256 |  9952 |     8 |    -  | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   140 |  5345 |     4 |  2.1% | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   466 | 16579 |     7 |  0.6% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   325 | 11320 |     6 |  0.9% | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    16 |   526 |     0 |    -  | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    39 |  1192 |     4 |  7.7% | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   348 | 11888 |     5 |  0.9% | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   291 | 10198 |     4 |  1.0% | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   143 |  4583 |     5 |  2.1% | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   283 |  9740 |     5 |  1.1% | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   239 |  9334 |     6 |  1.3% | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    50 |  1708 |     3 |    -  | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   180 |  6025 |     5 |  1.7% | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |    77 |  1687 |     1 |    -  | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |    -  | Module declarations                 |
| src/generated           | models_insertable.rs |   102 |  3750 |     1 |    -  | Insertable data models              |
| src/generated           | models_queryable.rs  |   120 |  4379 |     1 |    -  | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |    -  | Database schema (Diesel)            |

**Total:** 37 files, 7,527 lines, 261,208 characters

## Test Files

| Folder | File                              | Lines | Chars | Depth | Description                           |
|:-------|:----------------------------------|------:|------:|------:|--------------------------------------:|
| models | insertable_tests.rs               |    77 |  2371 |     3 | Diesel insertable model tests         |
| models | queryable_tests.rs                |   156 |  4797 |     3 | Diesel queryable model tests          |
| tests  | additional_coverage.rs            |   119 |  3846 |     2 | Additional coverage tests             |
| tests  | behavior.rs                       |   172 |  4860 |     4 | behavior module tests                 |
| tests  | db.rs                             |   206 |  6123 |     2 | database module tests                 |
| tests  | db_selection.rs                   |    59 |  1715 |     3 | Database selection tests              |
| tests  | fmt.rs                            |   306 |  7976 |     2 | fmt module tests                      |
| tests  | logging.rs                        |   311 |  8837 |     3 | logging module tests                  |
| tests  | messages.rs                       |   481 | 14944 |     2 | messages module tests                 |
| tests  | network.rs                        |    49 |  1638 |     1 | network module tests                  |
| tests  | nickname.rs                       |   388 | 12680 |     3 | nickname module tests                 |
| tests  | p2p_integration.rs                |  1023 | 36428 |    10 | P2P integration tests                 |
| tests  | peers.rs                          |   299 |  8696 |     2 | peers module tests                    |
| tests  | swarm_handler.rs                  |   102 |  3282 |     2 | swarm_handler module tests            |
| tests  | test_utils.rs                     |    63 |  1921 |     3 | Test utilities                        |
| tests  | tui_binary_integration.rs         |   284 |  8825 |     3 | TUI binary integration tests          |
| tests  | tui_chat.rs                       |   788 | 25091 |     4 | TUI chat functionality tests          |
| tests  | tui_helpers.rs                    |   762 | 22781 |     3 | TUI helpers tests                     |
| tests  | tui_integration.rs                |   437 | 13897 |     3 | TUI integration tests                 |
| tests  | tui_render_integration.rs         |   640 | 19695 |     5 | TUI render integration tests          |
| tests  | tui_state.rs                      |   309 |  9373 |     2 | TUI state tests                       |
| tests  | tui_tabs_dedicated.rs             |   193 |  5037 |     2 | Dedicated TUI tabs tests              |
| tests  | tui_tasks.rs                      |   233 |  7533 |     7 | TUI task tests                        |
| tests  | tui_test_state_dedicated.rs       |   157 |  4316 |     1 | Dedicated TUI test-state tests        |
| tests  | types.rs                          |   298 |  8028 |     3 | types module tests                    |
| tests  | unit_behavior.rs                  |    58 |  1823 |     2 | Unit tests for behavior module        |
| tests  | unit_bin_tui_click_handlers.rs    |   709 | 26037 |     4 | Unit tests for TUI click handlers     |
| tests  | unit_bin_tui_command_processor.rs |   318 | 11735 |     4 | Unit tests for TUI command processor  |
| tests  | unit_bin_tui_event_source.rs      |    46 |  1518 |     3 | Unit tests for TUI event source       |
| tests  | unit_bin_tui_input_processor.rs   |   166 |  5988 |     4 | Unit tests for TUI input processor    |
| tests  | unit_bin_tui_main_loop.rs         |   220 |  7464 |     4 | Unit tests for TUI main loop          |
| tests  | unit_bin_tui_message_handlers.rs  |   110 |  3873 |     4 | Unit tests for TUI message handlers   |
| tests  | unit_bin_tui_render_loop_mod.rs   |   165 |  5595 |     3 | Unit tests for TUI render loop        |
| tests  | unit_bin_tui_scroll_handlers.rs   |   436 | 15138 |     4 | Unit tests for TUI scroll handlers    |
| tests  | unit_bin_tui_state.rs             |   217 |  6723 |     4 | Unit tests for TUI state              |
| tests  | unit_bin_tui_test_helpers.rs      |    60 |  1764 |     3 | Unit tests for TUI test helpers       |
| tests  | unit_db.rs                        |   104 |  3419 |     1 | Unit tests for database module        |
| tests  | unit_lib.rs                       |   264 |  8131 |     3 | Unit tests for library re-exports/api |
| tests  | unit_logging.rs                   |   128 |  3782 |     5 | Unit tests for logging module         |
| tests  | unit_messages.rs                  |   105 |  3954 |     3 | Unit tests for messages module        |
| tests  | unit_network.rs                   |    54 |  1724 |     3 | Unit tests for network module         |
| tests  | unit_nickname.rs                  |   103 |  3466 |     3 | Unit tests for nickname module        |
| tests  | unit_peers.rs                     |    76 |  2458 |     3 | Unit tests for peers module           |
| tests  | unit_swarm_handler.rs             |    34 |  1146 |     3 | Unit tests for swarm_handler module   |
| tests  | unit_tui_helpers.rs               |   111 |  4528 |     3 | Unit tests for TUI helpers            |
| tests  | unit_tui_render_state.rs          |   253 |  8070 |     3 | Unit tests for TUI render state       |
| tests  | unit_tui_tabs.rs                  |   210 |  6631 |     3 | Unit tests for TUI tabs               |
| tests  | unit_tui_test_state.rs            |    93 |  3030 |     2 | Unit tests for TUI test state         |
| tests  | unit_types.rs                     |   183 |  5588 |     4 | Unit tests for types module           |

**Total:** 49 test files, 12,135 lines, 388,275 characters
