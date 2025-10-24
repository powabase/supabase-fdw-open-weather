# Quickstart Guide

Get OpenWeather data in your Supabase database in 3 minutes.

## Prerequisites

- Existing Supabase project (local or hosted)
- Supabase CLI installed (for local)
- OpenWeather API key ([Get one free](https://openweathermap.org/api/one-call-3))
- Basic SQL knowledge

## Step 1: Get Your OpenWeather API Key

1. Sign up at [OpenWeather](https://openweathermap.org/api)
2. Subscribe to "One Call by Call" (1,000 free calls/day)
3. Copy your API key

## Step 2: Create Foreign Server

Connect to your Supabase database and run:

```sql
-- Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

-- Create WASM FDW wrapper
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler
  VALIDATOR wasm_fdw_validator;

-- Create OpenWeather server
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum 'aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'  -- Replace with your actual API key
  );
```

## Step 3: Create Foreign Tables

Choose the weather data you need. Here's the current weather endpoint:

```sql
-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;

-- Create current_weather table
CREATE FOREIGN TABLE fdw_open_weather.current_weather (
  lat numeric,
  lon numeric,
  timezone text,
  dt bigint,
  temp numeric,
  feels_like numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  uvi numeric,
  clouds bigint,
  visibility bigint,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'current_weather');
```

## Step 4: Query Data

```sql
-- Get current weather for Berlin, Germany
SELECT
  timezone,
  TO_TIMESTAMP(dt) as time,
  temp as temp_celsius,
  weather_main,
  weather_description,
  humidity,
  wind_speed
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

**Expected result:** 1 row with current weather conditions.

## All Endpoints - Complete Setup

### 1. Current Weather (1 row)

Real-time weather conditions for any location.

```sql
CREATE FOREIGN TABLE fdw_open_weather.current_weather (
  lat numeric,
  lon numeric,
  timezone text,
  dt bigint,
  temp numeric,
  feels_like numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  uvi numeric,
  clouds bigint,
  visibility bigint,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'current_weather');

-- Example query
SELECT temp, humidity, weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 40.7128 AND lon = -74.0060;  -- New York
```

### 2. Minutely Forecast (60 rows)

Minute-by-minute precipitation forecast for the next hour.

```sql
CREATE FOREIGN TABLE fdw_open_weather.minutely_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  precipitation numeric
)
SERVER openweather_server
OPTIONS (object 'minutely_forecast');

-- Example query
SELECT
  TO_TIMESTAMP(dt) as time,
  precipitation
FROM fdw_open_weather.minutely_forecast
WHERE lat = 51.5074 AND lon = -0.1278  -- London
ORDER BY dt
LIMIT 10;
```

### 3. Hourly Forecast (48 rows)

Hour-by-hour weather forecast for 48 hours.

```sql
CREATE FOREIGN TABLE fdw_open_weather.hourly_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  temp numeric,
  feels_like numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  uvi numeric,
  clouds bigint,
  visibility bigint,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  pop numeric,
  rain_1h numeric,
  snow_1h numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'hourly_forecast');

-- Example query
SELECT
  TO_TIMESTAMP(dt) as time,
  temp,
  pop * 100 as precipitation_probability_pct,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 48.8566 AND lon = 2.3522  -- Paris
ORDER BY dt
LIMIT 12;  -- Next 12 hours
```

### 4. Daily Forecast (8 rows)

Daily weather forecast for 8 days.

```sql
CREATE FOREIGN TABLE fdw_open_weather.daily_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  sunrise bigint,
  sunset bigint,
  moonrise bigint,
  moonset bigint,
  moon_phase numeric,
  temp_day numeric,
  temp_min numeric,
  temp_max numeric,
  temp_night numeric,
  temp_eve numeric,
  temp_morn numeric,
  feels_like_day numeric,
  feels_like_night numeric,
  feels_like_eve numeric,
  feels_like_morn numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  clouds bigint,
  pop numeric,
  rain numeric,
  snow numeric,
  uvi numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'daily_forecast');

-- Example query
SELECT
  TO_TIMESTAMP(dt)::date as date,
  temp_min,
  temp_max,
  pop * 100 as rain_probability_pct,
  weather_description
FROM fdw_open_weather.daily_forecast
WHERE lat = 35.6762 AND lon = 139.6503  -- Tokyo
ORDER BY dt;
```

### 5. Weather Alerts (0-N rows)

National weather alerts from major national weather warning systems.

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_alerts (
  lat numeric,
  lon numeric,
  sender_name text,
  event text,
  start bigint,
  "end" bigint,
  description text,
  tags text
)
SERVER openweather_server
OPTIONS (object 'weather_alerts');

-- Example query
SELECT
  sender_name,
  event,
  TO_TIMESTAMP(start) as start_time,
  TO_TIMESTAMP("end") as end_time,
  description
FROM fdw_open_weather.weather_alerts
WHERE lat = 25.7617 AND lon = -80.1918  -- Miami
ORDER BY start DESC;
```

### 6. Historical Weather (1 row)

Historical weather data for any timestamp since Jan 1, 1979.

