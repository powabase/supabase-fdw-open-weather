# Changelog

All notable changes to the OpenWeather WASM FDW will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [v0.3.1] - 2025-10-29

### Changed
- **BREAKING**: Parameter extraction now uses semantic columns instead of raw API parameters
  - `historical_weather`: Query with `observation_time` (TIMESTAMPTZ) instead of `dt` (BIGINT)
  - `daily_summary`: Query with `summary_date` (TEXT) instead of `date`
  - `weather_overview`: Query with `overview_date` (TEXT) instead of `date`

### Added
- Parameter transformation layer for seamless API abstraction
- Native TIMESTAMPTZ support enables PostgreSQL interval arithmetic
- Error messages now reference correct semantic column names

### Technical
- Binary size: 147 KB
- Checksum: `0abb03a28bce499c1fdeedd0b64c461b226a907c3bcfc6542eb6d36e951f9eee`
- Standards compliance: 100%

## [v0.3.0] - 2025-10-29

### Changed
- **BREAKING**: All column names updated for 100% database schema standards compliance
  - Geographic: `lat` → `latitude`, `lon` → `longitude`
  - Temporal: All Unix timestamps now TIMESTAMPTZ with semantic names
  - Units: Added explicit suffixes (`_hpa`, `_pct`, `_m_s`, etc.)
  - Abbreviations: Expanded (`uvi` → `uv_index`, `pop` → `precipitation_probability`)

### Added
- 78 column name improvements across 8 endpoints
- Native PostgreSQL temporal operations support

### Technical
- Binary size: 147 KB
- Checksum: `55e0226ad0880b25f2aac2e3028f3ce6987f33f91575afd6222537af3b5c8a31`

## [v0.2.0] - 2025-10-24

### Added
- `daily_summary` endpoint - Daily aggregated weather statistics
- `weather_overview` endpoint - AI-generated weather summaries
- Complete test coverage for all 8 endpoints

### Technical
- Binary size: 143 KB

## [v0.1.0] - 2025-10-24

### Added
- Initial release with 6 endpoints
- `current_weather`, `minutely_forecast`, `hourly_forecast`
- `daily_forecast`, `weather_alerts`, `historical_weather`
- Full WASM FDW implementation
- Zero WASI CLI imports

---

## Migration Guides

For detailed upgrade instructions, see [MIGRATION.md](MIGRATION.md).
