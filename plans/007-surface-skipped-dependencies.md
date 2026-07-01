# Plan 007: Log and surface dependencies that silently fail to resolve

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md` — unless a reviewer dispatched you and told you they
> maintain the index.
>
> **Drift check (run first)**: `git diff --stat 729f4fd..HEAD -- src/backend/services/mod_install_service.rs src/views/mod_detail.rs src/views/lobbies.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (001 recommended first)
- **Category**: bug / dx
- **Planned at**: commit `729f4fd`, 2026-07-01

## Why this matters

When a mod's transitive dependency fails to resolve (API fetch fails, no
version matches), `walk_dep` returns silently — no log line, no signal to the
caller. The install then "succeeds" without that dependency and the mod won't
load in game, with nothing to tell the user or a debugger why. The lobby path
already models the right answer: `plan_lobby_mods` returns an `unresolved`
list that the lobbies UI shows. This plan makes dependency resolution report
what it skipped: a `warn!` log at each failure point, and the unresolved list
propagated to both existing UIs.

## Current state

- `src/backend/services/mod_install_service.rs:79-124` — resolution as it
  exists today:

  ```rust
  pub fn resolve_dependencies(dependencies: &[ModDependency]) -> AppResult<Vec<ResolvedDependency>> {
      resolve_dependencies_excluding(dependencies, &HashSet::new())
  }

  pub fn resolve_dependencies_excluding(
      dependencies: &[ModDependency],
      skip: &HashSet<String>,
  ) -> AppResult<Vec<ResolvedDependency>> {
      let mut out = Vec::new();
      let mut visited: HashSet<String> = skip.clone();
      for dep in dependencies {
          walk_dep(dep, &mut visited, &mut out);
      }
      Ok(out)
  }

  fn walk_dep(dep: &ModDependency, visited: &mut HashSet<String>, out: &mut Vec<ResolvedDependency>) {
      if !visited.insert(dep.mod_id.clone()) {
          return;
      }
      let Ok(mod_item) = api::fetch_mod(&dep.mod_id) else {
          return;                                    // ← silent drop
      };
      let Ok(mut versions) = api::fetch_mod_versions(&dep.mod_id) else {
          return;                                    // ← silent drop
      };
      versions.sort_by_key(|version| std::cmp::Reverse(version.created_at));
      let Some(version) = resolve_version(&dep.version_constraint, &versions) else {
          return;                                    // ← silent drop
      };
      // Recurse into this dep's own dependencies first ...
      if let Ok(info) = api::fetch_mod_version_info(&dep.mod_id, &version) {
          for sub in &info.dependencies {
              walk_dep(sub, visited, out);
          }
      }
      out.push(ResolvedDependency { ... });
  }
  ```

  Note: `resolve_version` (lines 52-72) returns `None` only for an empty
  version list — otherwise it falls back to newest. So the practical silent
  drops are the two `api::` failures.

- Callers of `resolve_dependencies*`:
  1. `mod_install_service.rs:152` inside `plan_lobby_mods` — its enclosing
     function already returns `(Vec<InstallModInput>, Vec<String> /*unresolved*/)`
     but only top-level lobby mods land in `unresolved` (line 149); nested
     resolution failures vanish.
  2. `src/views/mod_detail.rs:178` — install panel dependency preview:
     `mod_install_service::resolve_dependencies(&deps).ok()`, then
     `.unwrap_or_default()` at line 181; rows rendered at lines 199-206.
  3. There is no other caller (`grep -rn "resolve_dependencies" src/`).

- Lobbies UI already surfaces `unresolved` — `src/views/lobbies.rs:1096`:
  `let (installable, unresolved) = mod_install_service::plan_lobby_mods(&required);`
  with a doc comment at lobbies.rs:960 describing how missing mods are shown.
  Nothing in lobbies.rs needs to change if `plan_lobby_mods` puts more ids
  into the same `unresolved` vec.

- Logging convention: `log::warn!` for retryable/degraded outcomes — see
  `warn!("update check failed: {e}")` at `src/workspace.rs:210`.

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
- `src/backend/services/mod_install_service.rs`
- `src/views/mod_detail.rs` (propagate + display unresolved in the install panel)

