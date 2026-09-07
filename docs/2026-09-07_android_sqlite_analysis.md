# Android SQLite: why it's embedded in the Rust part, and paths to Android-provided SQLite

## Question
Why does the Android build embed its own SQLite (compiled into the Rust cdylib) instead of
using a SQLite provided by Android, and how would we switch to Android-provided SQLite?

## Current setup (the "embedded" SQLite)

- `Cargo.toml:54` — `sqlite_bundled = ["dep:libp2p", "dep:libsqlite3-sys"]`; the direct dep at
  `Cargo.toml:72` is `libsqlite3-sys = { version = "0.36.0", features = ["bundled"], optional = true }`.
- `Cargo.toml:58` — `mobile = ["basic", "sqlite_bundled", "quic", "flutter_rust_bridge"]`, so every
  `--features mobile` build activates `libsqlite3-sys/bundled` **globally for the crate graph**.
- `apps/flutter_app/build_rust_android.sh` — the mobile build is plain `cargo build --lib --features mobile`
  per NDK target (`aarch64/armv7/x86_64-linux-android`), then the cdylib lands in `jniLibs`.
- `libsqlite3-sys 0.36.0` semantics (from its `build.rs`/`Cargo.toml.orig`):
  - **Without** `bundled` (default, `min_sqlite_version_3_34_1`): locates a **system** SQLite via
    `pkg-config`/`vcpkg` and links it. This is what the desktop build does (default features ≠ mobile).
  - **With** `bundled`: compiles the shipped amalgamation `sqlite3.c` directly into the linking crate.
- The bundled amalgamation in 0.36.0 is **SQLite 3.51.1** (verified in `sqlite3/sqlite3.h`), compiled per-ABI
  by the `cc` crate using the NDK clang that `build_rust_android.sh` exports.
- DB access on Android flows: Flutter calls FRB → `mobile_node::start_node(db_path)` (`src/mobile_node.rs:41-80`)
  → `init_mobile_database(path)` → the diesel code in `src/db.rs` running embedded migrations against the file.

## Why Android needs an embedded/bundled SQLite

1. **Android exposes no linkable native SQLite to apps.** The platform keeps a private
   `libsqlite.so` in `libandroid_runtime`/the system partition; it is **not** in the NDK
   (no headers, no `.so` in the sysroot) and is not part of the NDK ABI. Since Nougat's
   linker-namespace isolation, an app cannot even `dlopen` it (`dlopen failed: library
   "libsqlite.so" not found`). So a crate that talks SQLite over C FFI — diesel → `libsqlite3-sys`
   — literally has nothing to link against on Android unless it supplies the engine itself.
2. **The only Android-provided SQLite is the Java framework class**
   `android.database.sqlite.SQLiteDatabase` (and the `sqlite3` program on the shell user).
   That is a JNI wrapper around the private native library, reachable only from Kotlin/Java.
3. **Version pinning for diesel.** The SQLite backend uses `returning_clauses_for_sqlite_3_35`
   (`Cargo.toml:68`) — `INSERT … RETURNING`, which needs SQLite ≥ 3.35. Android's platform SQLite
   version varies per device/ROM (≈3.32 on Android 12, 3.39 on 13, 3.44 on 14, 3.46+ on 15); old
   devices would break diesel's generated queries. The bundle pins a known-new engine (3.51.1) on
   every device, which also keeps migration behavior deterministic.
4. **Schema/engine consistency.** Diesel runs embedded migrations against the file; a heterogeneous
   set of platform engines would produce per-device behavior differences on top of the same schema.

That's also why the earlier `sqlite-bundled` restructure was rejected in backlog item 7 and the
`sqlite_bundled` feature already scopes the bundling to mobile correctly (it stays off for the host
build, which keeps using the system SQLite via pkg-config).

## Paths to "use Android-provided SQLite"

### Path A — Framework `SQLiteDatabase` becomes the storage backend (big rewrite)
Move persistence out of diesel entirely: FRB calls Kotlin, Kotlin owns a `SQLiteDatabase` (or Room),
Rust stores its parts (peers, chat history, broadcast receipts) via those calls.

