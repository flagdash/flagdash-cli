# FlagDash CLI

Interactive terminal UI for [FlagDash](https://flagdash.io) — manage feature flags, remote configs, AI configs, and webhooks directly from your terminal. Cross-platform, built with Rust and [Ratatui](https://ratatui.rs).

## Installation

### Homebrew (macOS/Linux)

```sh
brew install flagdash/tap/flagdash
```

### Shell script

```sh
curl -fsSL https://flagdash.io/install.sh | sh
```

### npm

```sh
npm install -g @flagdash/cli
```

### Cargo (build from source)

```sh
cargo install flagdash-cli
```

## Quick Start

```sh
# Launch the TUI (prompts for API key on first run)
flagdash

# Or pass options directly
flagdash --api-key management_xxx --project-id prj_xxx --environment-id env_xxx
```

On first run, enter your management API key. It will be saved to `~/.config/flagdash/config.toml`.

## Features

- **Dashboard** — Overview with flag/config/webhook/AI config counts
- **Flags** — List, create, edit, delete, toggle per environment, set rollout percentage, manage targeting rules, A/B variations, and schedules
- **Remote Config** — List, create, edit, delete, set values per environment
- **AI Configs** — List, create, edit, delete markdown-based AI config files with folder grouping
- **Webhooks** — List, create, edit, delete endpoints, view delivery logs
- **Environments** — View all environments (read-only)
- **Search** — Filter flags, configs, and AI configs with `/`
- **Read-only mode** — Automatically detected for `client_` and `server_` API keys

## Keyboard Shortcuts

### Global

| Key | Action |
|-----|--------|
| `1-6` | Switch sidebar sections |
| `j/k` or `↑↓` | Navigate lists |
| `Enter` | Open detail view |
| `Esc` | Go back |
| `/` | Search/filter |
| `q` | Quit |

### List Views

| Key | Action |
|-----|--------|
| `c` | Create new resource |
| `d` | Delete selected |
| `t` | Toggle flag (flags only) |

### Detail Views

| Key | Action |
|-----|--------|
| `e` | Edit resource |
| `t` | Toggle flag on/off (flags only) |
| `r` | Edit rollout percentage (flags only) |
| `u` | Edit targeting rules (flags only) |
| `v` | Edit variations (flags) / Edit value (configs) |
| `s` | View schedules (flags only) |

## Configuration

Config file: `~/.config/flagdash/config.toml`

```toml
[auth]
api_key = "management_xxx"

[connection]
base_url = "https://flagdash.io"

[defaults]
project_id = "prj_xxx"
environment_id = "env_xxx"
```

### Priority

CLI args > environment variables > config file

### Environment Variables

| Variable | Description |
|----------|-------------|
| `FLAGDASH_API_KEY` | Management API key |
| `FLAGDASH_BASE_URL` | API base URL (default: `https://flagdash.io`) |
| `FLAGDASH_PROJECT_ID` | Default project ID |
| `FLAGDASH_ENVIRONMENT_ID` | Default environment ID |

## API Key Tiers

| Prefix | Tier | Permissions |
|--------|------|-------------|
| `management_` | Management | Full CRUD on all resources |
| `server_` | Server | Read-only (mutations disabled in UI) |
| `client_` | Client | Read-only (mutations disabled in UI) |

## Development

```sh
# Build (debug)
cargo build

# Build (release, optimized)
cargo build --release

# Run
cargo run -- --api-key management_xxx

# Test
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --check
```

## Building & Releasing

Releases are automated via GitHub Actions. To publish a new version:

### 1. Bump the version

Edit `Cargo.toml`:

```toml
[package]
version = "0.2.0"  # bump this
```

### 2. Tag and push

```sh
git add sdk/flagdash-cli/Cargo.toml
git commit -m "cli: bump to 0.2.0"
git tag cli-v0.2.0
git push origin main --tags
```

The `cli-v*` tag triggers the `cli-release.yml` workflow, which:

1. **Builds** binaries for 6 targets (linux amd64/arm64, macOS amd64/arm64, Windows amd64/arm64)
2. **Creates a GitHub Release** with all binaries attached
3. **Publishes to crates.io** (`cargo publish`)
4. **Publishes npm wrapper** (`@flagdash/cli` on npmjs.com)
5. **Updates Homebrew formula** in `flagdash/homebrew-tap`

### Required Secrets

| Secret | Purpose |
|--------|---------|
| `CRATES_IO_TOKEN` | Publish to crates.io |
| `NPM_TOKEN` | Publish `@flagdash/cli` to npm |
| `SDK_SYNC_TOKEN` | Push to `flagdash/homebrew-tap` repo |

### Manual / Local Build

```sh
cd sdk/flagdash-cli

# Debug build
cargo build

# Release build (optimized, stripped)
cargo build --release

# The binary is at:
# target/release/flagdash        (macOS/Linux)
# target/release/flagdash.exe    (Windows)
```

### Cross-compilation

For cross-compiling to other platforms locally, use [cross](https://github.com/cross-rs/cross):

```sh
cargo install cross --git https://github.com/cross-rs/cross

# Linux ARM64 from macOS
cross build --release --target aarch64-unknown-linux-gnu

# Linux AMD64 from macOS
cross build --release --target x86_64-unknown-linux-gnu
```

### Distribution Channels

| Channel | How it works |
|---------|-------------|
| **Homebrew** | Formula in `flagdash/homebrew-tap` repo, auto-updated by CI on release |
| **npm** | Wrapper package `@flagdash/cli` runs a postinstall script that downloads the binary |
| **crates.io** | Standard `cargo publish`, users install with `cargo install flagdash-cli` |
| **Shell script** | `install.sh` detects OS/arch, downloads the binary from GitHub Releases |
| **GitHub Releases** | Pre-built binaries for all platforms attached to each release |

### Setting Up Homebrew Tap (First Time)

1. Create a repo `flagdash/homebrew-tap` on GitHub
2. The CI workflow pushes the formula to `Formula/flagdash.rb` in that repo
3. Users install with `brew install flagdash/tap/flagdash`

### Setting Up npm (First Time)

1. Create the `@flagdash` org on npmjs.com
2. Generate an automation token, save as `NPM_TOKEN` secret
3. The wrapper package at `sdk/flagdash-cli/dist/npm/` is published by CI

### Setting Up crates.io (First Time)

1. Create an account on [crates.io](https://crates.io)
2. Generate an API token, save as `CRATES_IO_TOKEN` secret
3. Ensure the `flagdash-cli` crate name is available (or you own it)

## License

MIT
