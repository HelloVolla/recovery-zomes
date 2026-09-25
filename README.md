# recovery-zomes

Holochain zomes for Volla Recovery, packaged as reusable cargo crates.

This repository is the single source of the DNA. It contains the coordinator and integrity zomes, the `shared-types` and `integrity-types` crates they share with clients, and the zome integration tests.

## Crates

| Crate                | Path                                              | Purpose                                |
| -------------------- | ------------------------------------------------- | -------------------------------------- |
| `membrane_integrity` | `crates/dnas/recovery/zomes/integrity/membrane`   | Membrane proof validation              |
| `registry_integrity` | `crates/dnas/recovery/zomes/integrity/registry`   | Registry entry types                   |
| `storage_integrity`  | `crates/dnas/recovery/zomes/integrity/storage`    | Storage entry and chunk types          |
| `membrane`           | `crates/dnas/recovery/zomes/coordinator/membrane` | Membrane coordinator                   |
| `registry`           | `crates/dnas/recovery/zomes/coordinator/registry` | Registry coordinator                   |
| `storage`            | `crates/dnas/recovery/zomes/coordinator/storage`  | Storage coordinator                    |
| `shared_types`       | `crates/shared-types`                             | Types shared between zomes and clients |
| `integrity_types`    | `crates/integrity-types`                          | Integrity-layer types                  |
| `zomes_tests`        | `crates/tests`                                    | Zome integration tests (sweettest)     |

The integrity zomes are compiled to wasm and run inside the DNA. `shared_types` and `integrity_types` are also linked into native clients so both sides agree on the wire format.

## Using the zomes from another cargo project

Pin a revision. Point every crate at the same `rev`, so the types compiled into your client and the wasm running in the DNA come from one commit. Use the
`https` URL: this repository is public, so cargo fetches it with no credentials.

```toml
[dependencies]
shared_types = { git = "https://github.com/HelloVolla/recovery-zomes.git", rev = "<commit>" }
```

```bash
cargo add --git https://github.com/HelloVolla/recovery-zomes.git --rev <commit> storage_integrity
```

To work against a local checkout instead, add a `[patch]` override in the consumer's `Cargo.toml`. The URL key must match the git dependency exactly:

```toml
[patch."https://github.com/HelloVolla/recovery-zomes.git"]
shared_types = { path = "../recovery-zomes/crates/shared-types" }
```

`[patch]` still requires the git URL to be fetchable, so the remote must exist before the override takes effect.

## Consuming the packaged DNA

Clients should **not** build the zomes themselves. Download the released hApp instead, so everyone ends up on the same DHT:

```bash
gh release download v0.1.0 --repo HelloVolla/recovery-zomes --pattern '*.happ'
```

### Why the DNA is not built by consumers

The DnaHash is derived from the compiled wasm bytes, so it changes whenever the build environment changes: rustc version, cargo release profile (`lto`,
`opt-level`), the resolved dependency versions, or which workspace root runs the build. Any of these produce wasm that does not match the released hApp, and a
DNA with a different hash lives on a different DHT.

A consumer that builds these crates from its own workspace root gets a different DnaHash than a build in this repository, even from the same commit. Measured on one machine for an identical source commit:

| Build context                                        | DnaHash                                                 |
| ---------------------------------------------------- | ------------------------------------------------------- |
| `cargo build` in this repository                     | `uhC0kwV1EsBtaBCe9nizwPGRODr853tS_9p-l9moFFmdwrbPw_qUt` |
| the same commit built from a consumer workspace root | different                                               |
| `Cargo.lock` regenerated instead of committed        | different                                               |

The committed `Cargo.lock`, `rust-toolchain.toml` and `[profile.release]` are load-bearing parts of the DNA, not build hygiene. Build and release only through `scripts/package.sh` in this repository, and consume the released `.happ`.

## Building

Requires Nix (for the pinned `hc` CLI and Rust toolchain from `flake.lock`):

```bash
nix develop --no-update-lock-file
./scripts/package.sh
```

`scripts/package.sh` compiles all zomes for `wasm32-unknown-unknown` in release mode, then packs `crates/dnas/recovery/workdir/volla_recovery.dna` and `workdir/volla_recovery.happ`.

Print the current DNA hash:

```bash
hc dna hash crates/dnas/recovery/workdir/volla_recovery.dna
```

### Build inputs that must not drift

- `rust-toolchain.toml` pins rustc. A newer rustc fails to link the zomes (undefined `__hc__*` host symbols).
- `.cargo/config.toml` sets `--cfg=getrandom_backend="custom"` for `wasm32-unknown-unknown`. Without it the build fails inside `getrandom`. Cargo reads this file from the current directory upward, so it does not apply when a consumer builds these crates from its own workspace root.
- `[profile.release]` in the root `Cargo.toml` affects the wasm bytes. A consumer's profile wins when the consumer runs the build, which is another reason to build the DNA here.
- `Cargo.lock` is committed. Updating it can pull in newer Holochain crates whose APIs drifted, which changes the wasm and can break `crates/tests`.

## Tests

```bash
cargo test-all                    # all features
cargo test-feature single_agent   # one feature
nix develop --command cargo test-all
```

The suite needs a packaged DNA at `crates/dnas/recovery/workdir/volla_recovery.dna`, so run `./scripts/package.sh` first.

Note: `crates/tests` does not currently compile. The holochain 0.7 update changed the sweettest API (`SweetConductor::from_standard_config`, `SweetConductorBatch::from_config`, `RoleSettings::Provisioned::init_properties`, `InstallAppPayload::restore_from_dht`) and the tests were not updated. This predates the repository split; the same errors exist in the previous history.
