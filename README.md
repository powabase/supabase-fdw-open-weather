# OpenWeather WASM FDW

WebAssembly Foreign Data Wrapper for PostgreSQL enabling SQL queries against the OpenWeather One Call API 3.0.

## Overview

This wrapper allows you to query comprehensive weather data from [OpenWeather](https://openweathermap.org/api/one-call-3) using standard SQL:

```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

A standalone WASM FDW that can be used with any Supabase project.

**🚀 Want to get started immediately?** See [QUICKSTART.md](QUICKSTART.md) for a 3-minute setup guide.

## Status

**✅ Production Ready - v0.2.0**

All 8 endpoints implemented and tested! Ready for deployment.

## Features

- ✅ **WASM-Based** - Works on hosted Supabase (no native extensions needed)
- ✅ **WHERE Clause Pushdown** - Efficient API parameter translation
- ✅ **8 Production Endpoints** - Complete One Call API 3.0 coverage
- ✅ **Current + Forecasts** - Real-time weather and multi-horizon forecasts
- ✅ **Historical Data** - 46+ years of weather history
- ✅ **Daily Aggregations** - Statistical weather summaries
- ✅ **AI Summaries** - Human-readable weather overviews
- ✅ **Optimized Binary** - 143 KB (under 150 KB target)
- ✅ **Fast Response** - Sub-2-second query execution

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
-- Example SQL setup
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum '8411826e9bedd01f51b5a2c51e6b0ea2f0b20870c90ba9324e76583a2c709bd9',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'
  );
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    SQL Query                             │
│  SELECT * FROM fdw_open_weather.current_weather           │
│  WHERE lat = 52.52 AND lon = 13.405                     │
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
│  1. Extracts WHERE clause: lat = 52.52, lon = 13.405   │
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
- **[Backend Integration](https://github.com/powabase/powabase-backend)** - How this integrates with powabase

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

## Related Projects

- [Supabase Wrappers](https://github.com/supabase/wrappers) - WASM FDW framework
- [OpenWeather API](https://openweathermap.org/api) - Weather data provider
- [Powabase Backend](https://github.com/powabase/powabase-backend) - Integration target
- [Energy Charts FDW](https://github.com/powabase/supabase-fdw-energy-charts) - Reference implementation

## Support

- **Documentation:** See `docs/` folder
- **Issues:** GitHub Issues
- **OpenWeather API:** https://openweathermap.org/api/one-call-3
- **Supabase WASM FDW:** https://supabase.com/blog/postgres-foreign-data-wrappers-with-wasm

## Changelog

### v0.2.0 (October 24, 2025)
- ✅ Added `daily_summary` endpoint - Daily aggregated weather statistics
- ✅ Added `weather_overview` endpoint - AI-generated weather summaries
- ✅ Complete test coverage for all 8 endpoints
- ✅ Comprehensive documentation (82+ KB of endpoint docs)
- ✅ Binary size: 143 KB (optimized)

### v0.1.0 (October 24, 2025)
- ✅ Initial release with 6 core endpoints
- ✅ Current weather, forecasts (minutely/hourly/daily), alerts, historical data
- ✅ Full WASM FDW implementation
- ✅ Zero WASI CLI imports
- ✅ Production-ready binary
