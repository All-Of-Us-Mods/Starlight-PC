# Plan 006: Stop destroying the stored session before the new one is written

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/backend/services/storage_service.rs`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: MED (touches credential storage; failure mode is being logged out)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

`KeyringStorage::save` persists the Epic session (access/refresh tokens) to
the OS keyring as N base64 chunks plus a count entry. Today it calls
`self.clear()?` **first**, then writes the count, then each chunk. If any
write after `clear()` fails (keyring service hiccup, locked keychain), the
old session is already destroyed and the new one is partial — `load()` will
return `None` and the user is silently logged out of Epic. Reordering so the
count entry is written **last** makes the count the commit point: a failure
mid-save leaves the previous count in place, and `load()` keeps working
against a state that is at worst refreshed-chunks-with-old-count (which fails
decode gracefully to `None` — same as today's failure mode, never worse, and
the common case is fully preserved).

## Current state

- `src/backend/services/storage_service.rs:31-53` — the current save:

  ```rust
  pub fn save(&self, data: &T, chunk_size: usize) -> AppResult<()> {
      info!("Saving {} to keyring", self.base_key);
      self.clear()?;

      let json = serde_json::to_vec(data)?;
      let encoded = B64.encode(compress(&json)?);
      let chunks: Vec<_> = encoded.as_bytes().chunks(chunk_size).collect();

      self.entry("n")?
          .set_password(&chunks.len().to_string())
          .map_err(AppError::from)?;

      for (i, chunk) in chunks.iter().enumerate() {
          let chunk_str = std::str::from_utf8(chunk)
              .map_err(|e| AppError::parse(format!("Invalid UTF-8 chunk: {e}")))?;
          self.entry(&i.to_string())?
              .set_password(chunk_str)
              .map_err(AppError::from)?;
      }

      info!("{} saved ({} chunks)", self.base_key, chunks.len());
      Ok(())
  }
  ```

- `clear()` (lines 68-98) reads the count entry, deletes it, then deletes
  each chunk `0..count`. It is also called directly for logout
  (`epic_auth_service.rs` logout path) — that use stays.

- `load()` (lines 55-66) reads the count, then chunks `0..count`, and returns
  `Option<T>` — any missing/corrupt piece yields `None`. This graceful
  degradation is what makes the reorder safe.

- Keyring entries are keyed `"{base_key}_{suffix}"` with suffixes `"n"` and
  `"0"`, `"1"`, … (`entry()` at lines 27-29). `set_password` overwrites an
  existing entry in place (keyring crate semantics) — no delete needed before
  rewriting a chunk.

- Sole consumer: `KeyringStorage<EpicSession>` in
  `src/backend/services/epic_auth_service.rs:22` (`static STORAGE`).

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
- `src/backend/services/storage_service.rs`

**Out of scope** (do NOT touch):
- `src/backend/services/epic_auth_service.rs` — the `save`/`load`/`clear`
  API surface does not change.
- The chunking/compression/encoding scheme.
- Adding OS-keyring-backed unit tests — tests would write to the real user
  keyring; do not do this.

## Git workflow

- Branch: `advisor/006-keyring-save-preserve-session`
- Commit style: short lowercase imperative subject (e.g. "make keyring save non-destructive").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Reorder `save` so the count is the commit point

Rewrite `save` to:

1. Read the **old** count first (same tolerant logic `clear()` uses at lines
   69-78: missing/unparsable → 0). Extract that snippet into a private
   `fn stored_count(&self) -> usize` and have both `save` and `clear` call it.
2. Do NOT call `clear()`.
3. Encode and chunk exactly as today.
4. Write all data chunks `0..new_count` (the existing loop, unchanged).
5. Write the `"n"` count entry **after** all chunks succeeded.
6. Best-effort delete stale chunks `new_count..old_count` when the new chunk
   count is smaller (log failures with `error!`, don't propagate — matching
   `clear()`'s current style of logging deletion failures).

The doc comment on `save` should state the invariant in one line: the count
entry is written last so a failed save never destroys the previous session.

**Verify**: `cargo check --all-targets` → exit 0.

### Step 2: Lint and test

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.
**Verify**: `cargo test` → exit 0 (no behavior change expected in existing tests).

## Test plan

No new automated tests — `KeyringStorage` talks to the real OS keyring, and
this repo (correctly) has no keyring mock. The correctness argument is
ordering, verified by reading the diff: chunks → count → stale-chunk cleanup,
and no `clear()` call inside `save`. Manual check if you want one: log into
Epic in the app, restart, confirm still logged in (exercises save + load).

## Done criteria

- [ ] `save` no longer calls `clear()`
- [ ] The `"n"` count entry is written after all chunk entries
- [ ] Stale chunks beyond the new count are best-effort deleted after commit
- [ ] `clear()` still exists unchanged in behavior (logout path)
- [ ] `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test` all exit 0
- [ ] `git status` shows only `storage_service.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `storage_service.rs` at HEAD doesn't match the excerpts (drift).
- You find that `set_password` on this keyring backend does NOT overwrite an
  existing entry (e.g. it errors on duplicates) — the whole reorder rests on
  overwrite semantics; report the observed behavior.
- You find a second consumer of `KeyringStorage` besides the Epic session —
  re-check the plan's assumptions against how that consumer calls `save`.

## Maintenance notes

- The residual (accepted) failure window: a crash after some chunks are
  overwritten but before the count is written can leave mixed old/new chunks
  under the old count → `load()` returns `None` (logged out). That is the
  same outcome as today's *entire* window, shrunk to a much smaller one.
  True atomicity would require a versioned key namespace (write to
  `"{base_key}_v{N+1}_*"`, flip a pointer entry, delete old) — deferred as
  not worth the complexity for one session blob.
- Reviewer should scrutinize step ordering in the diff, and that
  `stored_count` is shared by `save` and `clear` rather than duplicated.