- **Would use Android-provided SQLite for real** (the framework's engine, JNI-exposed).
- **Cost:** discards the diesel SQLite backend, `sqlite_connect`, embedded migrations, schema/
  columns generation (start over with Room/SQLDelight or hand-written Kotlin DAOs); the whole
  `src/peers.rs`, `src/messages.rs`, `src/nickname.rs`, `src/db.rs` surface moves. Version still
  varies per device unless the Kotlin layer guards on `SQLiteDatabase.getVersion` (Android 14+).
- Verdict: only worth it if the project is willing to abandon shared Rust persistence; otherwise too invasive.

### Path B — Link system `libsqlite.so` from native code (won't work)
Use `libsqlite3-sys` without `bundled` and let it `dlopen`/link `libsqlite.so`.

- Fails on all counts: not in the NDK, not in the app linker namespace, and its symbol set isn't a
  stable public ABI. Non-starter on modern Android.

### Path C — Ship a prebuilt `libsqlite3.so` in `jniLibs` and link against it (de-embed, keep diesel)
Compile sqlite once per ABI (build step with NDK clang, e.g. via a Makefile or a tiny `cc` build),
put `libsqlite3.so` alongside `libp2p_app.so` in `jniLibs`, and build the Rust lib **without** the
`bundled` feature against a matching header (`sqlite3.h`) — link with `#[link]`/`-L` to the prebuilt
in `build.rs` or `build_rust_android.sh` `RUSTFLAGS`.

- Removes the C-from-Cargo compile (`cc` on a 1.3 MB amalgamation per ABI) from the Rust build and
  shrinks the cdylib a bit, but the SQLite engine is still **self-supplied** (nothing Android-provided).
- Must keep the prebuilt version ≥ 3.35 for diesel's `RETURNING`; adds a second `.so` to package/version.
- Verdict: the pragmatic "de-embed," but it does not satisfy "use Android-provided SQLite" — there is
  no Android-provided native library to reuse.

### Path D — Symmetric-app storage via TWO backends
Keep diesel+embedded for the shared Rust core and mirror to framework `SQLiteDatabase` for anything
the UI reads. Adds a read-path + sync layer for zero correctness gain. Not recommended.

## Deeper: "Android-provided SQLite ⇒ Kotlin does the persistence" — generation feasibility and what it does to Rust

### Confirming the shift
Yes. The only Android-provided SQLite is the `android.database.sqlite.SQLiteDatabase` framework
class (with Room as its typed wrapper). Both are Kotlin/Java-side. So Path A is precisely: **persistence
moves out of the Rust crate and into Kotlin** — Rust stops calling `sqlite_connect()` on Android and
instead makes requests that end in Kotlin `SQLiteDatabase` calls. Rust keeps only what's pure
computation on already-fetched rows (display-name memoization, peer-table sort, message stats),
because those functions don't touch the DB — only their inputs now arrive from the Kotlin backend.

### Can the Kotlin layer be generated from the Rust/diesel code instead of hand-duplicated?

**What's already single-source and reusable as-is:**
- **The DDL.** `migrations/*/up.sql` is the canonical schema (diesel embeds it at compile time).
  `SQLiteOpenHelper.onCreate` can run that same SQL verbatim (`execSQL` per statement), or use
  Room against a migrated database. Zero schema duplication.
- **Column names/types.** `build.rs` already parses `src/generated/schema.rs` and emits
  `src/generated/columns.rs` at build time. The same parser is the raw material for generated
  Kotlin entity classes.

**Realistically generatable at build time:**
- **Entities + DAO skeletons.** Extend the existing `build.rs` schema parser to also stamp out Kotlin
  `data class` row types (and/or Room `@Entity`).
  `@Dao` interface skeletons per table (INSERT-by-pk / SELECT-by-pk / DELETE-by-pk) are mechanical
  from the schema. If we use Room, its annotation processor then **generates the interface
  implementations itself** — so the "generate DAO code" ask is largely Room's own job once the
  `@Entity`/`@Dao` stubs exist. Generation would be an extra emit into the Android module
  (`apps/flutter_app/android/app/src/main/kotlin/...`) before the Gradle compile, wired into
  `build_rust_android.sh`.
- **The FRB surface as a checklist.** `src/mobile_api.rs` enumerates exactly which persistence RPCs
  the UI needs; a generator could emit Kotlin method signatures/stubs from it, but the **bodies**
  still need real query logic.
- **Dart↔Kotlin channel.** Pigeon can generate the MethodChannel bridge (Dart + Kotlin) from a small
  interface file — an orthogonal, well-trodden generator.

**What cannot be generated:**
- **The diesel query bodies.** Queries are written as the type-checked diesel DSL *inside function
  bodies* (`src/peers.rs`, `src/messages.rs`, `src/nickname.rs`, `src/db.rs`). There is no runtime IR
  of a query to transpile — diesel produces the SQL at codegen time, but the *intent* (e.g. the
  two-pass `GROUP BY`/`UNION ALL` in `get_all_peer_stats`, or `record_broadcast_recipients`'s
  idempotent upsert) lives only in hand-written Rust code. Nearly all of the ~40 `sqlite_connect()`
  call sites must be **hand-ported to Kotlin**. The schema is not the duplication problem; the
  query logic is.

### "Android mode" in the Rust crate — the cost you flagged
- **Mode detection: runtime backend enum, not `cfg` fork.** Add a `PersistenceBackend { Embedded, Kotlin }`
  latched in `start_node_impl` (which already branches on whether a `db_path` was passed). A
  `cfg(target_os = "android")` fork would duplicate whole modules and wouldn't be unit-testable on
  the desktop host; dispatch instead of fork keeps one artifact and lets each backend be tested.
- **Every DB function becomes a dispatch.** Each `sqlite_connect()`-rooted persistence function grows
  a `match` on the backend; the Kotlin arm turns the call into a request over the Dart/FRB bridge.
  Reads that Rust needs for its pure logic (peer rows, message history, nickname data) come back to
  Rust across that bridge.
- **Sync↔async bridge.** Diesel call sites are synchronous, but a Rust→Dart call is inherently async.
  FRB's synchronous callback variant lets Rust block on a Dart-side call, which then proxies to
  Kotlin over MethodChannel and returns — so existing call signatures can stay synchronous. Rows
  travel as JSON (or generated serializers), adding per-call marshalling cost on top of the
  double-FFI hop (Rust→Dart→Kotlin and back).
- **Divergence (behavior-drift) risk.** The desktop/TUI path keeps diesel; only Android runs the
  Kotlin backend, so the two paths can drift (sort, stats, nickname logic duplicated on both sides
  of the bridge) and need an `androidTest` suite to cover what desktop unit tests can't.
- **Diesel stays compiled on Android** unless the SQLite backend is excluded per-target — a
  dependency restructure on its own. Pragmatically it stays; only the runtime dispatch changes.

### Verdict on this angle
Partial generation is genuinely available: reuse `migrations/*/up.sql` for DDL and extend `build.rs`
to emit Kotlin entities/`@Dao` skeletons (Room finishes the implementations). But the diesel query
logic cannot be generated away — it must be mirrored by hand in Kotlin — and the Rust side gains a
backend-dispatch + synchronous FRB bridge on every persistence call. That hand-ported logic + the
dispatch plumbing, not the embedding itself, is the real price of using Android's SQLite.

## Conclusion
Android provides no linkable native SQLite — the embedding is not an arbitrary choice but a hard
platform constraint (the framework class is the only Android-provided SQLite). Therefore:

- "Use Android-provided SQLite" realistically means **Path A** (abandon diesel for Kotlin storage).
  Schema/entity code can be partially generated from the Rust sources (see the section above), but the
  diesel query logic must be hand-ported to Kotlin and the Rust side gains a backend-dispatch +
  synchronous FRB bridge — a large rewrite that drops the single shared persistence layer; or
- accept the current embedded bundle (recommended), or
- **Path C** if the goal is only to move the SQLite compile out of Cargo into a packaged prebuilt `.so`
  while keeping diesel — this de-embeds but still self-supplies the engine.

Considered and rejected for now: Path A (too invasive — Kotlin persistence mirror + Rust backend
dispatch, see previous section) and Path B (impossible on modern Android). Path C is the fallback if
the per-ABI amalgamation compile ever becomes a build-time pain.