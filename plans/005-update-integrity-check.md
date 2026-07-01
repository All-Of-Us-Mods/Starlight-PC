# Plan 005: Verify the self-update binary against the GitHub asset digest

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/backend/services/update_service.rs`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S–M
- **Risk**: LOW
- **Depends on**: 001 (test gate — the new unit tests should run in CI)
- **Category**: security (defense-in-depth hardening)
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

The Windows self-update downloads an exe named in the GitHub release and
swaps it in for the running binary — an arbitrary-code-execution path guarded
today only by TLS. There is no integrity check on the downloaded bytes and no
check that the asset URL points at GitHub at all. Mod downloads in this same
codebase are SHA-256-verified; the far more sensitive update path is not.
GitHub's release API now serves a `digest` field (`"sha256:<hex>"`) per asset
— verified present on this repo's latest release (v1.1.1, checked 2026-07-01)
— so an independent end-to-end integrity check is a small change: the hash
travels via `api.github.com` while the bytes travel via the release-download
CDN, meaning corruption or tampering on the download path is caught. (This
does not defend against a full GitHub-account compromise; that would need
release signing, deliberately out of scope.)

## Current state

- `src/backend/services/update_service.rs` — the whole service (127 lines).
  Key excerpts at `729f4fd`:

  ```rust
  // lines 23-33
  #[derive(Deserialize)]
  struct GithubAsset {
      name: String,
      browser_download_url: String,
  }

  // lines 17-21
  #[derive(Debug, Clone)]
  pub struct UpdateInfo {
      pub version: String,
      pub download_url: String,
  }

  // lines 63-77: asset selection inside check_for_update()
  let Some(asset) = release
      .assets
      .iter()
      .find(|a| a.name.eq_ignore_ascii_case("Starlight-windows-x86_64.exe"))
  else { ... return Ok(None); };
  ...
  Ok(Some(UpdateInfo {
      version: latest.to_string(),
      download_url: asset.browser_download_url.clone(),
  }))

  // lines 86-113: apply_update_and_relaunch (cfg(windows))
  http_download::download_file(&info.download_url, &download_path, |_, _| {})?;
  let _ = fs::remove_file(&old_path);
  fs::rename(&current_exe, &old_path)?;
  ...
  ```

- `UpdateInfo` is constructed only in `check_for_update` and consumed only in
  `src/workspace.rs` (notification → `install_update` at workspace.rs:592-619,
  which passes it straight to `apply_update_and_relaunch`). Adding a field is
  a two-file-max change, and workspace.rs does not name the fields — it just
  clones the struct — so it likely needs no edit.

- Hashing exemplar in this repo:
  `src/backend/services/mod_download_service.rs:66,79,94` —
  `Sha256::new()` / `hasher.update(&chunk[..n])` /
  `format!("{:x}", hasher.finalize())`. `sha2` is already a dependency.

- Error convention: `AppResult` / `AppError` (`AppError::validation(...)`,
  `AppError::parse(...)`) from `src/backend/error.rs`.

- Test-module convention: `#[cfg(test)] mod tests` at the bottom of the file
  — see `src/backend/services/finder_service.rs:434-443`.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Check | `cargo check --all-targets` | exit 0 |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test` | exit 0, all pass incl. new digest tests |
| Live API sanity (read-only, optional) | `curl -s https://api.github.com/repos/All-Of-Us-Mods/Starlight-PC/releases/latest` | assets have `"digest": "sha256:…"` |

Note for this machine: `cargo` is not on PATH; in PowerShell run
`$env:Path += ";$env:USERPROFILE\.cargo\bin"` first. This code is
`cfg(windows)` in part — you are on Windows, so `--all-targets` compiles it.

## Scope

