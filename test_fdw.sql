-- OpenWeather FDW Test Script
-- Run this in your Supabase database to test the FDW

-- Step 1: Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

-- Step 2: Create WASM FDW wrapper
CREATE FOREIGN DATA WRAPPER IF NOT EXISTS wasm_wrapper
  HANDLER wasm_fdw_handler
  VALIDATOR wasm_fdw_validator;

-- Step 3: Create OpenWeather server
CREATE SERVER IF NOT EXISTS openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://host.docker.internal:8000/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum 'dbb34d6f19b47e16c4373f793aeb8a7e33499a61f02d8a7656eb849fb3d340d7',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'  -- Get free key: https://openweathermap.org/api/one-call-3
  );

-- Step 4: Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;

-- Step 5: Create foreign tables for all 8 endpoints

-- Table 1: current_weather (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.current_weather (
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

-- Table 2: minutely_forecast (60 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.minutely_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  precipitation numeric
)
SERVER openweather_server
OPTIONS (object 'minutely_forecast');

-- Table 3: hourly_forecast (48 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.hourly_forecast (
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

-- Table 4: daily_forecast (8 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.daily_forecast (
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

-- Table 5: weather_alerts (0-N rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.weather_alerts (
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

-- Table 6: historical_weather (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.historical_weather (
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

-- Table 7: daily_summary (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.daily_summary (
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

-- Table 8: weather_overview (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.weather_overview (
  lat numeric,
  lon numeric,
  tz text,
  date text,
  units text,
  weather_overview text
)
SERVER openweather_server
OPTIONS (object 'weather_overview');

-- Step 6: Test queries
-- Note: Replace coordinates with your desired location
-- Example uses Berlin, Germany (52.52, 13.405)

\echo ''
\echo '========================================='
\echo 'Testing current_weather (1 row expected)'
\echo '========================================='
SELECT
  timezone,
  TO_TIMESTAMP(dt) as time,
  temp as temp_celsius,
  weather_main,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

\echo ''
\echo '================================================='
\echo 'Testing minutely_forecast (up to 60 rows expected)'
\echo '================================================='
SELECT
  TO_TIMESTAMP(dt) as time,
  precipitation
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 5;

\echo ''
\echo '=============================================='
\echo 'Testing hourly_forecast (48 rows expected)'
\echo '=============================================='
SELECT
  TO_TIMESTAMP(dt) as time,
  temp as temp_celsius,
  weather_main,
  pop as precipitation_probability
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 5;

\echo ''
\echo '============================================'
\echo 'Testing daily_forecast (8 rows expected)'
\echo '============================================'
SELECT
  TO_TIMESTAMP(dt) as date,
  temp_min,
  temp_max,
  weather_main,
  pop as precipitation_probability
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 5;

\echo ''
\echo '================================================'
\echo 'Testing weather_alerts (0-N rows expected)'
\echo '================================================'
SELECT
  event,
  TO_TIMESTAMP(start) as start_time,
  TO_TIMESTAMP("end") as end_time,
  description
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405
LIMIT 5;

\echo ''
\echo '======================================================='
\echo 'Testing historical_weather (1 row expected)'
\echo 'Using timestamp: 2024-01-01 00:00:00 UTC (1704067200)'
\echo '======================================================='
SELECT
  TO_TIMESTAMP(dt) as time,
  temp as temp_celsius,
  weather_main,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;

\echo ''
\echo '============================================'
\echo 'Testing daily_summary (1 row expected)'
\echo 'Using date: 2024-01-15'
\echo '============================================'
SELECT
  date,
  temp_min,
  temp_max,
  temp_afternoon,
  precipitation_total,
  wind_max_speed
FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-15';

\echo ''
\echo '============================================'
\echo 'Testing weather_overview (1 row expected)'
\echo 'Using today or custom date'
\echo '============================================'
SELECT
  date,
  LEFT(weather_overview, 100) || '...' as overview_preview
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

\echo ''
\echo '========================================='
\echo 'All 8 endpoints tested!'
\echo '========================================='
