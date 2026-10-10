# Ocalculator v1.0.0 — Release process

Target: production `1.0.0` ready for ODS-os integration, APO packaging, and
first public release. Architecture is frozen (no merges, no logic in UI,
Engine/Modules/Iced UI separated).

## 1. Version and metadata

- Workspace version is `1.0.0` in the root `Cargo.toml`
  (`[workspace.package] version`). All crates use `version.workspace = true`.
- Package metadata (`authors`, `description`, `license`, `repository`,
  `homepage`, `edition`, `rust-version`) is inherited from
  `[workspace.package]` — verify with:
  ```bash
  grep -A7 '\[workspace.package\]' Cargo.toml
  for f in crates/*/Cargo.toml; do echo "== $f"; grep -E 'version|description' "$f" | head -n 3; done
  ```
- `Cargo.lock` is committed and kept stable for release. Do not delete it.
  Update only via `cargo update -p <crate>` when a security fix requires it,
  then re-run the full checklist.

## 2. Quality gates (must all pass)

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all --check
cargo build --release
```

- Tests: every crate ships unit tests + engine integration/regression/
  security/stress suites. Zero failures allowed.
- Clippy: zero warnings (`--all-targets` covers tests + benches). Fixed
  before release: `unnecessary_sort_by`, `manual_is_multiple_of`,
  `assertions_on_constants` (allowed with justification), `neg_cmp_op_on_partial_ord`
  (rewritten with `partial_cmp`), dead-code `EvalOutcome::display` (documented
  dual-formatting, `#[allow(dead_code)]`).
- Fmt: `cargo fmt --all` then re-check.
- Release build: `opt-level=3`, `lto=true`, `codegen-units=1`,
  `panic="abort"`, `strip=true` (see root `Cargo.toml`). Debug builds use the
  default dev profile and must keep working.

Smoke-test the release binaries:

```bash
./target/release/ocalc eval "sqrt(25) * 10 + 2^3"          # 58
./target/release/ocalc eval "6 AND 3"
./target/release/ocalc units 1 km m                        # 1000
./target/release/ocalc functions | head
./target/release/ocalc modules
./target/release/ocalc history show --limit 5
echo "1+2" | ./target/release/ocalc
./target/release/ocalc-desktop                              # launches, no stderr errors
```

## 3. Configuration and data (XDG)

| Kind | Location | Contents |
|---|---|---|
| Config | `$XDG_CONFIG_HOME/ocalculator/` (fallback `~/.config/ocalculator/`) | `settings.json` (versioned, `config_version: 1`) |
| Data | `$XDG_DATA_HOME/ocalculator/` (fallback `~/.local/share/ocalculator/`) | `history.json` (versioned), `logs/ocalculator.log` |
| Cache | `$XDG_CACHE_HOME/ocalculator/` (fallback `~/.cache/ocalculator/`) | reserved for future cached data |

- `ocalc-history/src/paths.rs` owns resolution (`config_dir_for` /
  `data_dir_for` / `cache_dir_for` pure functions + env-backed wrappers),
  `ensure_dirs`, `ensure_parent`, `migrate_legacy_to` / `migrate_legacy`,
  `log_event` / `log_startup`.
- Migration is non-destructive copy (legacy files kept as backups):
  `~/.config/ocalc-desktop/settings.json` → new `settings.json`,
  `~/.local/share/ocalc/history.json` → new `history.json`. Never overwrites
  an existing new file.
- `settings.json` forward rule: unknown future `config_version` resets to
  defaults (never crashes on newer configs); missing version (pre-1.0) is
  treated as v1 and clamped.
- `history.json` forward rule: unknown future `format_version` is rejected
  with a clear error; legacy bare stores (no envelope) still load.

## 4. Logging (no silent failures)

Log file: `~/.local/share/ocalculator/logs/ocalculator.log`
(`[RFC3339] [tag] message`, parent dirs auto-created, all I/O best-effort).

Logged: startup banner (`[startup] ocalc[v] starting (config=…, data=…,
cache=…)`), module inventory + load failures (`[module]`), history load/save
failures (`[history]`), settings autosave failures (`[config]`), fatal UI
errors (`[ui]`). Every log site also prints to stderr (`warning:` / `error:`).

## 5. Module system hardening

- `ModuleMetadata::validate`: kebab-case id (1–64 chars), semver version,
  non-empty description (≤256), non-empty capabilities, `format_version == 1`.
- `ModuleMetadata::check_compatibility(engine_api)`: major must match; module
  minor ≤ engine minor; prerelease requires exact match. Clear
  `IncompatibleVersion` messages.
- Registration: `register` (validate + init) and `register_checked` (validate
  + compatibility + init). Bulk `register_all` collects all errors. Hosts use
  `validate_all` / `check_compatibility_all` for diagnostics.
- Trust: `ModuleSource::{Builtin, External{path}, Repository{url}}` +
  `signature: Option<String>` + `verify_signature()` (built-ins always trusted;
  external/repository require a non-empty signature → else `UntrustedSource`
  with remediation hint). Crypto verification is out of scope for v1; the API
  is reserved so ODS hosts can enforce it without breakage.
- CLI (`new_calculator`, `:enable`/`:disable`, `modules`) and desktop
  (`modules::discover/register/create`, settings panel, `module_load_report`)
  share the same four ids (`cas`, `chemistry`, `finance`, `physics`) and the
  same stable calls (`CalculatorEngine::register_module_checked` /
  `unregister_module` / `describe_modules` / `function_names` /
  `sample_graph`). No calculation logic in UI.

## 6. Testing checklist (final gate)

- [ ] Version is `1.0.0` (`Cargo.toml` workspace + `ocalc --version` +
      desktop title + metainfo release).
- [ ] All tests pass (`cargo test --workspace`).
- [ ] Clippy clean (`cargo clippy --workspace --all-targets`).
- [ ] Format clean (`cargo fmt --all --check`).
- [ ] Release build works (`cargo build --release`, smoke tests above).
- [ ] Desktop launcher works (`ocalculator.desktop` validates, launches
      `ocalculator`, icon renders).
- [ ] Settings persist (change theme/precision → restart → restored;
      `settings.json` has `config_version: 1`).
- [ ] History works (evaluate → `history show` / sidebar lists it; restart →
      still there; search finds it).
- [ ] Theme works (light/dark + accents + font scale apply instantly).
- [ ] All modules load (`ocalc modules` lists 4; desktop settings shows 4
      enabled; `force(2,3)=6`, `molar_mass(1,2,8,1)≈18.015`,
      `compound(1000,0.05,1)=1050`, `solve_linear(2,-4)=2`).
- [ ] CLI works (`eval`, REPL, `graph --format csv|json`, `units`,
      `functions`, `history`, piped stdin).
- [ ] Package structure is ready (`ocalculator-1.0.0-x86_64/usr/...` complete,
      `make-package.sh` reproduces it, checksums recorded).
- [ ] Logs exist (`logs/ocalculator.log` has `[startup]` + `[module]` lines,
      no silent failures).
- [ ] XDG respected (`XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_CACHE_HOME`
      overrides change locations; legacy files migrate once).

## 7. Publishing

1. Tag `v1.0.0` on a green checklist.
2. Attach `ocalculator-1.0.0-x86_64.tar.gz` (built from the package tree) +
   `sha256sums.txt` + release notes (features, XDG migration notes, module
   list).
3. File ODS-os integration ticket with: package path, `.desktop` + metainfo
   validation logs, smoke-test transcript, log sample.