**In scope** (the only files you should modify):
- `src/backend/services/update_service.rs`
- `src/workspace.rs` (only if the `UpdateInfo` construction/consumption
  requires it — it shouldn't)

**Out of scope** (do NOT touch):
- `src/backend/services/http_download.rs` — reused as-is.
- Release signing / signature verification — future work, bigger lift.
- The release workflow (`.github/workflows/release.yml`).
- Linux self-update — doesn't exist yet (separate direction item).

## Git workflow

- Branch: `advisor/005-update-integrity-check`
- Commit style: short lowercase imperative subject (e.g. "verify update digest").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Carry the digest through `UpdateInfo`

1. Add `digest: Option<String>` to `GithubAsset` (serde will accept its
   absence on old releases).
2. Add a helper near the structs:

   ```rust
   /// Parse a GitHub asset digest of the form "sha256:<64 hex chars>" into
   /// the lowercase hex hash. Returns None for any other shape.
   fn parse_sha256_digest(digest: &str) -> Option<String>
   ```

   Implementation: strip a `"sha256:"` prefix (case-insensitive is fine),
   require exactly 64 remaining chars, all `is_ascii_hexdigit()`, return
   lowercased.
3. Add `pub expected_sha256: Option<String>` to `UpdateInfo`, populated in
   `check_for_update` via
   `asset.digest.as_deref().and_then(parse_sha256_digest)`.

**Verify**: `cargo check --all-targets` → exit 0.

### Step 2: Restrict the download URL to this repo's releases

In `check_for_update`, after selecting the asset, reject unexpected hosts:

```rust
const RELEASE_DOWNLOAD_PREFIX: &str =
    "https://github.com/All-Of-Us-Mods/Starlight-PC/releases/download/";

if !asset.browser_download_url.starts_with(RELEASE_DOWNLOAD_PREFIX) {
    return Err(AppError::validation(format!(
        "Unexpected update download URL: {}",
        asset.browser_download_url
    )));
}
```

**Verify**: `cargo check --all-targets` → exit 0.

### Step 3: Verify the hash before swapping the exe

In `apply_update_and_relaunch`, immediately after the
`http_download::download_file(...)` call and **before** any rename:

1. Fail closed when the release has no digest:

   ```rust
   let Some(expected) = info.expected_sha256.as_deref() else {
       let _ = fs::remove_file(&download_path);
       return Err(AppError::validation(
           "Release asset has no sha256 digest; refusing to install update",
       ));
   };
   ```

2. Hash the downloaded file (streaming, reusing the repo's Sha256 idiom —
   read in a `[0u8; 64 * 1024]` loop from `File::open(&download_path)?`)
   and compare to `expected` (both lowercase hex). On mismatch, remove
   `download_path` and return
   `AppError::validation(format!("Update checksum mismatch: expected {expected}, got {computed}"))`.

Only when the hash matches does the existing rename/swap sequence run,
unchanged.

**Verify**: `cargo check --all-targets` → exit 0.

### Step 4: Unit tests + lint

Add `#[cfg(test)] mod tests` at the bottom of `update_service.rs` (NOT inside
the `cfg(windows)` block) covering `parse_sha256_digest`:
- valid `"sha256:<64 hex>"` → `Some(lowercased)`
- uppercase hex accepted and lowercased
- wrong prefix (`"sha512:…"`), wrong length, non-hex chars, empty string → `None`

**Verify**: `cargo test` → exit 0, new tests listed as passing.
**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.

## Test plan

- Unit tests for `parse_sha256_digest` as listed in Step 4 (5 cases minimum),
  modeled structurally on `finder_service.rs`'s test module.
- The end-to-end path (download → hash → swap) is exercised manually on the
  next real release; it cannot be unit-tested without a network mock this
  repo doesn't have.

## Done criteria

- [ ] `GithubAsset` deserializes `digest`; `UpdateInfo` carries `expected_sha256`
- [ ] `apply_update_and_relaunch` refuses to install when the digest is
      missing or mismatched, and removes the downloaded file in both cases
- [ ] `check_for_update` rejects download URLs outside
      `https://github.com/All-Of-Us-Mods/Starlight-PC/releases/download/`
- [ ] `cargo test` exits 0 with the new `parse_sha256_digest` tests passing
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `git status` shows only in-scope files modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `update_service.rs` at HEAD doesn't match the "Current state" excerpts.
- Serde fails to deserialize the release JSON with the new field on a live
  check (would indicate the API shape assumption is wrong — report the raw
  field shape you observed instead of guessing).
- You are tempted to make the missing-digest case a warning instead of an
  error — that's a security-posture decision; the plan says fail closed, and
  changing it needs the maintainer's sign-off.

## Maintenance notes

- Fail-closed consequence: if GitHub ever stops serving `digest`, updates
  will refuse to install until this code is revisited — that is intentional.
- Future work (deferred): sign release binaries and verify the signature with
  an embedded public key; that also covers GitHub-account compromise.
- Reviewer should scrutinize: the downloaded file is deleted on *both*
  failure paths, and the hash comparison happens before the first rename.
