# Codebase Metrics

## Summary

| Metric                  | Value   |
|:------------------------|--------:|
| Total Rust Files        |      41 |
| Total Lines of Code     |   7,587 |
| Total Characters        | 256,264 |
| Average Lines per File  |     185 |
| Average Characters/File |   6,250 |

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
| src                     | swarm_handler.rs     |   378 | 12846 |     7 |   30% | Network event translation           |
| src                     | tui_events.rs        |    76 |  2586 |     1 |  100% | Event/command types & channels      |
| src                     | tui_helpers.rs       |   293 |  8662 |     3 |  100% | Source file                         |
| src                     | tui_render.rs        |   195 |  6221 |     4 |    -  | Source file                         |
| src                     | tui_render_state.rs  |   159 |  4608 |     4 |    -  | Source file                         |
| src                     | tui_tabs.rs          |   198 |  5277 |     5 |    -  | Tab management & navigation         |
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
| src/bin/tui             | presentation.rs      |   220 |  6472 |     3 |   52% | Source file                         |
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

**Total:** 41 files, 7,587 lines, 256,264 characters
