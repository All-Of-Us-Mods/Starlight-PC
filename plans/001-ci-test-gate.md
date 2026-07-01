# Plan 001: Run the test suite in CI and document local verification

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- .github/workflows/ci.yml README.md`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: tests / dx
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

The repo has a real test suite (3 tests across `src/backend/error.rs`,
`src/backend/services/finder_service.rs`, `src/backend/state/game_runtime.rs`)
but CI only runs `cargo check` and `cargo clippy` — the tests are never
executed anywhere. Any test added in the future is dead weight until this
lands, and several other plans in `plans/` add tests that need this gate to
be meaningful. This plan makes CI run `cargo test` on both platforms and
documents the one-command local verification sequence.

As of commit `729f4fd`, `cargo test` compiles and passes on Windows:
`test result: ok. 3 passed; 0 failed` (verified 2026-07-01, ~1 min warm build).

## Current state

- `.github/workflows/ci.yml` — two jobs. `check` (ubuntu, runs inside
  `nix develop`) and `check-windows` (windows, rustup stable). Each job has
  exactly two build steps:

  ```yaml
  # ci.yml, job "check" (ubuntu):
      - name: cargo check
        run: nix develop --command cargo check --all-targets

      - name: cargo clippy
        run: nix develop --command cargo clippy --all-targets -- -D warnings

  # ci.yml, job "check-windows":
      - name: cargo check
        run: cargo check --all-targets

      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
  ```

- `README.md` — "Development" section currently documents only:

  ```bash
  cargo run            # Start in development mode
  cargo build --release # Build for production
  ```

- Convention: the Linux job wraps every cargo invocation in
  `nix develop --command …`; the Windows job calls cargo directly. Match that.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Tests (local) | `cargo test` | exit 0, `3 passed; 0 failed` |
| Lint | `cargo clippy --all-targets -- -D warnings` | exit 0, no warnings |
| YAML sanity | open the file; CI validates on push | n/a |

Note for this machine: `cargo` is not on PATH; in PowerShell run
`$env:Path += ";$env:USERPROFILE\.cargo\bin"` first.

## Scope

**In scope** (the only files you should modify):
- `.github/workflows/ci.yml`
- `README.md`

**Out of scope** (do NOT touch):
- `.github/workflows/build.yml` and `.github/workflows/release.yml` — release
  pipelines; unrelated.
- Adding new tests (other plans do that).
- Adding a Makefile/justfile — the repo has no task-runner convention; keep
  verification as documented cargo commands.

## Git workflow

- Branch: `advisor/001-ci-test-gate`
- Commit style: short lowercase imperative subject (repo examples:
  "cargo fmt", "fix clippy", "add lobby browser").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add a `cargo test` step to both CI jobs

In `.github/workflows/ci.yml`, add after the `cargo clippy` step of each job:

Job `check` (ubuntu):
```yaml
      - name: cargo test
        run: nix develop --command cargo test
```

Job `check-windows`:
```yaml
      - name: cargo test
        run: cargo test
```

**Verify**: `git diff .github/workflows/ci.yml` shows exactly two added steps,
one per job, with the nix wrapper only on the ubuntu job.

### Step 2: Document local verification in README

In `README.md`, extend the "Development" section's code block with a
verification line so it reads:

```bash
cargo run             # Start in development mode
cargo build --release # Build for production
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test  # What CI runs
```

**Verify**: the README code block contains all three lines; no other README
sections changed.

### Step 3: Run the suite locally

**Verify**: `cargo test` → exit 0, output contains `3 passed; 0 failed`.
**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.

## Test plan

No new tests — this plan wires the existing suite into CI. The verification
is Step 3 plus CI going green on the branch.

## Done criteria

- [ ] `.github/workflows/ci.yml` has a `cargo test` step in both jobs
- [ ] `cargo test` exits 0 locally with `3 passed`
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `git status` shows no modified files outside the in-scope list
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `cargo test` fails locally before your change — the suite is broken at HEAD
  and gating CI on it would block everyone; report the failing test instead.
- The CI file's job names or step layout no longer match the excerpts above.
- `cargo test` takes longer than ~20 minutes (would indicate the git
  dependencies are being rebuilt from scratch in a way that needs CI caching
  work beyond this plan's scope — note it, still land the change if it passes).

## Maintenance notes

- Plans 003–007 add or rely on tests; they assume this gate exists.
- Reviewer should confirm the nix wrapper is used for the ubuntu step —
  a bare `cargo test` there would use the wrong toolchain.
- Deferred: CI test-result caching/parallelization — not worth it at 3 tests.
