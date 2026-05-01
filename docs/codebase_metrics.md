# Codebase Metrics

## Summary

| Metric                      | Count   |
|:--------------------------|--------:|
| Total Rust Files            |      37|
| Total Lines of Code         |   6,133|
| Total Characters            | 211,498|
| Average Lines per File      |     165|
| Average Characters per File |    5716|


## All Source Files

| Folder                  | File                 | Lines | Chars | Depth | Purpose                             |
|:------------------------|:---------------------|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   111 |  3864 |     5 | Build script                        |
| src                     | behavior.rs          |   119 |  3888 |     4 | Network behavior definitions        |
| src                     | db.rs                |   377 | 13271 |     5 | Database connection & identity mgmt |
| src                     | fmt.rs               |   102 |  3167 |     4 | Formatting & display utilities      |
| src                     | lib.rs               |   327 | 10233 |     3 | Module declarations & re-exports    |
| src                     | logging.rs           |   234 |  6837 |     4 | Logging utilities & setup           |
| src                     | logging_config.rs    |    38 |  1772 |     2 | Tracing configuration               |
| src                     | messages.rs          |   192 |  6764 |     4 | Message persistence & retrieval     |
| src                     | network.rs           |    37 |  1143 |     3 | Network size classification         |
| src                     | nickname.rs          |   134 |  4855 |     3 | Nickname management                 |
| src                     | peers.rs             |   176 |  5963 |     3 | Peer management & tracking          |
| src                     | swarm_handler.rs     |   270 | 10112 |     8 | Network event translation           |
| src                     | tui_events.rs        |    51 |  1377 |     1 | Event/command types & channels      |
| src                     | tui_tabs.rs          |   187 |  4880 |     5 | Tab management & navigation         |
| src                     | tui_test_state.rs    |   152 |  4506 |     6 | TUI test state & mouse handling     |
| src                     | types.rs             |    62 |  1685 |     2 | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   163 |  5888 |     7 | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   208 |  7137 |     8 | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   136 |  5127 |     4 | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   428 | 16198 |     7 | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   282 | 10845 |     6 | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    19 |   609 |     0 | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    44 |  1631 |     6 | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   306 | 12138 |    10 | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   267 |  9847 |     5 | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |   107 |  3919 |     5 | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   378 | 12888 |     5 | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   227 |  9290 |     7 | Shared application state            |
| src/bin/tui/render_loop | layout.rs            |    65 |  2294 |     3 | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   139 |  4879 |     5 | Render loop orchestration (60 FPS)  |
| src/bin/tui/render_loop | tab_renderers.rs     |   441 | 14736 |     7 | Tab-specific renderers              |
| src/bin/tui/render_loop | visibility.rs        |   117 |  3390 |     5 | Message visibility calculations     |
| src/generated           | columns.rs           |    37 |  1539 |     1 | Auto-generated column definitions   |
| src/generated           | mod.rs               |     4 |    86 |     0 | Module declarations                 |
| src/generated           | models_insertable.rs |    59 |  1458 |     1 | Insertable data models              |
| src/generated           | models_queryable.rs  |    68 |  1737 |     1 | Queryable data models               |
| src/generated           | schema.rs            |    69 |  1545 |     2 | Database schema (Diesel)            |

**Total:** 37 files, 6,133 lines, 211,498 characters


### File Details

build.rs                                   111 lines     3864 chars  nesting= 5
src/behavior.rs                            119 lines     3888 chars  nesting= 4
src/db.rs                                  377 lines    13271 chars  nesting= 5
src/fmt.rs                                 102 lines     3167 chars  nesting= 4
src/lib.rs                                 327 lines    10233 chars  nesting= 3
src/logging.rs                             234 lines     6837 chars  nesting= 4
src/logging_config.rs                       38 lines     1772 chars  nesting= 2
src/messages.rs                            192 lines     6764 chars  nesting= 4
src/network.rs                              37 lines     1143 chars  nesting= 3
src/nickname.rs                            134 lines     4855 chars  nesting= 3
src/peers.rs                               176 lines     5963 chars  nesting= 3
src/swarm_handler.rs                       270 lines    10112 chars  nesting= 8
src/tui_events.rs                           51 lines     1377 chars  nesting= 1
src/tui_tabs.rs                            187 lines     4880 chars  nesting= 5
src/tui_test_state.rs                      152 lines     4506 chars  nesting= 6
src/types.rs                                62 lines     1685 chars  nesting= 2
src/bin/p2p_chat.rs                        163 lines     5888 chars  nesting= 7
src/bin/p2p_chat_dioxus.rs                 208 lines     7137 chars  nesting= 8
src/bin/p2p_chat_tui.rs                    136 lines     5127 chars  nesting= 4
src/bin/tui/click_handlers.rs              428 lines    16198 chars  nesting= 7
src/bin/tui/command_processor.rs           282 lines    10845 chars  nesting= 6
src/bin/tui/constants.rs                    19 lines      609 chars  nesting= 0
src/bin/tui/event_source.rs                 44 lines     1631 chars  nesting= 6
src/bin/tui/input_processor.rs             306 lines    12138 chars  nesting=10
src/bin/tui/main_loop.rs                   267 lines     9847 chars  nesting= 5
src/bin/tui/message_handlers.rs            107 lines     3919 chars  nesting= 5
src/bin/tui/scroll_handlers.rs             378 lines    12888 chars  nesting= 5
src/bin/tui/state.rs                       227 lines     9290 chars  nesting= 7
src/bin/tui/render_loop/layout.rs           65 lines     2294 chars  nesting= 3
src/bin/tui/render_loop/mod.rs             139 lines     4879 chars  nesting= 5
src/bin/tui/render_loop/tab_renderers.rs   441 lines    14736 chars  nesting= 7
src/bin/tui/render_loop/visibility.rs      117 lines     3390 chars  nesting= 5
src/generated/columns.rs                    37 lines     1539 chars  nesting= 1
src/generated/mod.rs                         4 lines       86 chars  nesting= 0
src/generated/models_insertable.rs          59 lines     1458 chars  nesting= 1
src/generated/models_queryable.rs           68 lines     1737 chars  nesting= 1
src/generated/schema.rs                     69 lines     1545 chars  nesting= 2
