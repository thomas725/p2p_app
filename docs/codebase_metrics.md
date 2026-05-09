# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      41 |
| Total Lines of Code     |   8,049 |
| Total Characters        | 270,618 |
| Average Lines per File  |     196 |
| Average Characters/File |   6,600 |

## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Cover | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   121 |  4147 |     5 |    -  | Build script                        |
| src                     | behavior.rs          |   142 |  4933 |     4 |   89% | Network behavior definitions        |
| src                     | db.rs                |   403 | 14294 |     5 |   28% | Database connection & identity mgmt |
| src                     | fmt.rs               |   111 |  3400 |     4 |  100% | Formatting & display utilities      |
| src                     | lib.rs               |   379 | 12541 |     3 |   70% | Module declarations & re-exports    |
| src                     | logging.rs           |   229 |  6677 |     4 |  100% | Logging utilities & setup           |
| src                     | logging_config.rs    |    66 |  2515 |     2 |   97% | Tracing configuration               |
| src                     | messages.rs          |   230 |  8437 |     4 |  100% | Message persistence & retrieval     |
| src                     | network.rs           |    44 |  1414 |     3 |  100% | Network size classification         |
| src                     | nickname.rs          |   201 |  6997 |     3 |  100% | Nickname management                 |
| src                     | peers.rs             |   179 |  6113 |     3 |  100% | Peer management & tracking          |
| src                     | swarm_handler.rs     |   418 | 14278 |     7 |   37% | Network event translation           |
| src                     | tui_events.rs        |   154 |  4916 |     3 |  100% | Event/command types & channels      |
| src                     | tui_helpers.rs       |   293 |  8662 |     3 |  100% | TUI helper functions & utilities    |
| src                     | tui_render.rs        |   195 |  6221 |     4 |    -  | TUI rendering & state management    |
| src                     | tui_render_state.rs  |   284 |  8383 |     4 |   44% | TUI render state & tab content      |
| src                     | tui_tabs.rs          |   357 | 10222 |     5 |   44% | Tab management & navigation         |
| src                     | tui_test_state.rs    |   219 |  6579 |     6 |    -  | TUI test state & mouse handling     |
| src                     | types.rs             |    88 |  2735 |     2 |  100% | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   159 |  5823 |     7 |    -  | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   205 |  7073 |     8 |    -  | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   139 |  5237 |     4 |    -  | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   505 | 18240 |     7 |   19% | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   260 | 10064 |     6 |    -  | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    16 |   526 |     0 |    -  | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    44 |  1631 |     6 |    -  | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   318 | 10992 |     5 |    -  | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   267 |  9849 |     5 |    -  | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   107 |  3919 |     5 |    -  | Message sending logic               |
| src/bin/tui             | presentation.rs      |   280 |  8344 |     3 |   62% | TUI presentation & formatting he... |
| src/bin/tui             | scroll_handlers.rs   |   298 | 10313 |     5 |    -  | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   227 |  9290 |     7 |    -  | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    65 |  2294 |     3 |    -  | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   120 |  4010 |     5 |    -  | Render loop orchestration (60 FPS)  |
| src/bin/tui/render_loop | tab_renderers.rs     |   371 | 12155 |     6 |    -  | Tab-specific renderers              |
| src/bin/tui/render_loop | visibility.rs        |   209 |  5676 |     4 |   98% | Message visibility calculations     |
| src/generated           | columns.rs           |    42 |  1653 |     1 |    -  | Auto-generated column definitions   |
| src/generated           | mod.rs               |    11 |   488 |     0 |    -  | Module declarations                 |
| src/generated           | models_insertable.rs |   102 |  3689 |     1 |    -  | Insertable data models              |
| src/generated           | models_queryable.rs  |   120 |  4318 |     1 |    -  | Queryable data models               |
| src/generated           | schema.rs            |    71 |  1570 |     2 |    -  | Database schema (Diesel)            |

**Total:** 41 files, 8,049 lines, 270,618 characters

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
| tests  | messages.rs               |   366 | 11326 |     2 | messages module tests         |
| tests  | network.rs                |    46 |  1419 |     1 | network module tests          |
| tests  | nickname.rs               |   298 |  9827 |     3 | nickname module tests         |
| tests  | p2p_integration.rs        |  1014 | 36315 |    10 | P2P integration tests         |
| tests  | peers.rs                  |   231 |  6768 |     2 | peers module tests            |
| tests  | test_utils.rs             |    59 |  1855 |     3 | Test utilities                |
| tests  | tui_binary_integration.rs |   284 |  8853 |     3 | TUI binary integration tests  |
| tests  | tui_chat.rs               |   672 | 21332 |     4 | TUI chat functionality tests  |
| tests  | tui_events.rs             |   182 |  5613 |     4 | TUI events tests              |
| tests  | tui_helpers.rs            |   667 | 19923 |     3 | TUI helpers tests             |
| tests  | tui_integration.rs        |   443 | 14004 |     4 | TUI integration tests         |
| tests  | tui_render_integration.rs |   265 |  8257 |     5 | TUI render integration tests  |
| tests  | tui_state.rs              |   249 |  7557 |     2 | TUI state tests               |
| tests  | tui_tasks.rs              |   234 |  7567 |     7 | TUI task tests                |
| tests  | types.rs                  |   104 |  3099 |     2 | types module tests            |

**Total:** 23 test files, 6,162 lines, 194,533 characters
