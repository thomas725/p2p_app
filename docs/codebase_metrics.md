Scanning Rust source files...

================================================================================
CODEBASE METRICS
================================================================================

## Summary

| Metric                      | Count   |
|:--------------------------|--------:|
| Total Rust Files            |      38|
| Total Lines of Code         |   4,577|
| Total Characters            | 158,067|
| Average Lines per File      |     120|
| Average Characters per File |    4159|


## All Source Files

| Folder                    | File                 | Lines | Chars | Depth | Purpose                             |
|:-------------------------|---------------------:|------:|------:|------:|------------------------------------:|
| /                       | build.rs             |   107 |  3762 |     5 | Build script                        |
| src                     | behavior.rs          |   113 |  3703 |     4 | Network behavior definitions        |
| src                     | db.rs                |   331 | 11671 |     5 | Database connection & identity mgmt |
| src                     | fmt.rs               |    87 |  2664 |     4 | Formatting & display utilities      |
| src                     | lib.rs               |   207 |  6555 |     3 | Module declarations & re-exports    |
| src                     | logging.rs           |   234 |  6837 |     4 | Logging utilities & setup           |
| src                     | logging_config.rs    |    38 |  1772 |     2 | Tracing configuration               |
| src                     | messages.rs          |   123 |  4797 |     4 | Message persistence & retrieval     |
| src                     | network.rs           |    49 |  1423 |     3 | Network size classification         |
| src                     | nickname.rs          |   108 |  3741 |     3 | Nickname management                 |
| src                     | peers.rs             |   120 |  4348 |     3 | Peer management & tracking          |
| src                     | swarm_handler.rs     |   195 |  6774 |     6 | Network event translation           |
| src                     | tui_events.rs        |    51 |  1377 |     1 | Event/command types & channels      |
| src                     | tui_tabs.rs          |   187 |  4880 |     5 | Tab management & navigation         |
| src                     | tui_test_state.rs    |   152 |  4506 |     6 | TUI test state & mouse handling     |
| src                     | types.rs             |    42 |  1144 |     2 | Event & command type defs           |
| src/bin                 | p2p_chat.rs          |   161 |  5818 |     6 | CLI chat application                |
| src/bin                 | p2p_chat_dioxus.rs   |   208 |  7137 |     8 | Web UI (Dioxus framework)           |
| src/bin                 | p2p_chat_tui.rs      |   137 |  5136 |     4 | Main TUI application entry point    |
| src/bin/tui             | click_handlers.rs    |   186 |  7259 |     5 | Click handlers & index mapping      |
| src/bin/tui             | command_processor.rs |   125 |  5494 |     6 | Event routing & state updates       |
| src/bin/tui             | constants.rs         |    23 |   759 |     0 | TUI constants & config              |
| src/bin/tui             | event_source.rs      |    44 |  1631 |     6 | Terminal event polling (60 FPS)     |
| src/bin/tui             | input_processor.rs   |   190 |  7402 |     5 | Input event routing & processing    |
| src/bin/tui             | main_loop.rs         |   200 |  7055 |     4 | Task orchestration & async          |
| src/bin/tui             | message_handlers.rs  |    56 |  2161 |     4 | Message sending logic               |
| src/bin/tui             | scroll_handlers.rs   |   294 | 11375 |     6 | Scroll & hover-aware navigation     |
| src/bin/tui             | state.rs             |   142 |  5399 |     6 | Shared application state            |
| src/bin/tui             | tracing_writer.rs    |     3 |   246 |     0 | Tracing log output handling         |
| src/bin/tui/render_loop | layout.rs            |    57 |  2172 |     2 | UI layout component rendering       |
| src/bin/tui/render_loop | mod.rs               |   109 |  3830 |     5 | Render loop orchestration (60 FPS)  |
| src/bin/tui/render_loop | tab_renderers.rs     |   202 |  7116 |     4 | Tab-specific renderers              |
| src/bin/tui/render_loop | visibility.rs        |   117 |  3401 |     5 | Message visibility calculations     |
| src/generated           | columns.rs           |    27 |  1082 |     1 | Auto-generated column definitions   |
| src/generated           | mod.rs               |     4 |    86 |     0 | Module declarations                 |
| src/generated           | models_insertable.rs |    46 |  1108 |     1 | Insertable data models              |
| src/generated           | models_queryable.rs  |    54 |  1321 |     1 | Queryable data models               |
| src/generated           | schema.rs            |    48 |  1125 |     2 | Database schema (Diesel)            |

**Total:** 38 files, 4,577 lines, 158,067 characters


================================================================================
FILE DETAILS (for verification)
================================================================================

build.rs                                   107 lines     3762 chars  nesting= 5
src/behavior.rs                            113 lines     3703 chars  nesting= 4
src/db.rs                                  331 lines    11671 chars  nesting= 5
src/fmt.rs                                  87 lines     2664 chars  nesting= 4
src/lib.rs                                 207 lines     6555 chars  nesting= 3
src/logging.rs                             234 lines     6837 chars  nesting= 4
src/logging_config.rs                       38 lines     1772 chars  nesting= 2
src/messages.rs                            123 lines     4797 chars  nesting= 4
src/network.rs                              49 lines     1423 chars  nesting= 3
src/nickname.rs                            108 lines     3741 chars  nesting= 3
src/peers.rs                               120 lines     4348 chars  nesting= 3
src/swarm_handler.rs                       195 lines     6774 chars  nesting= 6
src/tui_events.rs                           51 lines     1377 chars  nesting= 1
src/tui_tabs.rs                            187 lines     4880 chars  nesting= 5
src/tui_test_state.rs                      152 lines     4506 chars  nesting= 6
src/types.rs                                42 lines     1144 chars  nesting= 2
src/bin/p2p_chat.rs                        161 lines     5818 chars  nesting= 6
src/bin/p2p_chat_dioxus.rs                 208 lines     7137 chars  nesting= 8
src/bin/p2p_chat_tui.rs                    137 lines     5136 chars  nesting= 4
src/bin/tui/click_handlers.rs              186 lines     7259 chars  nesting= 5
src/bin/tui/command_processor.rs           125 lines     5494 chars  nesting= 6
src/bin/tui/constants.rs                    23 lines      759 chars  nesting= 0
src/bin/tui/event_source.rs                 44 lines     1631 chars  nesting= 6
src/bin/tui/input_processor.rs             190 lines     7402 chars  nesting= 5
src/bin/tui/main_loop.rs                   200 lines     7055 chars  nesting= 4
src/bin/tui/message_handlers.rs             56 lines     2161 chars  nesting= 4
src/bin/tui/scroll_handlers.rs             294 lines    11375 chars  nesting= 6
src/bin/tui/state.rs                       142 lines     5399 chars  nesting= 6
src/bin/tui/tracing_writer.rs                3 lines      246 chars  nesting= 0
src/bin/tui/render_loop/layout.rs           57 lines     2172 chars  nesting= 2
src/bin/tui/render_loop/mod.rs             109 lines     3830 chars  nesting= 5
src/bin/tui/render_loop/tab_renderers.rs   202 lines     7116 chars  nesting= 4
src/bin/tui/render_loop/visibility.rs      117 lines     3401 chars  nesting= 5
src/generated/columns.rs                    27 lines     1082 chars  nesting= 1
src/generated/mod.rs                         4 lines       86 chars  nesting= 0
src/generated/models_insertable.rs          46 lines     1108 chars  nesting= 1
src/generated/models_queryable.rs           54 lines     1321 chars  nesting= 1
src/generated/schema.rs                     48 lines     1125 chars  nesting= 2
