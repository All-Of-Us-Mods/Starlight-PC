# Plan 008: Fetch mod catalog names concurrently in the profile detail view

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/views/library_detail/mod.rs`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: perf
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

The profile detail page resolves display names for a profile's mods by
awaiting one catalog fetch at a time in a loop — a profile with 10
uncached mods pays 10 sequential network round-trips before the last name
appears. The lobbies view solves the identical problem by spawning all
fetches up front so they run concurrently (`src/views/lobbies.rs:301-330`,
via the same shared `mod_catalog_cache`). Copy that structure; keep this
view's nicer property of updating each name as it arrives.

## Current state

- `src/views/library_detail/mod.rs:215-231` — the sequential loop inside
  `ensure_mod_info` (method starts ~line 190):

  ```rust
  cx.spawn(async move |this, cx| {
      for mod_id in pending {
          let id_for_fetch = mod_id.clone();
          let resolved = cx
              .background_executor()
              .spawn(async move { mod_catalog_cache::fetch(&id_for_fetch).map(|m| m.name) })
              .await;                                   // ← serializes the fetches
          if let Some(name) = resolved {
              let _ = this.update(cx, |this, cx| {
                  this.mod_names.insert(mod_id, name);
                  cx.notify();
              });
          }
      }
  })
  .detach();
  ```

  `pending` is a `Vec<String>` of mod ids not in the cache (built at lines
  197-202). `this.mod_names` is a `HashMap<String, String>` on the view.

- The concurrent exemplar, `src/views/lobbies.rs:310-321`:

  ```rust
  cx.spawn(async move |this, cx| {
      let tasks: Vec<_> = missing
          .iter()
          .cloned()
          .map(|id| {
              cx.background_executor()
                  .spawn(async move { mod_catalog_cache::fetch(&id) })
          })
          .collect();
      for task in tasks {
          task.await;
      }
      ...
  ```

  gpui `background_executor().spawn(...)` starts the future immediately; the
  later `.await`s only collect results. Spawning all before awaiting any is
  what buys the concurrency.

- `mod_catalog_cache::fetch` is shared and de-duplicates in-flight lookups
  across views (see the doc comment at lobbies.rs:296-300) — no extra
  guarding needed here.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Check | `cargo check --all-targets` | exit 0 |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test` | exit 0 |

Note for this machine: `cargo` is not on PATH; in PowerShell run
`$env:Path += ";$env:USERPROFILE\.cargo\bin"` first.

## Scope

**In scope** (the only files you should modify):
- `src/views/library_detail/mod.rs` (only the `ensure_mod_info` method)

**Out of scope** (do NOT touch):
- `src/views/lobbies.rs` — the exemplar, not a target.
- `src/backend/state/mod_catalog_cache.rs` — the cache is correct as-is.
- The cached-names fast path at lines 196-211 of library_detail/mod.rs.

## Git workflow

- Branch: `advisor/008-concurrent-mod-name-fetches`
- Commit style: short lowercase imperative subject (e.g. "fetch mod names concurrently").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Spawn all fetches before awaiting any

Rewrite the `cx.spawn(async move |this, cx| { … })` body at lines 215-230 to:

1. Map `pending` into a `Vec` of `(mod_id, task)` pairs, where each task is
   `cx.background_executor().spawn(async move { mod_catalog_cache::fetch(&id_for_fetch).map(|m| m.name) })`
   — build ALL pairs before the first `.await` (this is the entire fix).
2. Then loop over the pairs: `if let Some(name) = task.await { … }` with the
   same `this.update` body as today (insert into `this.mod_names`, `cx.notify()`),
   so names still appear progressively as fetches complete.

**Verify**: `cargo check --all-targets` → exit 0.

### Step 2: Lint and test

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.
**Verify**: `cargo test` → exit 0.

## Test plan

No new unit tests — this is view-layer async plumbing with no pure logic.
The structural property (all spawns precede the first await) is verified by
reading the diff. Manual check: open a profile with several mods on a cold
start; names should populate near-simultaneously instead of one by one.

## Done criteria

- [ ] In `ensure_mod_info`, every `background_executor().spawn` call happens
      before the first `.await` on any of those tasks
- [ ] Per-name progressive UI updates retained (an `this.update` per resolved
      name, not one batch at the end)
- [ ] `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test` all exit 0
- [ ] `git status` shows only `src/views/library_detail/mod.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `ensure_mod_info` at HEAD doesn't match the excerpt (drift).
- The borrow checker forces you to restructure beyond this method (e.g.
  cloning view state into the closure) — the lobbies exemplar compiles under
  the same constraints, so a fight with the borrow checker signals a wrong
  turn, not a needed workaround.

## Maintenance notes

- If the profile page ever needs thumbnails too (like lobbies), extend the
  task payload rather than adding a second loop.
- Reviewer check: no unbounded parallelism concern — profiles have at most
  tens of mods and `mod_catalog_cache` already dedups in-flight fetches.
