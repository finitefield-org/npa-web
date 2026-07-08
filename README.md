# NPA Web

`npa-web` is a local browser tool for the human-facing NPA proof flow. It is a
standalone Rust workspace that depends on the public sibling `npa-core`
checkout for NPA crates and compact package fixtures. Keeping it outside the
`npa-core` workspace keeps web dependencies and checks off the core hot path.

M1 scope:

- Serve a usable proof page at `GET /`.
- Create an import-free Human session from browser source input.
- Run Human tactics through htmx form posts.
- Verify the closed Human proof state.
- Serve vendored htmx from the repository.
- Generate CSS with the Rust `ironframe` crate.

W2-01 adds a fixed standard-library demo:

- Select between the import-free demo and a standard-library demo in the
  browser.
- Load verified `Std.Nat.Basic` and `Std.Logic.Eq` certificates from embedded
  `../npa-core/testdata/package/npa-std` fixtures.
- Pass those verified imports explicitly to the Human API for the standard
  demo.
- Show the root declaration certificate hash and import export/certificate hash
  summary after verification.

W3-01 adds package fixture mode:

- Select a package fixture from a fixed server-side allowlist.
- Run package `check`, `build-certs --check`, and `verify-certs --checker fast`
  through existing package command APIs.
- Display package command diagnostics separately from proof-state authoring.
- Treat package diagnostics as untrusted metadata; only checker-backed
  `module_verified` diagnostics with live certificate evidence are proof
  evidence.

W4-01 adds LSP payload panels:

- Keep the baseline editor as a plain `<textarea>`.
- Render optional hover, completion, and code-action panels server-side.
- Request panel contents through htmx routes backed by existing
  `human_lsp_*` Human API adapters.
- Keep LSP payloads as Human UI metadata only; they do not enter `/machine/*`
  responses, certificate payloads, replay plans, or proof evidence.

Out of scope for the browser MVP:

- Arbitrary package roots, registry-backed package workflows, and dependency
  solving.
- CodeMirror, Monaco, editor workers, frontend bundlers, npm, Node.js,
  Tailwind CLI, or PostCSS.
- Persistence, collaboration, or multi-user isolation.
- JSON API clients.

## Run

From this directory:

```sh
cargo run
```

The default bind address is:

```text
127.0.0.1:7420
```

Open:

```text
http://127.0.0.1:7420
```

An explicit bind address may be passed for local development:

```sh
cargo run -- --bind 127.0.0.1:9000
```

Do not bind publicly unless that is an intentional local-tool decision for the
current run.

## Default Proof Smoke

The first screen is the proof tool itself. The default source is:

```npa
theorem id (A : Type) (x : A) : A := by exact x
```

Manual browser smoke:

1. Open `http://127.0.0.1:7420`.
2. Click `Create session`.
3. Run `intro A`.
4. Run `intro x`.
5. Run `exact x`.
6. Click `Verify`.
7. Confirm the verify panel shows `verified` and a certificate hash.

## Standard Library Demo Smoke

The `Standard library` selector fills this source:

```npa
import Std.Nat.Basic
import Std.Logic.Eq

theorem nat_self_eq (n : Nat) : Eq.{1} Nat n n := by
  intro n
  exact @Eq.refl.{1} Nat n
```

Manual browser smoke:

1. Open `http://127.0.0.1:7420`.
2. Select `Standard library`.
3. Click `Create session`.
4. Run `intro n`.
5. Run `exact @Eq.refl.{1} Nat n`.
6. Click `Verify`.
7. Confirm the verify panel shows `verified`, a root declaration certificate
   hash, and import summaries for `Std.Nat.Basic` and `Std.Logic.Eq`.

## Package Fixture Smoke

Manual browser smoke:

1. Open `http://127.0.0.1:7420`.
2. Select `npa-std` in `Package fixture`.
3. Click `Run package check`.
4. Confirm the package panel shows `passed`, the `package check`,
   `package build-certs`, and `package verify-certs` steps, and
   `module_verified` diagnostics for the allowed fixture.

## LSP Payload Panel Smoke

Manual browser smoke:

1. Open `http://127.0.0.1:7420`.
2. Select `Standard library`.
3. Click `Create session`.
4. In `Hover`, enter `Eq.refl` and click `Hover`.
5. Click `Completions` and `Code actions`.
6. Confirm the LSP panel shows a hover result, completion items, and code
   actions without replacing the source textarea or proof-state workspace.

## Verification

Use the nested workspace checks:

```sh
cargo fmt --all -- --check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

Container integration checks from the repository root:

```sh
git diff --check
rg -n 'tools/npa-web|../../crates|../../fixtures|../../../fixtures' npa-web --glob '!README.md'
rg -n '^version https://git-lfs.github.com/spec/v1$' .
```

The stale-path scan should have no hits. These checks confirm that the copied
web app now uses the standalone subtree layout and that no Git LFS pointer file
was introduced.

## Safety Boundary

The browser MVP calls existing Human API functions in process. It does not shell
out to `npa` for proof-state operations.

Browser input is intentionally narrow:

- Source input is limited to 128 KiB.
- Tactic input is limited to 4 KiB.
- Imports are rejected in the import-free demo.
- The standard-library demo only accepts the fixed `Std.Nat.Basic` and
  `Std.Logic.Eq` imports and loads them from embedded server-owned
  `npa-core/testdata` fixtures.
- Package fixture mode only accepts allowlisted fixture ids. Browser input is
  never interpreted as a filesystem path.
- Package diagnostics are untrusted metadata unless they are backed by a
  certificate checker verdict.
- Package fixture mode does not perform registry lookup, latest-version
  resolution, dependency solving, network fetches, or external checker runs.
- LSP hover/completion/code-action panels are Human UI metadata. They are not
  Machine API responses and are not certificate payloads.
- Path-like module/theorem names are rejected.
- Browser input does not name filesystem paths, execute commands, perform
  network fetches, or add dynamic imports.

The trusted NPA kernel, certificate format, independent checker, Machine API
schemas, hashes, fingerprints, and package-authoring helpers are not part of
this web tool milestone.

## Remaining Editor Limitations

- The source editor is still a plain textarea.
- There is no live cursor-position transport, incremental document sync, syntax
  highlighting, go-to-definition, semantic token rendering, or inlay hint UI.
- The current LSP panels are request/response previews, not a real LSP server.
