# GitHub Actions Workflows

This directory contains automated CI/CD workflows for the OpenWeather WASM FDW project, implementing 2025 best practices for Rust WebAssembly releases.

## Workflows

### 1. `release.yml` - Production Release Automation

**Trigger:** Push tags matching `v*` (e.g., `v0.2.0`)

**Purpose:** Automatically builds, validates, and releases WASM binaries to GitHub Releases

**Steps:**
1. ✅ Install Rust toolchain with `wasm32-unknown-unknown` target
2. ✅ Cache Cargo dependencies (saves 2-5 minutes on subsequent runs)
3. ✅ Install `cargo-component` via pre-built binary (fast)
4. ✅ Build optimized WASM binary
5. ✅ **Validate WASM structure** (prevents WASI import bugs)
6. ✅ Check binary size (warns if > 160 KB)
7. ✅ Calculate SHA256 checksum
8. ✅ Create GitHub Release with comprehensive notes
9. ✅ Upload 3 files: `.wasm`, `.sha256`, `checksums.txt`

**Build Time:**
- First run: ~5-7 minutes
- Cached runs: ~1-2 minutes ⚡

**Usage:**
```bash
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

---

### 2. `test.yml` - Continuous Integration

**Trigger:** Push to `main` or `develop`, Pull Requests

**Purpose:** Validate code quality and WASM builds before merging

**Jobs:**

#### Lint & Format
- Runs `cargo fmt --check` (formatting)
- Runs `cargo clippy` (linting)
- Fails on warnings

#### Build & Validate
- Builds WASM in debug and release modes
- Validates WASM structure
- Checks for WASI CLI imports (must be zero)
- Verifies binary size < 500 KB
- Uploads WASM artifact for inspection

**Build Time:**
- First run: ~3-4 minutes
- Cached runs: ~30-60 seconds

---

## Key Features

### 2025 Best Practices

**Modern Actions:**
- `dtolnay/rust-toolchain@stable` - Maintained Rust toolchain (replaces deprecated `actions-rs/toolchain`)
- `Swatinem/rust-cache@v2` - Intelligent Cargo caching
- `taiki-e/install-action@v2` - Fast binary installations
- `softprops/action-gh-release@v2` - Latest release action

**Security & Validation:**
- Automated WASI import detection (prevents #1 deployment bug)
- Binary size monitoring
- SHA256 checksum generation
- Release asset integrity

**Performance:**
- Intelligent caching (Cargo registry, git deps, build artifacts)
- Pre-built binaries for tools (cargo-component)
- Parallel job execution where possible

---

## WASM Validation

### Critical Check: Zero WASI Imports

The workflow **automatically validates** that the WASM binary has **zero WASI CLI imports**. This prevents the most common deployment issue:

```bash
# What the workflow checks
wasm-tools component wit open_weather_fdw.wasm | grep wasi:cli
# Expected: (no output)
```

**Why this matters:**
- Supabase Wrappers doesn't provide WASI CLI interfaces
- Using `wasm32-wasip1` target causes this error:
  ```
  component imports instance 'wasi:cli/environment@0.2.0',
  but a matching implementation was not found
  ```
- Workflow FAILS if any WASI CLI imports detected
- Saves hours of debugging deployment issues

### Expected WASM Imports

The binary should ONLY import Supabase Wrappers interfaces:

```wit
world root {
  import supabase:wrappers/http@0.2.0;
  import supabase:wrappers/stats@0.2.0;
  import supabase:wrappers/utils@0.2.0;
  import supabase:wrappers/types@0.2.0;
  export supabase:wrappers/routines@0.2.0;
}
```

---

## Release Process

### Standard Release

```bash
# 1. Ensure all changes committed
git add .
git commit -m "Release v0.2.0: Description"
git push origin main

# 2. Create and push tag
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0

# 3. Monitor workflow
# Visit: https://github.com/powabase/supabase-fdw-open-weather/actions
```

### Pre-Release Testing (Recommended)

Test with a release candidate first:

```bash
# 1. Create RC tag
git tag -a v0.2.0-rc1 -m "Release candidate v0.2.0-rc1"
git push origin v0.2.0-rc1

