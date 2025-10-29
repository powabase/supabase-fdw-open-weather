# OpenWeather One Call API 3.0 Overview

**Base URL:** `https://api.openweathermap.org/data/3.0`
**Authentication:** API key required
**Documentation:** https://openweathermap.org/api/one-call-3

## Endpoints

| FDW Table | API Path | Required Parameters | Rows |
|-----------|----------|---------------------|------|
| `current_weather` | `/onecall` | latitude, longitude | 1 |
| `minutely_forecast` | `/onecall` | latitude, longitude | 60 |
| `hourly_forecast` | `/onecall` | latitude, longitude | 48 |
| `daily_forecast` | `/onecall` | latitude, longitude | 8 |
| `weather_alerts` | `/onecall` | latitude, longitude | 0-N |
| `historical_weather` | `/onecall/timemachine` | latitude, longitude, observation_time | 1 |
| `daily_summary` | `/onecall/day_summary` | latitude, longitude, summary_date | 1 |
| `weather_overview` | `/onecall/overview` | latitude, longitude, overview_date | 1 |

## Optional Parameters

All endpoints support:
- `units` - standard (default), metric, imperial
- `lang` - Language code (e.g., en, de, fr)

## API Key

Get your free API key at: https://openweathermap.org/api/one-call-3

**Free tier:** 1,000 calls/day included

## More Information

- **Setup:** [QUICKSTART.md](../../QUICKSTART.md)
- **SQL Examples:** [SQL_EXAMPLES.md](SQL_EXAMPLES.md)
- **Endpoint Details:** [docs/endpoints/](../endpoints/)
