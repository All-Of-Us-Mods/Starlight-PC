# Plan 004: Stream mod downloads to a temp file instead of buffering in RAM

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/backend/services/mod_download_service.rs`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (001 recommended first)
- **Category**: bug / perf
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

`download_mod` accumulates the entire remote file in a `Vec<u8>` before
writing it to disk: a large mod (or a server that streams more than
expected) grows the buffer without bound until OOM. Separately, the final
write goes directly to the destination path — if `write_all` fails partway
(disk full), a truncated file sits at the real plugin path; the caller's
rollback in `mod_install_service` never sees it because the download returned
`Err` before the mod was added to the `downloaded` list. Streaming to a
`.part` file with an incremental hash, then renaming into place only after
the checksum passes, fixes both with less code.

## Current state

- `src/backend/services/mod_download_service.rs:37-111` — the whole
  `download_mod` function. The relevant parts today:

  ```rust
  // lines 66-91: buffer accumulation
  let mut hasher = Sha256::new();
  let mut downloaded: u64 = 0;
  let mut buffer = Vec::new();
  let mut chunk = vec![0u8; 64 * 1024];
  let mut last_pct: i64 = -1;
  ...
  loop {
      let n = response.read(&mut chunk)?;
      if n == 0 { break; }
      hasher.update(&chunk[..n]);
      buffer.extend_from_slice(&chunk[..n]);
      downloaded += n as u64;
      // Throttle to whole-percent changes ...
      ...
  }

  // lines 93-106: verify, then write buffer to the final path
  emit_progress(&mod_id, downloaded, total_size, "verifying");
  let computed_checksum = format!("{:x}", hasher.finalize());
  if let Some(expected_checksum) = expected_checksum.filter(|checksum| !checksum.is_empty())
      && computed_checksum != expected_checksum.to_lowercase()
  {
      return Err(AppError::validation(format!(
          "Checksum mismatch: expected {}, got {}",
          expected_checksum, computed_checksum
      )));
  }

  emit_progress(&mod_id, downloaded, total_size, "writing");
  let mut file = File::create(dest_path)?;
  file.write_all(&buffer)?;
  ```

- The progress-event stages emitted, in order: `"connecting"`,
  `"downloading"` (percent-throttled), `"verifying"`, `"writing"`,
  `"complete"`. UI code matches on these strings — keep all five stages and
  their order (`"writing"` may become nearly instantaneous; that's fine).

- Caller: `src/backend/services/mod_install_service.rs:318-332` calls
  `download_mod(...)` and on `Err` runs `rollback(...)`, which only deletes
  files listed in `downloaded` — i.e. files whose download returned `Ok`.
  So any partial file left at `dest_path` on error is never cleaned up by
  the caller: cleanup must happen inside `download_mod`.

- Error convention: `AppResult` / `AppError` from `src/backend/error.rs`,
  propagate with `?`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Check | `cargo check --all-targets` | exit 0 |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test` | exit 0, all pass |

Note for this machine: `cargo` is not on PATH; in PowerShell run
`$env:Path += ";$env:USERPROFILE\.cargo\bin"` first.

## Scope

**In scope** (the only files you should modify):
- `src/backend/services/mod_download_service.rs`

**Out of scope** (do NOT touch):
- `src/backend/services/http_download.rs` — the generic downloader used by
  BepInEx/update paths. Consolidating the two is a separate, deliberately
  deferred refactor.
- `src/backend/services/mod_install_service.rs` — its rollback contract
  ("only Ok downloads appear in `downloaded`") continues to hold.
- The progress event shape (`ModDownloadProgress`) and stage strings.

## Git workflow

- Branch: `advisor/004-stream-mod-downloads`
- Commit style: short lowercase imperative subject (e.g. "stream mod downloads to disk").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Stream to a `.part` sibling with incremental hashing

Rework the middle of `download_mod`:

1. After creating the parent dir, define
   `let part_path = dest_path.with_extension("part");`
   (destination file names are mod plugin files, typically `.dll`; replacing
   the extension is fine because the final rename restores the real name).
2. Open `File::create(&part_path)?` *before* the read loop.
3. In the loop, replace `buffer.extend_from_slice(&chunk[..n]);` with a
   `file.write_all(&chunk[..n])` — keep `hasher.update` and the
   percent-throttled `emit_progress` exactly as they are.
4. Delete the `buffer` variable entirely.
5. Wrap the loop + checksum phase so that **any** error path removes
   `part_path` before returning. The cleanest shape: extract the
   loop-and-verify into an inner closure/function returning `AppResult<()>`,
   then at the call site:

   ```rust
   if let Err(e) = stream_and_verify(...) {
       let _ = fs::remove_file(&part_path);
       return Err(e);
   }
   ```

6. After the checksum passes (keep the existing comparison logic verbatim,
   including `.to_lowercase()` and the empty-string filter), emit the
   existing `"writing"` progress event, then `fs::rename(&part_path, dest_path)?`.
   Drop the file handle (end of scope or explicit `drop(file)`) **before**
   the rename — Windows cannot rename an open file.

**Verify**: `cargo check --all-targets` → exit 0.
**Verify**: `grep -n "Vec::new()" src/backend/services/mod_download_service.rs` → no match.

### Step 2: Lint and test

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.
**Verify**: `cargo test` → exit 0.

## Test plan

The function's core is network I/O, which this repo does not mock — no new
unit test is required. The safety properties are structural and checked by
the done criteria (no in-memory buffer, rename-after-verify, `.part` cleanup
on error). If you want a cheap guard, a test that
`Path::new("x.dll").with_extension("part")` equals `x.part` is acceptable
but optional.

## Done criteria

- [ ] No `Vec`-buffer accumulation remains in `download_mod`
- [ ] Download writes to `<dest>.part` and renames to `dest_path` only after
      checksum verification
- [ ] Every error path in `download_mod` after part-file creation removes the
      part file (read the diff to confirm; there is no automated check)
- [ ] All five progress stages still emitted in order
- [ ] `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test` all exit 0
- [ ] `git status` shows only `mod_download_service.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- The function at HEAD no longer matches the "Current state" excerpts.
- You find code elsewhere that depends on `download_mod` leaving the full
  file contents in memory (there should be none — the function returns `()`).
- The fix seems to require changing `mod_install_service.rs` — the contract
  was designed so it doesn't.

## Maintenance notes

- A future "consolidate download plumbing" refactor (deliberately deferred)
  should fold this checksum+part-file pattern into
  `http_download::download_file` so the BepInEx/update paths get it too.
- Reviewer should scrutinize: file handle dropped before rename (Windows),
  and part-file cleanup on the checksum-mismatch path specifically.
