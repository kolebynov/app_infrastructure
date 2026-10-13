# Repository Guidelines

## Project Overview

`app_infrastructure` is a small Rust library for layered application configuration and optional configuration-driven tracing. It is not a standalone application; `examples/tracing.rs` demonstrates integration.

## Architecture & Data Flow

- `src/lib.rs` exposes `app_config`, re-exports the `config` crate, and defines `BoxError = Box<dyn Error + Send + Sync>`. `app_tracing` is available only with the `app_tracing` feature.
- `AppConfigurationBuilder` reads `<prefix>_ENVIRONMENT` (default: `RUST_APP_ENVIRONMENT`, falling back to `dev`). `dev`/`prod` are recognized case-insensitively; custom environment names retain their spelling.
- Default configuration precedence, lowest to highest: optional `app_settings` file → optional `app_settings.<environment>` file → prefixed environment variables. File paths are relative to the process working directory; enabled Cargo format features determine supported extensions.
- Environment keys use `_` after the prefix and `__` between nested keys: `RUST_APP_DATABASE__HOST` maps to `database.host`. Values use `config`'s automatic parsing.
- `build_with_custom_config_builder` accepts a `FnOnce(ConfigBuildingInfo) -> ConfigBuilder<DefaultState>`. It replaces the entire default source pipeline, rather than appending to it. The result retains both the selected environment and the built `Config`.
- `app_tracing::init_from_config(&Config)` deserializes the `tracing` section, builds filtered subscriber layers, and installs a process-global subscriber via `try_init()`. Initialize once; repeated initialization returns an error. A missing section uses defaults with no output layers; malformed configuration propagates an error.
- Each tracing layer selects `stdout` (ANSI enabled) or `rolling_file` (hourly rotation, ANSI disabled). `filter` is an array of tracing filter directives; a layer can override the top-level filter. See `examples/app_settings.json` for the schema.

## Key Directories

- `src/`: library API and implementation; configuration and tracing remain separate modules.
- `examples/`: runnable integration example and JSON tracing configuration. Run from the repository root so `examples/app_settings.json` resolves.
- `.vscode/`, `.zed/`: Rust editor configuration; both enable all Cargo features for rust-analyzer.

## Development Commands

Run from the repository root:

```sh
cargo build                                      # Default JSON configuration support
cargo build --all-features                       # All formats and tracing
cargo test --all-features                        # Unit, integration, and doc tests
cargo fmt -- --check                             # Check formatting without rewriting files
cargo clippy --all-targets --all-features         # Include feature-gated code and examples
cargo run --example tracing --features app_tracing
```

The example uses its own `EXAMPLE_` environment source, not the default `RUST_APP_` source. For example, `EXAMPLE_CONFIG__VALUE=hello cargo run --example tracing --features app_tracing` supplies the value it logs. It also writes hourly rotated `examples/test.log.*` files; these are not ignored by `.gitignore`. Do not commit generated logs.

## Code Conventions & Common Patterns

- Follow Rust naming: `snake_case` modules/functions/fields, `PascalCase` types/variants, `SCREAMING_SNAKE_CASE` constants. Use rustfmt and keep unrelated formatting changes out of focused patches.
- Builder customization consumes `self` and returns the updated builder. Configuration source injection uses closures; tracing writer customization uses the private `LayerConfigurator` trait with associated types and boxed layers for heterogeneous writers. Reuse these patterns rather than introducing a DI framework.
- Use `Result` and `?`: configuration building returns `ConfigError`; tracing setup returns `BoxError`. Do not hide malformed configuration behind defaults; the current tracing fallback handles only `ConfigError::NotFound`.
- Tracing configuration derives Serde `Deserialize`; `WriterConfig` uses `snake_case` variant names. `EnvFilterWrapper` implements custom sequence deserialization for directive arrays. Preserve this external schema when editing internals.
- APIs are synchronous; there is no async runtime or application state store. State is carried by owned configuration structs, except for the process-global tracing subscriber.
- Preserve Cargo feature isolation: default consumers must not require tracing dependencies. Validate optional-feature changes with both default and all-feature builds.

## Important Files

- `Cargo.toml`: package metadata, optional dependencies, file-format features, and the example's required tracing feature.
- `Cargo.lock`: resolved dependency versions.
- `src/lib.rs`: public module boundary, dependency re-export, and shared error type.
- `src/app_config.rs`: source precedence, environment naming, and builder API.
- `src/app_tracing.rs`: Serde schema, filter parsing, writer setup, and global initialization.
- `examples/tracing.rs`, `examples/app_settings.json`: integration entry point and concrete configuration schema.

## Runtime/Tooling Preferences

Use Rust and Cargo, not Node or Bun. The crate uses Rust edition 2024, requiring an edition-capable compiler (Rust 1.85+); no `rust-version` or pinned toolchain is declared. Use rustfmt, Clippy, and rust-analyzer.

Default features enable only `config_json`. Tracing requires `app_tracing`; additional format features are `config_toml`, `config_yaml`, `config_ini`, `config_ron`, and `config_json5`. To select a format without JSON, use, for example, `cargo build --no-default-features --features config_toml`.

## Testing & QA

No unit tests, integration tests, doctests, CI workflows, or coverage thresholds are currently present. Cargo's standard test harness is the applicable framework; a successful `cargo test` alone does not prove behavior when there are no tests.

For changes, build with default and all features, run tests and lint checks, and exercise the tracing example. For configuration changes, check source precedence, missing optional files, custom environments, and nested environment overrides. For tracing changes, check invalid filters, per-layer filtering, file output, and repeated initialization errors. Keep tests deterministic: isolate process-global subscriber initialization and environment-variable scenarios in separate processes where needed.
