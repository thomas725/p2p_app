# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      38 |
| Total Lines of Code     |   7,637 |
| Total Characters        | 257,562 |
| Average Lines per File  |     200 |
| Average Characters/File |   6,777 |

## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   120 |  4133 |     5 |    -  | Build script                        |
| src                     | behavior.rs          |   142 |  4936 |     4 |   89% | Network behavior definitions        |
| src                     | db.rs                |   402 | 14245 |     5 |   28% | Database connection & identity mgmt |
| src                     | fmt.rs               |   111 |  3400 |     4 |  100% | Formatting & display utilities      |
| src                     | lib.rs               |   382 | 12639 |     3 |   70% | Module declarations & re-exports    |
| src                     | logging.rs           |   229 |  6677 |     4 |  100% | Logging utilities & setup           |
| src                     | logging_config.rs    |    66 |  2515 |     2 |   97% | Tracing configuration               |
| src                     | messages.rs          |   230 |  8437 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    44 |  1414 |     3 |  100% | Network size classification         |
| src                     | nickname.rs          |   201 |  6997 |     3 |  100% | Nickname management                 |
| src                     | peers.rs             |   179 |  6111 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   315 | 10898 |     7 |   39% | Network event translation           |
| src                     | tui_events.rs        |   154 |  4916 |     3 |  100% | Event/command types & channels      |
| src                     | tui_helpers.rs       |   255 |  7875 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   343 | 11355 |     5 |    -  | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   546 | 16717 |     4 |   37% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   333 |  9560 |     5 |   42% | Tab management & navigation         |
| src                     | tui_test_state.rs    |   219 |  6579 |     6 |    -  | TUI test state & mouse handling     |
| src                     | types.rs             |   246 |  7426 |     4 |  100% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   159 |  5823 |     7 |    -  | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |    11 |   401 |     1 |    -  | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   138 |  5235 |     4 |    -  | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   682 | 23928 |     7 |   33% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   260 | 10064 |     6 |    -  | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    16 |   526 |     0 |    -  | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    44 |  1631 |     6 |    -  | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   318 | 10992 |     5 |    -  | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   267 |  9849 |     5 |    -  | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   107 |  3919 |     5 |    -  | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   282 |  9586 |     5 |    -  | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   227 |  9290 |     7 |    -  | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    50 |  1710 |     3 |    -  | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   178 |  6026 |     5 |    -  | Render loop orchestration (60 FPS)  |
| src/generated           | columns.rs           |    77 |  1687 |     1 |    -  | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |    -  | Module declarations                 |
| src/generated           | models_insertable.rs |   102 |  3689 |     1 |    -  | Insertable data models              |
| src/generated           | models_queryable.rs  |   120 |  4318 |     1 |    -  | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |    -  | Database schema (Diesel)            |

**Total:** 38 files, 7,637 lines, 257,562 characters

## Test Files

| Folder | File                      | Lines | Chars | Depth | Description                   |
|:-------|:--------------------------|------:|------:|------:|------------------------------:|
| models | insertable_tests.rs       |    77 |  2371 |     3 | Diesel insertable model tests |
| models | queryable_tests.rs        |   156 |  4797 |     3 | Diesel queryable model tests  |
| tests  | additional_coverage.rs    |   119 |  3866 |     2 | Additional coverage tests     |
| tests  | behavior.rs               |   127 |  3613 |     4 | behavior module tests         |
| tests  | db.rs                     |   111 |  3274 |     2 | database module tests         |
| tests  | db_selection.rs           |    59 |  1721 |     3 | Database selection tests      |
| tests  | fmt.rs                    |   141 |  3755 |     2 | fmt module tests              |
| tests  | logging.rs                |   258 |  7421 |     3 | logging module tests          |
| tests  | messages.rs               |   421 | 12839 |     2 | messages module tests         |
| tests  | network.rs                |    46 |  1419 |     1 | network module tests          |
| tests  | nickname.rs               |   298 |  9827 |     3 | nickname module tests         |
| tests  | p2p_integration.rs        |  1014 | 36315 |    10 | P2P integration tests         |
| tests  | peers.rs                  |   231 |  6768 |     2 | peers module tests            |
| tests  | swarm_handler.rs          |    49 |  1410 |     2 | Test file                     |
| tests  | test_utils.rs             |    59 |  1855 |     3 | Test utilities                |
| tests  | tui_binary_integration.rs |   284 |  8853 |     3 | TUI binary integration tests  |
| tests  | tui_chat.rs               |   789 | 25107 |     4 | TUI chat functionality tests  |
| tests  | tui_events.rs             |   182 |  5613 |     4 | TUI events tests              |
| tests  | tui_helpers.rs            |   649 | 19258 |     3 | TUI helpers tests             |
| tests  | tui_integration.rs        |   443 | 14004 |     4 | TUI integration tests         |
| tests  | tui_render_integration.rs |   363 | 11227 |     5 | TUI render integration tests  |
| tests  | tui_state.rs              |   249 |  7557 |     2 | TUI state tests               |
| tests  | tui_tasks.rs              |   234 |  7567 |     7 | TUI task tests                |
| tests  | types.rs                  |   104 |  3099 |     2 | types module tests            |

**Total:** 24 test files, 6,463 lines, 203,536 characters
