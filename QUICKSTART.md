# Quick Start Guide

Get OpenWeather data into PostgreSQL in 3 minutes.

## Prerequisites

- Supabase project (local or hosted)
- OpenWeather API key ([free tier available](https://openweathermap.org/api/one-call-3))

## Installation

### Step 1: Create Foreign Server

```sql
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.3.1/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.1',
    fdw_package_checksum 'see README.md for current checksum',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key_here'
  );
```

> **Note:** Find the current checksum in [README.md](README.md#release-information)

### Step 2: Import Schema

```sql
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;
IMPORT FOREIGN SCHEMA public
  FROM SERVER openweather_server
  INTO fdw_open_weather;
```

### Step 3: Query Weather Data

```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;
```

## Next Steps

- 📖 [Full documentation](README.md)
- 📊 [All endpoints](docs/endpoints/) (8 endpoints with schemas)
- 💡 [SQL examples](docs/reference/SQL_EXAMPLES.md)
- 🚀 [Deployment guide](docs/guides/DEPLOYMENT_GUIDE.md)
- ❓ [Troubleshooting](docs/guides/TROUBLESHOOTING.md)
