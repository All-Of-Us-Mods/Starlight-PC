# Plan 003: Write the settings file atomically (temp + rename)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/backend/services/core_service.rs`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: 001 (test gate — so the new test actually runs in CI)
- **Category**: bug
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

App settings (game path, platform, BepInEx URLs, launch options) are saved
with a plain `fs::write`. If the process dies or the machine loses power
mid-write, the JSON is truncated. On the next start, `get_settings` catches
the parse error and **silently falls back to default settings** — the user
loses their configured game path and platform with no error shown. Profile
metadata already uses the safe temp-file-then-rename pattern; settings should
match it.

## Current state

- `src/backend/services/core_service.rs` — settings persistence. The unsafe
  write at lines 109–115:

  ```rust
  fn write_settings_to_file(path: &Path, settings: &AppSettings) -> AppResult<()> {
      if let Some(parent) = path.parent() {
          fs::create_dir_all(parent)?;
      }
      fs::write(path, serde_json::to_vec_pretty(settings)?)?;
      Ok(())
  }
  ```

  The silent-fallback read path at lines 208–229 (`get_settings`) logs a
  `warn!` on parse failure and returns migration/default settings — that
  behavior stays as-is; this plan removes the main way it gets triggered.

- The repo's exemplar for atomic writes,
  `src/backend/services/profile_service.rs:120-129`:

  ```rust
  fn write_profile(profile: &ProfileEntry) -> AppResult<()> {
      let profile_dir = PathBuf::from(&profile.path);
      fs::create_dir_all(&profile_dir)?;
      let metadata = serde_json::to_vec_pretty(profile)?;
      let metadata_path = metadata_path(&profile_dir);
      let temporary_path = metadata_path.with_extension("json.tmp");
      fs::write(&temporary_path, metadata)?;
      fs::rename(&temporary_path, &metadata_path)?;
      Ok(())
  }
  ```

  Match this pattern exactly (same `.with_extension` style, same ordering).

- Error handling convention: functions return `AppResult<T>` and propagate
  with `?` (`AppError` in `src/backend/error.rs`). No `.unwrap()` on I/O.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Check | `cargo check --all-targets` | exit 0 |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test` | exit 0, all pass (3 existing + 1 new) |

Note for this machine: `cargo` is not on PATH; in PowerShell run
`$env:Path += ";$env:USERPROFILE\.cargo\bin"` first.

## Scope

**In scope** (the only files you should modify):
- `src/backend/services/core_service.rs`

**Out of scope** (do NOT touch):
- `src/backend/services/profile_service.rs` — already atomic; it's the
  exemplar, not a target.
- `get_settings`'s parse-failure fallback behavior — intentional resilience;
  keep it.
- The settings schema (`AppSettings`) or `update_settings` field list.
- Adding a settings mutex / fixing the read-modify-write race — considered
  and deliberately deferred (single-process app, updates come from UI
  actions; the crash-truncation risk this plan fixes is the real one).

## Git workflow

- Branch: `advisor/003-atomic-settings-write`
- Commit style: short lowercase imperative subject (e.g. "write settings atomically").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Make `write_settings_to_file` atomic

Rewrite the function body to write to a sibling temp file, then rename over
the target — mirroring `write_profile`:

```rust
fn write_settings_to_file(path: &Path, settings: &AppSettings) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary_path = path.with_extension("json.tmp");
    fs::write(&temporary_path, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&temporary_path, path)?;
    Ok(())
}
```

(The settings file name ends in `.json`, so `.with_extension("json.tmp")`
produces a sibling `.json.tmp`, same as the profile pattern.)

**Verify**: `cargo check --all-targets` → exit 0.

### Step 2: Add a round-trip unit test

Add a `#[cfg(test)] mod tests` at the bottom of `core_service.rs` (the repo
convention — see the test module at the bottom of
`src/backend/state/game_runtime.rs:434+` for structure). Test that
`write_settings_to_file` + a `serde_json::from_str::<AppSettings>` read
round-trips, and that no `.json.tmp` file remains:

- Build the target path inside `std::env::temp_dir()` with a unique file name
  (e.g. suffix `std::process::id()`), write `AppSettings::default()`, read the
  file back, assert it parses and the temp sibling does not exist. Clean up
  the file at the end of the test.

**Verify**: `cargo test` → exit 0, output includes the new test passing and
`0 failed`.

### Step 3: Lint

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.

## Test plan

- New test in `core_service.rs::tests`: round-trip write/read of default
  settings via `write_settings_to_file`, asserting (a) parseable output,
  (b) no leftover `.json.tmp`. Model the module layout on
  `game_runtime.rs`'s existing tests.
- Verification: `cargo test` → all pass.

## Done criteria

- [ ] `write_settings_to_file` writes via temp file + `fs::rename`
- [ ] `cargo test` exits 0 and includes the new round-trip test
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `grep -n "fs::write(path" src/backend/services/core_service.rs` returns no match
- [ ] `git status` shows only `core_service.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `write_settings_to_file` at HEAD no longer matches the excerpt (drift).
- The rename fails on Windows in your test — would suggest the settings dir
  and temp path landed on different volumes; report rather than switching to
  a copy-based fallback.
- You find other callers writing the settings file directly (there should be
  exactly two callers of `write_settings_to_file`: `get_settings`'s legacy
  migration at ~line 224 and `update_settings` at ~line 281).

## Maintenance notes

- Any future settings-file writer must go through `write_settings_to_file`;
  a reviewer should flag any new direct `fs::write` of `settings.json`.
- If a background thread ever starts calling `update_settings` concurrently
  with the UI, revisit the deferred read-modify-write race (a
  `Mutex<()>` around the read+write would suffice).
