# Clippy Decision Framework

Guidelines for when to apply clippy suggestions vs use `#[allow]`.

## The Core Principle

If the "fix" requires architectural changes that don't improve the actual code (just satisfy a lint), add `#[allow]`. If the fix makes the code objectively better, do it.

## When to use `#[allow]` (Exceptions)

### 1. False positives or wrong suggestions
Clippy doesn't understand your intent or context.

Example: `AppState::new` with 12 params - that's internal TUI state, not something you'd refactor to a builder pattern that would never be used externally.

### 2. Idiomatic Rust that clippy disputes
Sometimes clippy's suggestion makes code *less* readable.

A builder with chainable methods is nice, but if you're only calling it from one place with all args, it's overkill.

### 3. Performance-critical tight loops
If the "fix" adds indirection or allocation that matters in a hot path.

### 4. Public API stability
If changing a function signature breaks downstream code you can't control.

### 5. The user explicitly says to skip
Sometimes the project owner prefers the current style.

## When to Actually Fix

### 1. Real bugs - Always fix
e.g., `unwrap()` that could panic on real input

### 2. Memory/safety issues - Always fix
e.g., buffer overflow, deadlocks, race conditions

### 3. Obvious improvements - Usually fix
e.g., using `.clone()` when you meant `.copy()`, unused variables, unnecessary casts

### 4. Style consistency - Fix if it's a project standard
e.g., `unwrap()` vs `?`, naming conventions, derive usage

## Examples in this project

### Fixed (made code better)
- `DirectMessage` / `BroadcastMessage`: use `#[derive(Default)]` instead of manual impl
- `save_message_with_meta`: use `MessageMeta` struct to reduce params 8→6

### Allowed (architectural overhead for no benefit)
- `AppState::new`: 12 params - adding a builder would be unused code

## Decision Process

1. Does the fix improve correctness, safety, or performance? → **Fix it**
2. Does it make code more readable/maintainable? → **Fix it**
3. Does it require architectural changes for a marginal style gain? → **Add `#[allow]`**
4. Is it a genuine improvement in code structure? → **Fix it**