**Out of scope** (do NOT touch):
- `src/views/lobbies.rs` — it already consumes `unresolved`; it must keep
  compiling unchanged (that's part of the design constraint).
- `install_mods_for_profile` / `rollback` — installation mechanics unchanged.
- Retry logic for failed API fetches — out of scope; this plan is visibility.

## Git workflow

- Branch: `advisor/007-surface-skipped-dependencies`
- Commit style: short lowercase imperative subject (e.g. "surface unresolved mod deps").
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Make `walk_dep` report failures

Change `walk_dep` to take a fourth parameter `unresolved: &mut Vec<String>`.
At each of the three silent-drop points, before returning: `warn!` with the
mod id and the reason (include the error `e` for the two `api::` failures —
change the `let Ok(x) = … else` bindings to `match`/`Err(e)` form so the
error is loggable), and push `dep.mod_id.clone()` onto `unresolved`.

Keep the shape: resolution still never aborts the walk (a missing dep must
not block the rest — that's existing, intentional behavior).

### Step 2: Return the unresolved list from the public functions

Change both public functions to return the pair:

```rust
pub fn resolve_dependencies(
    dependencies: &[ModDependency],
) -> AppResult<(Vec<ResolvedDependency>, Vec<String>)>
```

(and the same for `resolve_dependencies_excluding`). Thread the vec through.
Keep `AppResult` even though nothing errors today — callers already handle it.

Update the two call sites:

- `plan_lobby_mods` (line 152): destructure the pair; `extend` its existing
  `unresolved` vec with the nested unresolved ids (avoid duplicates: only push
  ids not already present — a simple `contains` check is fine at these sizes).
- `mod_detail.rs:178`: capture both halves (see Step 3).

**Verify**: `cargo check --all-targets` → exit 0 (this confirms lobbies.rs
still compiles untouched).

### Step 3: Show unresolved deps in the mod-detail install panel

In `src/views/mod_detail.rs`:

1. In the background task at lines 172-181, return the pair
   `(Option<Vec<ResolvedDependency>>, Vec<String>)` instead of just the
   resolved list.
2. Add an `unresolved: Vec<String>` field to the install panel state struct
   (the struct behind `this.install` — find it by the `deps` and `status`
   fields assigned at lines 203-205).
3. In the `this.update` closure (lines 199-206), store the unresolved list on
   the panel alongside `deps`.
4. In the panel's render function (find where `panel.deps` rows are rendered),
   when `!panel.unresolved.is_empty()`, render one muted warning line in the
   panel, styled like existing secondary text (`theme.text_muted` is the
   repo's muted color, e.g. `src/views/explore.rs:237`):
   `format!("{} dependencies could not be resolved and will be skipped: {}", n, ids.join(", "))`.

**Verify**: `cargo check --all-targets` → exit 0.

### Step 4: Lint and test

**Verify**: `cargo clippy --all-targets -- -D warnings` → exit 0.
**Verify**: `cargo test` → exit 0.

## Test plan

`walk_dep` calls the live API, so its failure paths can't be unit-tested
without a network mock this repo doesn't have. The machine-checkable part is
compilation of all three call sites plus clippy. Manual spot check: run the
app, open a mod with dependencies — the install panel must look unchanged
when everything resolves.

## Done criteria

- [ ] `grep -n "return;" src/backend/services/mod_install_service.rs` inside
      `walk_dep` — every early return except the `visited` dedup is preceded
      by a `warn!` and an `unresolved.push`
- [ ] `resolve_dependencies` / `_excluding` return the `(resolved, unresolved)` pair
- [ ] `plan_lobby_mods` folds nested unresolved ids into its existing
      `unresolved` return value, without duplicates
- [ ] Mod-detail install panel stores and renders the unresolved list
- [ ] `src/views/lobbies.rs` has zero diff
- [ ] `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test` all exit 0
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- The code at the cited locations doesn't match the excerpts (drift).
- You find a third caller of `resolve_dependencies*` not listed above.
- The install-panel struct doesn't have an obvious place for the new field or
  its render function can't be located from the `panel.deps` usage — report
  the actual structure instead of restructuring the view.

## Maintenance notes

- If a retry/backoff layer is ever added to `api::fetch_*`, the `warn!`s here
  become the signal for how often it's needed.
- Reviewer should scrutinize: the dedup guard in `walk_dep` (`visited`) must
  NOT add to `unresolved` — already-visited is success, not failure.
- Deferred: blocking install when a *required*-type dependency is unresolved
  (vs. just warning). Needs a product decision about `dependency_type`.
