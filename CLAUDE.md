# CLAUDE.md

This file provides guidance to Claude Code when working with the OpenWeather WASM FDW wrapper.

## Project Overview

**open-weather-fdw** is a WebAssembly (WASM) Foreign Data Wrapper for PostgreSQL that enables querying the OpenWeather One Call API 3.0 (https://openweathermap.org/api/one-call-3) as if it were a native PostgreSQL table.

This wrapper follows the WASM FDW architecture required for hosted Supabase instances and can be used with any Supabase project.

## Project Status

**✅ v0.2.0 - Production Ready**

- **Current Version:** v0.2.0 (all 8 endpoints implemented and tested)
- **Repository Initialized:** October 24, 2025
- **Implementation Completed:** October 24, 2025
- **Endpoints:** 8 of 8 implemented (100% complete)
- **Binary Size:** 143 KB (under 150 KB target)
- **Reference Implementation:** [supabase-fdw-energy-charts](https://github.com/powabase/supabase-fdw-energy-charts)
- **Backend Integration:** [powabase-backend](https://github.com/powabase/powabase-backend)

## Technology Stack

- **Language:** Rust 1.90.0+
- **Target:** wasm32-unknown-unknown (WebAssembly - NO wasip1!)
- **Framework:** Supabase Wrappers v2 API
- **Build Tool:** cargo-component 0.21.1
- **API:** OpenWeather One Call API 3.0
- **Deployment:** GitHub releases with WASM binaries

## Implemented Endpoints (v0.2.0)

All 8 endpoints from OpenWeather One Call API 3.0 are now implemented:

| Endpoint | API Path | Parameters | Rows | Columns | Status |
|----------|----------|------------|------|---------|--------|
| **current_weather** | /onecall | lat, lon, units?, lang? | 1 | 18 | ✅ v0.1.0 |
| **minutely_forecast** | /onecall | lat, lon, units?, lang? | 60 | 4 | ✅ v0.1.0 |
| **hourly_forecast** | /onecall | lat, lon, units?, lang? | 48 | 19 | ✅ v0.1.0 |
| **daily_forecast** | /onecall | lat, lon, units?, lang? | 8 | 32 | ✅ v0.1.0 |
| **weather_alerts** | /onecall | lat, lon, units?, lang? | 0-N | 8 | ✅ v0.1.0 |
| **historical_weather** | /onecall/timemachine | lat, lon, dt, units?, lang? | 1 | 15 | ✅ v0.1.0 |
| **daily_summary** | /onecall/day_summary | lat, lon, date, tz?, units?, lang? | 1 | 17 | ✅ v0.2.0 |
| **weather_overview** | /onecall/overview | lat, lon, date?, units?, lang? | 1 | 6 | ✅ v0.2.0 |

**Total:** 101 columns across 8 foreign tables

## API Reference

**Backend Configuration:**
- See: `/Users/cf/Documents/GitHub/powabase/powabase-backend/supabase/seed/14_openweather_api.sql`
- This file contains complete parameter schemas, validation rules, and examples

**OpenWeather Documentation:**
- One Call API 3.0: https://openweathermap.org/api/one-call-3
- API Key: Required (free tier: 1,000 calls/day)
- Base URL: https://api.openweathermap.org/data/3.0

## Quick Reference

### Build Commands

```bash
# Development build
cargo component build --target wasm32-unknown-unknown

# Production build (optimized for size)
# ⚠️ CRITICAL: Must use wasm32-unknown-unknown (NOT wasm32-wasip1)
cargo component build --release --target wasm32-unknown-unknown

# Verify output
ls -lh target/wasm32-unknown-unknown/release/*.wasm
# Target: < 150 KB

# Calculate checksum for deployment
shasum -a 256 target/wasm32-unknown-unknown/release/open_weather_fdw.wasm
```

### Local Testing

```bash
# Start local Supabase (in your Supabase project directory)
supabase start

# Serve WASM via HTTP (recommended for testing)
cd target/wasm32-unknown-unknown/release
python3 -m http.server 8000 &

# Test via SQL (use host.docker.internal for Docker containers)
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres
```

**Important Docker Networking:**
- ❌ `localhost:8000` won't work (container's localhost ≠ host machine)
- ❌ `file:///path/to/file.wasm` gives misleading errors
- ✅ `http://host.docker.internal:8000/open_weather_fdw.wasm` works on Docker Desktop

## Key Architecture Decisions

### Why WASM FDW?

Hosted Supabase instances **cannot install native PostgreSQL extensions**. WASM FDW is the only way to create custom foreign data wrappers. The WASM binary is:

1. Built locally or in CI/CD
2. Published to GitHub releases
3. Referenced by URL in Supabase SQL
4. Downloaded and cached on first query execution

### Critical Implementation Patterns (from energy-charts experience)

#### 1. Build Target (Most Common Error!)

**✅ ALWAYS use wasm32-unknown-unknown:**
```bash
cargo component build --release --target wasm32-unknown-unknown
```

**❌ NEVER use wasm32-wasip1:**
- Adds WASI CLI interfaces (stdin/stdout/env)
- Supabase doesn't provide these interfaces
- Causes: `component imports instance 'wasi:cli/environment@0.2.0'`

**Verify:**
```bash
wasm-tools component wit target/wasm32-unknown-unknown/release/open_weather_fdw.wasm | grep wasi:cli
# Expected: (no output - zero WASI CLI imports)
```

#### 2. Use .get() Instead of [] (Prevents Panics!)

**✅ Safe:**
```rust
let value = match json_obj.get("field") {
    Some(v) => v,
    None => return Ok(None),
};
```

**❌ Panics if key missing:**
```rust
let value = json_obj["field"];  // Don't do this!
```

#### 3. API Parameter Extraction

OpenWeather requires specific parameters for each endpoint. Extract from WHERE clause:

```rust
// Example: Extract lat/lon from WHERE clause
let quals = ctx.get_quals();
let lat = extract_numeric_qual(&quals, "lat")?;
let lon = extract_numeric_qual(&quals, "lon")?;

// Build URL with parameters
let url = format!(
    "{}/onecall?lat={}&lon={}&appid={}",
    base_url, lat, lon, api_key
);
```

## Top 3 Pitfalls to Avoid (from energy-charts)

### 1. Wrong Build Target (wasip1)
- **Symptom:** `component imports instance wasi:cli/environment@0.2.0`
- **Cause:** Built with `wasm32-wasip1` instead of `wasm32-unknown-unknown`
- **Solution:** Always use `wasm32-unknown-unknown` target
- **Prevention:** Verify with `wasm-tools component wit` before releasing

### 2. Using [] Instead of .get()
- **Symptom:** Panic at runtime, wrapper crashes
- **Cause:** Accessing JSON with `[]` when key doesn't exist
- **Solution:** Always use `.get()` for safe access
- **Prevention:** Code review for any `[]` usage on dynamic data

### 3. Local Testing with file:// URLs
- **Symptom:** Misleading "invalid WebAssembly component" error
- **Cause:** Supabase Docker can't access host filesystem
- **Solution:** Use HTTP server with `host.docker.internal`
- **Prevention:** Always test with HTTP URLs locally

## Implementation Guidance

### Phase 1: Start with One Endpoint

Recommend implementing **current_and_forecasts** first:
- Most commonly used endpoint
- Clear parameter schema (lat, lon required)
- Well-documented response format
- Can test immediately

### Phase 2: Response Schema Design

OpenWeather returns nested JSON. Need to flatten to SQL rows:

**Options:**
1. **Single table with all fields** - Simple but potentially wide
2. **Separate tables per data type** - current, hourly, daily as separate foreign tables
3. **Flattened arrays** - Like energy-charts does with time series

**Recommendation:** Start simple (option 1), refactor if needed.

### Phase 3: Parameter Handling

Map WHERE clause to API parameters:
- Required: lat, lon (always)
- Optional: units (standard/metric/imperial), lang, exclude
- Endpoint-specific: dt (historical), date (daily), etc.

### Phase 4: Testing Strategy

1. Build stub (current status) ✅
2. Implement one endpoint (current_and_forecasts)
3. Test locally with real API key
4. Add error handling
5. Optimize response parsing
6. Document schema
7. Repeat for other endpoints

## Documentation Map

### Initialization (Complete)
- **[README.md](README.md)** - Project overview
- **[QUICKSTART.md](QUICKSTART.md)** - Setup guide template
- **This File (CLAUDE.md)** - AI assistant instructions

### To Be Created During Implementation
- **Deployment Guide** - docs/guides/DEPLOYMENT_GUIDE.md
- **Troubleshooting Guide** - docs/guides/TROUBLESHOOTING.md
- **SQL Examples** - docs/reference/SQL_EXAMPLES.md
- **API Overview** - docs/reference/API_OVERVIEW.md
- **Endpoint Docs** - docs/endpoints/*.md (one per endpoint)

## Example SQL Setup (Template)

```sql
-- Create server with GitHub release URL
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.1.0',
    fdw_package_checksum 'TBD_AFTER_BUILD',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'
  );

-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;

-- Create foreign table (schema TBD during implementation)
CREATE FOREIGN TABLE fdw_open_weather.current_weather (
  -- TODO: Define schema based on OpenWeather response
  lat numeric,
  lon numeric
  -- ... more columns
)
SERVER openweather_server
OPTIONS (object 'current_and_forecasts');

-- Test query
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405
LIMIT 5;
```

## Reference Implementation

**Energy Charts FDW:**
- Repository: [supabase-fdw-energy-charts](https://github.com/powabase/supabase-fdw-energy-charts)
- Local path: `/Users/cf/Documents/GitHub/powabase/powabase-fdw-energy-charts`
- Status: Production-ready (v0.5.0)
- Use as reference for:
  - Project structure
  - Build configuration
  - Error handling patterns
  - Documentation style
  - GitHub workflows

## When Working on This Project

### Adding Endpoint Implementation

1. Update `EndpointData` enum in `src/lib.rs` with data structure
2. Implement response parsing in `begin_scan()`
3. Implement row iteration in `iter_scan()`
4. Test locally with HTTP server
5. Document in `docs/endpoints/[name].md`
6. Update README.md features
7. Update QUICKSTART.md with schema

### Debugging Issues

1. **Check build target first** (wasm32-unknown-unknown?)
2. **Verify WASI imports** (should be zero)
3. **Test with HTTP URL** (not file://)
4. **Check JSON parsing** (use .get(), not [])
5. **Verify API key** (check OpenWeather dashboard)
6. **Review logs** in Supabase dashboard

### Before Releasing v0.1.0

- [ ] At least one endpoint fully implemented
- [ ] Version updated in Cargo.toml, wit/world.wit, CLAUDE.md
- [ ] Built with `--release --target wasm32-unknown-unknown`
- [ ] Verified zero WASI CLI imports
- [ ] Binary size < 150 KB
- [ ] Tested locally with all implemented endpoints
- [ ] Documentation updated (endpoint docs, README, QUICKSTART)
- [ ] SHA256 checksum calculated
- [ ] GitHub release created with notes

## Support

- **Issues:** GitHub Issues
- **OpenWeather API:** https://openweathermap.org/api/one-call-3
- **Supabase WASM FDW:** https://fdw.dev
- **Reference:** Energy Charts FDW source code
- **Backend Integration:** powabase-backend repository

## Critical Implementation Notes

**Key JSON Parsing Patterns:**
- Weather data is returned as **array** - must extract `weather[0]`
- Daily temp/feels_like are **nested objects** `{day, min, max, ...}`
- Rain/snow are **conditional nested objects** `{1h: value}`
- Historical uses **data[0]** array, not flat response

**Best Practices from Energy Charts:**
- ALWAYS use `.get()` instead of `[]` for JSON access (prevents panics)
- Build with `--target wasm32-unknown-unknown` (NOT wasip1!)
- Test with HTTP server (not file://) for Docker compatibility
- Handle NULL values gracefully
- Validate parameters before API call