# 2. Workflow runs, creates pre-release
# 3. Test the binary in Supabase

# 4. If successful, create final release
git tag -d v0.2.0-rc1                  # Delete local
git push origin :refs/tags/v0.2.0-rc1  # Delete remote
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

---

## Release Notes Format

The workflow auto-generates comprehensive release notes:

```markdown
## OpenWeather WASM FDW v0.2.0

**Binary Size:** 143 KB
**SHA256:** `aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee`
**Target:** `wasm32-unknown-unknown` (bare WASM, no WASI)
**Supabase Wrappers:** Compatible with v0.5.3+

### Endpoints Included
1. current_weather - Real-time conditions (18 columns)
2. minutely_forecast - 60-minute precipitation (4 columns)
3. hourly_forecast - 48-hour forecast (19 columns)
4. daily_forecast - 8-day forecast (32 columns)
5. weather_alerts - Active warnings (8 columns)
6. historical_weather - Historical data (15 columns)
7. daily_summary - Daily aggregates (17 columns)
8. weather_overview - AI summaries (6 columns)

### Installation (Supabase)
[SQL code for deployment]

### Quick Test
[Example queries]
```

---

## Troubleshooting

### Workflow Fails: WASI Import Detected

**Error:**
```
❌ ERROR: Found 5 WASI CLI imports!
This means the build uses wasm32-wasip1 instead of wasm32-unknown-unknown
```

**Fix:**
- Verify `Cargo.toml` doesn't specify wasip1 target
- Check that `cargo component build` uses `--target wasm32-unknown-unknown`
- Rebuild locally and test: `wasm-tools component wit target/wasm32-unknown-unknown/release/open_weather_fdw.wasm | grep wasi:cli`

### Workflow Slow

**Expected:**
- First run: 5-7 minutes
- Cached runs: 1-2 minutes

**If slower:**
- Check if cache is working: Look for "Cache restored" in logs
- Verify `Swatinem/rust-cache@v2` step succeeds
- Consider cache key conflicts (cleared automatically)

### Binary Too Large

**Warning:**
```
⚠️ WARNING: Binary size (250 KB) is larger than expected
```

**Check:**
- Verify `Cargo.toml` has `[profile.release]` optimizations:
  ```toml
  [profile.release]
  opt-level = "z"       # Size optimization
  lto = true           # Link-time optimization
  strip = "debuginfo"  # Strip debug info
  codegen-units = 1    # Better optimization
  ```

---

## Caching Strategy

The workflows use `Swatinem/rust-cache@v2` which caches:

1. **Cargo registry** (`~/.cargo/registry`)
2. **Git dependencies** (`~/.cargo/git`)
3. **Build artifacts** (`target/`)

**Cache keys:**
- `wasm-release` - Release workflow cache
- `wasm-test` - Test workflow cache

**Benefits:**
- 2-5 minutes saved per run
- Shared across workflow runs
- Automatically cleaned if stale

---

## Permissions

Both workflows require:

```yaml
permissions:
  contents: write  # Create releases, upload assets
```

**Repository Settings:**
- Settings → Actions → General → Workflow permissions
- Select: "Read and write permissions" ✅

---

## Reference

- **GitHub Actions Documentation:** https://docs.github.com/en/actions
- **Rust Toolchain Action:** https://github.com/dtolnay/rust-toolchain
- **Rust Cache Action:** https://github.com/Swatinem/rust-cache
- **Install Action:** https://github.com/taiki-e/install-action
- **Release Action:** https://github.com/softprops/action-gh-release
- **wasm-tools:** https://github.com/bytecodealliance/wasm-tools
- **cargo-component:** https://github.com/bytecodealliance/cargo-component

---

**Last Updated:** October 24, 2025
**Workflow Version:** v0.2.0
**Maintainer:** Christian Fuerst