```sql
CREATE FOREIGN TABLE fdw_open_weather.historical_weather (
  lat numeric,
  lon numeric,
  dt bigint,
  temp numeric,
  feels_like numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  clouds bigint,
  visibility bigint,
  wind_speed numeric,
  wind_deg bigint,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'historical_weather');

-- Example query
SELECT
  TO_TIMESTAMP(dt) as time,
  temp,
  weather_main,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE lat = 55.7558 AND lon = 37.6173  -- Moscow
  AND dt = 1704067200;  -- Jan 1, 2024 00:00:00 UTC
```

### 7. Daily Summary (1 row)

Daily aggregated weather data. Available from Jan 2, 1979 + 1.5 years forecast.

```sql
CREATE FOREIGN TABLE fdw_open_weather.daily_summary (
  lat numeric,
  lon numeric,
  tz text,
  date text,
  units text,
  temp_min numeric,
  temp_max numeric,
  temp_morning numeric,
  temp_afternoon numeric,
  temp_evening numeric,
  temp_night numeric,
  cloud_cover_afternoon numeric,
  humidity_afternoon numeric,
  pressure_afternoon numeric,
  precipitation_total numeric,
  wind_max_speed numeric,
  wind_max_direction numeric
)
SERVER openweather_server
OPTIONS (object 'daily_summary');

-- Example query
SELECT
  date,
  temp_min,
  temp_max,
  precipitation_total,
  wind_max_speed
FROM fdw_open_weather.daily_summary
WHERE lat = 37.7749 AND lon = -122.4194  -- San Francisco
  AND date = '2024-01-15';
```

### 8. Weather Overview (1 row)

AI-generated human-readable weather summaries. Available for today and tomorrow only.

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_overview (
  lat numeric,
  lon numeric,
  tz text,
  date text,
  units text,
  weather_overview text
)
SERVER openweather_server
OPTIONS (object 'weather_overview');

-- Example query
SELECT
  date,
  weather_overview
FROM fdw_open_weather.weather_overview
WHERE lat = -33.8688 AND lon = 151.2093;  -- Sydney
```

## Complete Example - All Tables

```sql
-- Enable extension and create wrapper (if not done)
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler VALIDATOR wasm_fdw_validator;

-- Create server
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum 'aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key_here'
  );

-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;

-- Create all 8 foreign tables
-- (Use the CREATE FOREIGN TABLE statements from sections 1-8 above)
```

## Troubleshooting

### NULL values in results?

- Check WASM binary URL is accessible
- Verify checksum matches release
- Ensure API key is valid and has credits
- Verify coordinates are correct (lat: -90 to 90, lon: -180 to 180)

### Permission denied?

```sql
GRANT USAGE ON SCHEMA fdw_open_weather TO your_role;
GRANT SELECT ON ALL TABLES IN SCHEMA fdw_open_weather TO your_role;
```

### API rate limit errors?

- Free tier: 1,000 calls/day
- Check your usage at https://home.openweathermap.org/api_keys
- Consider upgrading plan if needed

### Invalid coordinates?

- Latitude must be between -90 and 90
- Longitude must be between -180 and 180
- Use decimal degrees format (e.g., 52.52, not 52°31'N)

### Error: "Parameter required: lat"?

The FDW requires `lat` and `lon` in the WHERE clause:

```sql
-- ❌ Wrong - missing WHERE clause
SELECT * FROM fdw_open_weather.current_weather;

-- ✅ Correct - includes lat/lon
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Historical data: "Parameter required: dt"?

The `historical_weather` endpoint requires a Unix timestamp:

```sql
-- ✅ Correct - includes dt parameter
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;
```

### Daily summary: "Parameter required: date"?

The `daily_summary` endpoint requires a date in YYYY-MM-DD format:

```sql
-- ✅ Correct - includes date parameter
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-15';
```

## Local Development

For local Supabase testing:

```bash
# Start Supabase
supabase start

# Connect to database
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres

# Run SQL from steps above
```

## Version Info

**Current Version:** v0.2.0 (Production Ready)
**Released:** October 24, 2025
**WASM Size:** 143 KB
**Endpoints:** 8 (complete One Call API 3.0 coverage)
**Supabase Wrappers:** v0.5.3+

## Next Steps

- **SQL Examples:** See [docs/reference/SQL_EXAMPLES.md](docs/reference/SQL_EXAMPLES.md)
- **Deployment Guide:** See [docs/guides/DEPLOYMENT_GUIDE.md](docs/guides/DEPLOYMENT_GUIDE.md)
- **Troubleshooting:** See [docs/guides/TROUBLESHOOTING.md](docs/guides/TROUBLESHOOTING.md)
- **Endpoint Details:** See [docs/endpoints/](docs/endpoints/)

## Popular Location Coordinates

| City | Latitude | Longitude |
|------|----------|-----------|
| New York | 40.7128 | -74.0060 |
| London | 51.5074 | -0.1278 |
| Paris | 48.8566 | 2.3522 |
| Tokyo | 35.6762 | 139.6503 |
| Berlin | 52.5200 | 13.4050 |
| Sydney | -33.8688 | 151.2093 |
| San Francisco | 37.7749 | -122.4194 |
| Miami | 25.7617 | -80.1918 |
| Moscow | 55.7558 | 37.6173 |

Need help? Check [issues](https://github.com/powabase/supabase-fdw-open-weather/issues) or see [full documentation](README.md).
