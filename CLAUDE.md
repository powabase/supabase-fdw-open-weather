# CLAUDE.md

This file provides guidance to Claude Code when working with the OpenWeather WASM FDW wrapper.

## Project Overview

**open-weather-fdw** is a WebAssembly (WASM) Foreign Data Wrapper for PostgreSQL that enables querying the OpenWeather One Call API 3.0 as native PostgreSQL tables.

This wrapper follows the WASM FDW architecture required for hosted Supabase instances.

## Project Status

**✅ v0.3.3 - Production Ready**

- **Current Version:** v0.3.3 (improved error messages + bug fixes)
- **Endpoints:** 8 of 8 implemented (100% complete)
- **Binary Size:** 148 KB (under 150 KB target)
- **Standards Compliance:** 100%

## Technology Stack

- **Language:** Rust 1.90.0+
- **Target:** wasm32-unknown-unknown (WebAssembly)
- **Framework:** Supabase Wrappers v2 API
- **Build Tool:** cargo-component 0.21.1
- **API:** OpenWeather One Call API 3.0

## Implemented Endpoints

| Endpoint | Parameters | Rows | Columns |
|----------|------------|------|---------|
| current_weather | lat, lon, units?, lang? | 1 | 18 |
| minutely_forecast | lat, lon, units?, lang? | 60 | 4 |
| hourly_forecast | lat, lon, units?, lang? | 48 | 19 |
| daily_forecast | lat, lon, units?, lang? | 8 | 32 |
| weather_alerts | lat, lon, units?, lang? | 0-N | 8 |
| historical_weather | lat, lon, observation_time, units?, lang? | 1 | 15 |
| daily_summary | lat, lon, summary_date, tz_offset?, units?, lang? | 1 | 17 |
| weather_overview | lat, lon, overview_date?, units?, lang? | 1 | 6 |

**Total:** 101 columns across 8 foreign tables

## Quick Reference

### Build Commands

```bash
# Production build (CRITICAL: must use wasm32-unknown-unknown, NOT wasip1)
cargo component build --release --target wasm32-unknown-unknown

# Verify binary
ls -lh target/wasm32-unknown-unknown/release/*.wasm  # Should be < 150 KB
wasm-tools component wit target/wasm32-unknown-unknown/release/open_weather_fdw.wasm | grep wasi:cli  # Should be empty

# Calculate checksum
shasum -a 256 target/wasm32-unknown-unknown/release/open_weather_fdw.wasm
```

### Local Testing

```bash
# Serve WASM via HTTP (for Docker compatibility)
cd target/wasm32-unknown-unknown/release && python3 -m http.server 8000 &

# Connect to local Supabase
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres
```

**Docker Networking:**
- ✅ Use `http://host.docker.internal:8000/open_weather_fdw.wasm`
- ❌ Don't use `localhost:8000` or `file://` paths

## Critical Implementation Patterns

### 1. Build Target (Most Common Error!)

**✅ ALWAYS:** `--target wasm32-unknown-unknown`
**❌ NEVER:** `--target wasm32-wasip1` (adds WASI CLI interfaces that Supabase doesn't support)

### 2. JSON Parsing Safety

**✅ Use `.get()` for safe access:**
```rust
let value = json_obj.get("field").ok_or("missing field")?;
```

**❌ Never use `[]`** - will panic if key is missing

### 3. Optional Fields

Some API responses have optional fields (e.g., `visibility` in historical weather). Use `.unwrap_or()`:
```rust
let visibility = obj.get("visibility").and_then(|v| v.as_i64()).unwrap_or(10000);
```

### 4. Date/Time Parameters

**Important:** `historical_weather.observation_time` requires **literal timestamps** in WHERE clauses:
- ✅ Works: `WHERE observation_time = '2024-10-23 00:00:00+00'`
- ❌ Doesn't work: `WHERE observation_time = NOW() - INTERVAL '7 days'`

This is a WASM FDW architecture limitation - expressions aren't evaluated before parameter extraction.

## Top 3 Pitfalls

### 1. Wrong Build Target
- **Symptom:** `component imports instance wasi:cli/environment@0.2.0`
- **Solution:** Always use `wasm32-unknown-unknown` target

### 2. Using [] Instead of .get()
- **Symptom:** Panic at runtime
- **Solution:** Always use `.get()` for JSON access

### 3. Local Testing with file:// URLs
- **Symptom:** Misleading "invalid WebAssembly component" error
- **Solution:** Use HTTP server with `host.docker.internal`

## Key JSON Parsing Patterns

- Weather data: Extract from **array** → `weather[0]`
- Daily temp/feels_like: **Nested objects** → `{day, min, max, ...}`
- Rain/snow: **Conditional nested objects** → `{1h: value}`
- Historical: Uses **data[0]** array, not flat response
- Optional fields: Use `.unwrap_or()` with sensible defaults

## API Reference

- **Documentation:** https://openweathermap.org/api/one-call-3
- **Base URL:** https://api.openweathermap.org/data/3.0
- **API Key:** Required (free tier: 1,000 calls/day)
- **Supabase WASM FDW:** https://fdw.dev

## Reference Implementation

**Energy Charts FDW:** https://github.com/powabase/supabase-fdw-energy-charts
Use as reference for project structure, error handling, and build configuration.

## Release Checklist (v0.3.3)

- [x] Improved error messages for date/time parameters
- [x] Fixed missing visibility field in historical weather
- [x] Updated all documentation
- [x] Version bumped in Cargo.toml, wit/world.wit, CLAUDE.md
- [x] Built with correct target (wasm32-unknown-unknown)
- [x] Verified zero WASI CLI imports
- [x] Binary size: 148 KB (< 150 KB)
- [x] Tested with literal timestamps
- [ ] SHA256 checksum: `4f2486b6e7cdddd8d23900f3868f12f7b356597d5152effb33f93f6b60266faf`
- [ ] GitHub release created
