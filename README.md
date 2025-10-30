# OpenWeather WASM FDW

[![Version](https://img.shields.io/badge/version-v0.3.1-blue)](https://github.com/powabase/supabase-fdw-open-weather/releases/tag/v0.3.1)

WebAssembly Foreign Data Wrapper for PostgreSQL enabling SQL queries against the OpenWeather One Call API 3.0.

## Overview

This wrapper allows you to query comprehensive weather data from [OpenWeather](https://openweathermap.org/api/one-call-3) using standard SQL:

```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;
```

A standalone WASM FDW that can be used with any Supabase project.

**🚀 Want to get started immediately?** See [QUICKSTART.md](QUICKSTART.md) for a 3-minute setup guide.

## Status

**✅ Production Ready - v0.3.1**

All 8 endpoints implemented with 100% database schema standards compliance. Ready for production deployment.

## Features

- ✅ **WASM-Based** - Works on hosted Supabase (no native extensions needed)
- ✅ **WHERE Clause Pushdown** - Efficient API parameter translation
- ✅ **8 Production Endpoints** - Complete One Call API 3.0 coverage
- ✅ **Current + Forecasts** - Real-time weather and multi-horizon forecasts
- ✅ **Historical Data** - 46+ years of weather history
- ✅ **Daily Aggregations** - Statistical weather summaries
- ✅ **AI Summaries** - Human-readable weather overviews
- ✅ **Optimized Binary** - 147 KB (under 150 KB target)
- ✅ **Fast Response** - Sub-2-second query execution
- ✅ **Standards Compliant** - 100% database schema design standards compliance

## Release Information

| Attribute | Value |
|-----------|-------|
| **Version** | v0.3.1 |
| **Release Date** | October 29, 2025 |
| **Binary Size** | 147 KB |
| **SHA256 Checksum** | `0abb03a28bce499c1fdeedd0b64c461b226a907c3bcfc6542eb6d36e951f9eee` |
| **Standards Compliance** | 100% (Database Schema Design Standards) |
| **WASI CLI Imports** | 0 |

See [CHANGELOG.md](CHANGELOG.md) for version history and [MIGRATION.md](MIGRATION.md) for upgrade guides.

## Endpoints

| Endpoint | API Path | Rows | Columns | Status |
|----------|----------|------|---------|--------|
| **current_weather** | /onecall | 1 | 18 | ✅ v0.1.0 |
| **minutely_forecast** | /onecall | 60 | 4 | ✅ v0.1.0 |
| **hourly_forecast** | /onecall | 48 | 19 | ✅ v0.1.0 |
| **daily_forecast** | /onecall | 8 | 32 | ✅ v0.1.0 |
| **weather_alerts** | /onecall | 0-N | 8 | ✅ v0.1.0 |
| **historical_weather** | /onecall/timemachine | 1 | 15 | ✅ v0.1.0 |
| **daily_summary** | /onecall/day_summary | 1 | 17 | ✅ v0.2.0 |
| **weather_overview** | /onecall/overview | 1 | 6 | ✅ v0.2.0 |

**Total:** 101 columns across 8 foreign tables

## Quick Examples

All endpoints use standard SQL with `latitude`/`longitude` parameters:

```sql
-- Current weather conditions
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;

-- Next hour minute-by-minute precipitation
SELECT * FROM fdw_open_weather.minutely_forecast
WHERE latitude = 52.52 AND longitude = 13.405;

-- 48-hour forecast
SELECT * FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405;

-- 8-day forecast
SELECT * FROM fdw_open_weather.daily_forecast
WHERE latitude = 52.52 AND longitude = 13.405;

-- Active weather alerts
SELECT * FROM fdw_open_weather.weather_alerts
WHERE latitude = 52.52 AND longitude = 13.405;

-- Historical weather (requires literal timestamp - functions not supported)
SELECT * FROM fdw_open_weather.historical_weather
WHERE latitude = 52.52 AND longitude = 13.405
  AND observation_time = '2024-10-23 00:00:00+00';

-- Daily aggregated statistics
SELECT * FROM fdw_open_weather.daily_summary
WHERE latitude = 52.52 AND longitude = 13.405
  AND summary_date = '2024-01-15';

-- AI-generated weather overview
SELECT * FROM fdw_open_weather.weather_overview
WHERE latitude = 52.52 AND longitude = 13.405;
```

See [SQL Examples](docs/reference/SQL_EXAMPLES.md) for advanced queries (joins, intervals, aggregations).

## Quick Start

**For Users:** See [QUICKSTART.md](QUICKSTART.md) for setup instructions.

**For Developers:** Building from source instructions below.

### Building from Source

**Prerequisites:**
- Rust (stable)
- cargo-component 0.21.1
- Supabase CLI ≥ 1.187.10
- OpenWeather API key ([Get one free](https://openweathermap.org/api))

**Build:**
```bash
git clone https://github.com/powabase/supabase-fdw-open-weather.git
cd supabase-fdw-open-weather
cargo component build --release --target wasm32-unknown-unknown
# Output: target/wasm32-unknown-unknown/release/open_weather_fdw.wasm
```

**Deploy:** See [QUICKSTART.md](QUICKSTART.md) for SQL setup.

```sql
-- Example SQL setup (using Vault for security - recommended)
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.3.2/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.2',
    fdw_package_checksum 'TBD_AFTER_BUILD',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key_id 'your_vault_secret_uuid_here'  -- Recommended: Use Vault (see Security section below)
    -- api_key 'your_api_key'  -- Deprecated: Plain text (backward compatible)
  );
```

## Security: Using Vault for API Keys (Recommended)

**🔒 Supabase Vault** provides secure secret storage for production environments. Instead of storing API keys in plain text, store them in Vault and reference them by ID.

### Setup Vault Secret

```sql
-- 1. Insert your OpenWeather API key into Vault (do this once)
INSERT INTO vault.secrets (name, secret)
VALUES (
  'openweather_api_key',
  'your_actual_openweather_api_key_here'
)
RETURNING id;  -- Save this UUID for the next step
```

### Create Server with Vault Reference (Recommended)

```sql
-- 2. Create server using api_key_id (Vault UUID)
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.3.2/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.2',
    fdw_package_checksum 'TBD_AFTER_BUILD',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key_id '2d2c2f51-e4f7-4c89-a5e2-1f6e89f3dc4a'  -- Use your Vault UUID here
  );
```

### Plain Text API Key (Deprecated)

**⚠️ Deprecated:** Plain text `api_key` is still supported for backward compatibility but will show deprecation warnings:

```sql
-- Deprecated: Using plain text api_key (not recommended)
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.3.2/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.2',
    fdw_package_checksum 'TBD_AFTER_BUILD',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'  -- ⚠️ Deprecated: Migrate to api_key_id
  );
```

**Migration Guide:** See [VAULT_MIGRATION_GUIDE.md](VAULT_MIGRATION_GUIDE.md) for detailed migration steps.

**Documentation:** [Supabase Vault Guide](https://supabase.com/docs/guides/database/vault)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    SQL Query                             │
│  SELECT * FROM fdw_open_weather.current_weather           │
│  WHERE latitude = 52.52 AND longitude = 13.405          │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│              PostgreSQL / Supabase                       │
│         (Identifies foreign table)                       │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│            WASM FDW Wrapper (This Project)               │
│  1. Extracts WHERE: latitude=52.52, longitude=13.405    │
│  2. Builds API request with API key                     │
│  3. Executes HTTP GET to OpenWeather                    │
│  4. Parses JSON response                                │
│  5. Converts to PostgreSQL rows                         │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────┐
│           OpenWeather One Call API 3.0                   │
│  GET /onecall?lat=52.52&lon=13.405&appid=xxx            │
│  Returns: {current: {...}, hourly: [...], ...}         │
└─────────────────────────────────────────────────────────┘
```

## Why WASM?

Hosted Supabase instances cannot install native PostgreSQL extensions. WASM FDW enables custom foreign data wrappers through:

1. Dynamic loading from URL (GitHub releases)
2. Sandboxed execution (security)
3. No database restart required
4. Near-native performance

## Documentation

**Getting Started:**
- **[QUICKSTART.md](QUICKSTART.md)** - 3-minute setup guide ⭐
- **[Troubleshooting](docs/guides/TROUBLESHOOTING.md)** - Common issues and solutions

**Reference:**
- **[Endpoints](docs/endpoints/)** - All 8 endpoints with schemas and examples
- **[API Overview](docs/reference/API_OVERVIEW.md)** - OpenWeather API documentation

**Development:**
- **[CLAUDE.md](CLAUDE.md)** - AI assistant development guide
- **[Deployment Guide](docs/guides/DEPLOYMENT_GUIDE.md)** - Production deployment best practices

### Project Structure

```
supabase-fdw-open-weather/
├── src/
│   └── lib.rs                    # Main FDW implementation (stub)
├── wit/
│   └── world.wit                 # WASM interface definitions
├── .github/
│   └── workflows/
│       └── release.yml           # Automated build & release
├── Cargo.toml                    # Rust configuration
├── README.md                     # This file
└── docs/                         # Documentation (to be created)
```

### Key Implementation Files

- **src/lib.rs** - Core FDW logic (1,500+ lines, complete implementation)
- **wit/world.wit** - WebAssembly Interface Type (WIT) definitions
- **Cargo.toml** - Dependencies and build configuration
- **test_fdw.sql** - Complete test script for all 8 endpoints

## API Requirements

**OpenWeather API Key:**
- Free tier: 1,000 calls/day included, then pay-per-call
- Required for all endpoints
- Get your key: https://openweathermap.org/api/one-call-3

**Supported Plans:**
- One Call by Call (1,000 free calls/day)
- One Call API 3.0 subscription plans

## Contributing

Contributions are welcome! Please:

1. Read [CLAUDE.md](CLAUDE.md) for development guidelines
2. Test locally with Supabase CLI before creating PR
3. Update endpoint documentation for schema changes
4. Ensure WASM binary size stays < 150 KB
5. Verify zero WASI CLI imports (`wasm-tools component wit` should show none)
6. Follow Supabase v2 API patterns

## License

Apache 2.0 (matches Supabase Wrappers framework)

## Integration

This FDW works with any Supabase project. To integrate:

1. Deploy your Supabase project (local or hosted)
2. Upload the WASM binary to a public URL (GitHub Releases recommended)
3. Run the setup SQL (see [QUICKSTART.md](QUICKSTART.md))
4. Import the foreign schema into your database

For production deployment best practices, see the [Deployment Guide](docs/guides/DEPLOYMENT_GUIDE.md).

## Related Projects

- [Supabase Wrappers](https://github.com/supabase/wrappers) - WASM FDW framework
- [OpenWeather API](https://openweathermap.org/api) - Weather data provider
- [Energy Charts FDW](https://github.com/powabase/supabase-fdw-energy-charts) - Reference implementation

## Support

- **Documentation:** See `docs/` folder
- **Issues:** GitHub Issues
- **OpenWeather API:** https://openweathermap.org/api/one-call-3
- **Supabase WASM FDW:** https://supabase.com/blog/postgres-foreign-data-wrappers-with-wasm

For version history and changes, see [CHANGELOG.md](CHANGELOG.md).
