# Endpoints

All 8 OpenWeather One Call API 3.0 endpoints are available:

| Endpoint | Description | Rows |
|----------|-------------|------|
| [current-weather](current-weather.md) | Real-time conditions | 1 |
| [minutely-forecast](minutely-forecast.md) | 60-minute precipitation | 60 |
| [hourly-forecast](hourly-forecast.md) | 48-hour forecast | 48 |
| [daily-forecast](daily-forecast.md) | 8-day forecast | 8 |
| [weather-alerts](weather-alerts.md) | Government alerts | 0-N |
| [historical-weather](historical-weather.md) | Historical data (1979+) | 1 |
| [daily-summary](daily-summary.md) | Daily aggregations | 1 |
| [weather-overview](weather-overview.md) | AI weather summary | 1 |

**Total:** 101 columns across 8 foreign tables

## Quick Start

See [QUICKSTART.md](../../QUICKSTART.md) for setup instructions.

## Standards Compliance

All endpoints use:
- `latitude`/`longitude` (not `lat`/`lon`)
- TIMESTAMPTZ for all temporal columns
- Explicit unit suffixes (`_celsius`, `_hpa`, `_pct`, etc.)
- Semantic names (`observation_time`, `summary_date`, etc.)

See [CHANGELOG.md](../../CHANGELOG.md) for version history.
