# AGENTS.md - Developer Guidelines for p2p_app

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 0. Don't Revert

When the user reports a bug, **DO NOT** run `git revert`. Fix the bug properly instead.

- Reverting breaks the commit history and makes debugging harder
- If you're unsure how to fix, ask the user for clarification
- Only proceed with fixes when you understand the problem

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:

- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:

- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:

- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:

- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:

1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## 5. Database Schema Changes

When modifying the data model (adding columns to tables):

1. **Add columns to CREATE TABLE statements** in the base migration (e.g., `migrations/2026-04-04-225730_messages/up.sql`)
2. **DO NOT use ALTER TABLE** in new migrations to add columns to existing databases
3. **Rely on Rust's ensure_columns() logic** in `db.rs` to add columns to pre-existing databases automatically

This approach:
- Works for fresh installs (column in CREATE TABLE)
- Works for existing databases (ensure_columns adds missing columns from SCHEMA_ENTRIES)
- Avoids "duplicate column name" errors on re-run

Example: Adding `sender_nickname` to messages table:
- Add to CREATE TABLE: `sender_nickname TEXT`
- Add to SCHEMA_ENTRIES in build.rs: `("messages", "sender_nickname", "TEXT")`
- Use ensure_columns() logic (already implemented) for existing DBs
