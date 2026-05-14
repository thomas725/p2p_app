# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      37 |
| Total Lines of Code     |   8,395 |
| Total Characters        | 292,284 |
| Average Lines per File  |     226 |
| Average Characters/File |   7,899 |

## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   120 |  4133 |     5 |    -  | Build script                        |
| src                     | behavior.rs          |   142 |  4936 |     4 |  100% | Network behavior definitions        |
| src                     | db.rs                |   392 | 14042 |     5 |   53% | Database connection & identity mgmt |
| src                     | dioxus_app.rs        |   675 | 29856 |    16 |    -  | Source file                         |
| src                     | fmt.rs               |   111 |  3402 |     4 |  100% | Formatting & display utilities      |
| src                     | lib.rs               |   382 | 12641 |     3 |   70% | Module declarations & re-exports    |
| src                     | logging.rs           |   261 |  7912 |     4 |  100% | Logging utilities & setup           |
| src                     | messages.rs          |   224 |  8375 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    55 |  1735 |     3 |   89% | Network size classification         |
| src                     | nickname.rs          |   138 |  4974 |     3 |  100% | Nickname management                 |
| src                     | peers.rs             |   178 |  6092 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   306 | 10428 |     7 |   45% | Network event translation           |
| src                     | tui_helpers.rs       |   268 |  8033 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   347 | 11389 |     5 |    -  | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   601 | 18505 |     4 |   43% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   414 | 11951 |     5 |   51% | Tab management & navigation         |
| src                     | tui_test_state.rs    |   237 |  6895 |     6 |    -  | TUI test state & mouse handling     |
| src                     | types.rs             |   246 |  7426 |     4 |  100% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   158 |  5800 |     7 |    -  | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   256 |  9952 |     8 |    -  | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   137 |  5242 |     4 |    -  | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   674 | 23747 |     7 |   33% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   256 |  9889 |     6 |    -  | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    16 |   526 |     0 |    -  | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    44 |  1639 |     6 |    -  | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   318 | 10976 |     5 |    -  | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   266 |  9832 |     5 |    -  | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   107 |  3911 |     5 |    -  | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   274 |  9470 |     5 |    -  | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   220 |  9089 |     7 |    -  | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    50 |  1708 |     3 |    -  | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   176 |  5938 |     5 |    -  | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |    42 |  1653 |     1 |    -  | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |    -  | Module declarations                 |
| src/generated           | models_insertable.rs |   102 |  3750 |     1 |    -  | Insertable data models              |
| src/generated           | models_queryable.rs  |   120 |  4379 |     1 |    -  | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |    -  | Database schema (Diesel)            |

**Total:** 37 files, 8,395 lines, 292,284 characters

## Test Files

| Folder | File                      | Lines | Chars | Depth | Description                   |
|:-------|:--------------------------|------:|------:|------:|------------------------------:|
| models | insertable_tests.rs       |    77 |  2371 |     3 | Diesel insertable model tests |
| models | queryable_tests.rs        |   156 |  4797 |     3 | Diesel queryable model tests  |
| tests  | additional_coverage.rs    |   119 |  3846 |     2 | Additional coverage tests     |
| tests  | behavior.rs               |   172 |  4860 |     4 | behavior module tests         |
| tests  | db.rs                     |   206 |  6123 |     2 | database module tests         |
| tests  | db_selection.rs           |    59 |  1715 |     3 | Database selection tests      |
| tests  | fmt.rs                    |   159 |  4281 |     2 | fmt module tests              |
| tests  | logging.rs                |   332 |  9539 |     3 | logging module tests          |
| tests  | logging_config.rs         |     6 |   191 |     1 | Test file                     |
| tests  | messages.rs               |   481 | 14944 |     2 | messages module tests         |
| tests  | network.rs                |    49 |  1638 |     1 | network module tests          |
| tests  | nickname.rs               |   388 | 12680 |     3 | nickname module tests         |
| tests  | p2p_integration.rs        |  1006 | 35991 |    10 | P2P integration tests         |
| tests  | peers.rs                  |   299 |  8696 |     2 | peers module tests            |
| tests  | swarm_handler.rs          |   102 |  3282 |     2 | Test file                     |
| tests  | test_utils.rs             |    63 |  1921 |     3 | Test utilities                |
| tests  | tui_binary_integration.rs |   284 |  8825 |     3 | TUI binary integration tests  |
| tests  | tui_chat.rs               |   788 | 25091 |     4 | TUI chat functionality tests  |
| tests  | tui_helpers.rs            |   762 | 22781 |     3 | TUI helpers tests             |
| tests  | tui_integration.rs        |   437 | 13897 |     3 | TUI integration tests         |
| tests  | tui_render_integration.rs |   640 | 19695 |     5 | TUI render integration tests  |
| tests  | tui_state.rs              |   309 |  9373 |     2 | TUI state tests               |
| tests  | tui_tasks.rs              |   233 |  7533 |     7 | TUI task tests                |
| tests  | types.rs                  |   152 |  4074 |     2 | types module tests            |

**Total:** 24 test files, 7,279 lines, 228,144 characters
