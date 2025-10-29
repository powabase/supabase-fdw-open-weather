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
    fdw_package_version 'v0.3.1',
    fdw_package_checksum '0abb03a28bce499c1fdeedd0b64c461b226a907c3bcfc6542eb6d36e951f9eee',  -- See README.md for latest version
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'  -- Get free key: https://openweathermap.org/api/one-call-3
  );

-- Step 4: Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;

-- Step 5: Create foreign tables for all 8 endpoints

-- Table 1: current_weather (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.current_weather (
  latitude numeric,
  longitude numeric,
  timezone_name text,
  observation_time timestamp with time zone,
  temperature_temp numeric,
  apparent_temperature_temp numeric,
  pressure_hpa bigint,
  humidity_pct bigint,
  dew_point_temp numeric,
  uv_index numeric,
  cloud_cover_pct bigint,
  visibility_m bigint,
  wind_speed_m_s numeric,
  wind_direction_deg bigint,
  wind_gust_speed_m_s numeric,
  weather_condition text,
  weather_description text,
  weather_icon_code text
)
SERVER openweather_server
OPTIONS (object 'current_weather');

-- Table 2: minutely_forecast (60 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.minutely_forecast (
  latitude numeric,
  longitude numeric,
  forecast_time timestamp with time zone,
  precipitation_mm numeric
)
SERVER openweather_server
OPTIONS (object 'minutely_forecast');

-- Table 3: hourly_forecast (48 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.hourly_forecast (
  latitude numeric,
  longitude numeric,
  forecast_time timestamp with time zone,
  temperature_temp numeric,
  apparent_temperature_temp numeric,
  pressure_hpa bigint,
  humidity_pct bigint,
  dew_point_temp numeric,
  uv_index numeric,
  cloud_cover_pct bigint,
  visibility_m bigint,
  wind_speed_m_s numeric,
  wind_direction_deg bigint,
  wind_gust_speed_m_s numeric,
  precipitation_probability numeric,
  rain_volume_1h_mm numeric,
  snow_volume_1h_mm numeric,
  weather_condition text,
  weather_description text,
  weather_icon_code text
)
SERVER openweather_server
OPTIONS (object 'hourly_forecast');

-- Table 4: daily_forecast (8 rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.daily_forecast (
  latitude numeric,
  longitude numeric,
  forecast_date timestamp with time zone,
  sunrise_time timestamp with time zone,
  sunset_time timestamp with time zone,
  moonrise_time timestamp with time zone,
  moonset_time timestamp with time zone,
  moon_phase_fraction numeric,
  temperature_day_temp numeric,
  temperature_min_temp numeric,
  temperature_max_temp numeric,
  temperature_night_temp numeric,
  temperature_eve_temp numeric,
  temperature_morn_temp numeric,
  apparent_temperature_day_temp numeric,
  apparent_temperature_night_temp numeric,
  apparent_temperature_eve_temp numeric,
  apparent_temperature_morn_temp numeric,
  pressure_hpa bigint,
  humidity_pct bigint,
  dew_point_temp numeric,
  wind_speed_m_s numeric,
  wind_direction_deg bigint,
  wind_gust_speed_m_s numeric,
  cloud_cover_pct bigint,
  precipitation_probability numeric,
  precipitation_rain_mm numeric,
  precipitation_snow_mm numeric,
  uv_index numeric,
  weather_condition text,
  weather_description text,
  weather_icon_code text
)
SERVER openweather_server
OPTIONS (object 'daily_forecast');

-- Table 5: weather_alerts (0-N rows)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.weather_alerts (
  latitude numeric,
  longitude numeric,
  alert_sender_name text,
  alert_event_type text,
  alert_start_time timestamp with time zone,
  alert_end_time timestamp with time zone,
  alert_description text,
  alert_tags text
)
SERVER openweather_server
OPTIONS (object 'weather_alerts');

-- Table 6: historical_weather (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.historical_weather (
  latitude numeric,
  longitude numeric,
  observation_time timestamp with time zone,
  temperature_temp numeric,
  apparent_temperature_temp numeric,
  pressure_hpa bigint,
  humidity_pct bigint,
  dew_point_temp numeric,
  cloud_cover_pct bigint,
  visibility_m bigint,
  wind_speed_m_s numeric,
  wind_direction_deg bigint,
  weather_condition text,
  weather_description text,
  weather_icon_code text
)
SERVER openweather_server
OPTIONS (object 'historical_weather');

-- Table 7: daily_summary (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.daily_summary (
  latitude numeric,
  longitude numeric,
  timezone_offset text,
  summary_date text,
  units text,
  temperature_min_temp numeric,
  temperature_max_temp numeric,
  temperature_morning_temp numeric,
  temperature_afternoon_temp numeric,
  temperature_evening_temp numeric,
  temperature_night_temp numeric,
  cloud_cover_afternoon_pct numeric,
  humidity_afternoon_pct numeric,
  pressure_afternoon_hpa numeric,
  precipitation_total_mm numeric,
  wind_max_speed_m_s numeric,
  wind_max_direction_deg numeric
)
SERVER openweather_server
OPTIONS (object 'daily_summary');

-- Table 8: weather_overview (1 row)
CREATE FOREIGN TABLE IF NOT EXISTS fdw_open_weather.weather_overview (
  latitude numeric,
  longitude numeric,
  timezone_offset text,
  overview_date text,
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
  timezone_name,
  observation_time,
  temperature_temp,
  weather_condition,
  weather_description
FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;

\echo ''
\echo '================================================='
\echo 'Testing minutely_forecast (up to 60 rows expected)'
\echo '================================================='
SELECT
  forecast_time,
  precipitation_mm
FROM fdw_open_weather.minutely_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_time
LIMIT 5;

\echo ''
\echo '=============================================='
\echo 'Testing hourly_forecast (48 rows expected)'
\echo '=============================================='
SELECT
  forecast_time,
  temperature_temp,
  weather_condition,
  precipitation_probability
FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_time
LIMIT 5;

\echo ''
\echo '============================================'
\echo 'Testing daily_forecast (8 rows expected)'
\echo '============================================'
SELECT
  forecast_date,
  temperature_min_temp,
  temperature_max_temp,
  weather_condition,
  precipitation_probability
FROM fdw_open_weather.daily_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_date
LIMIT 5;

\echo ''
\echo '================================================'
\echo 'Testing weather_alerts (0-N rows expected)'
\echo '================================================'
SELECT
  alert_event_type,
  alert_start_time,
  alert_end_time,
  alert_description
FROM fdw_open_weather.weather_alerts
WHERE latitude = 52.52 AND longitude = 13.405
LIMIT 5;

\echo ''
\echo '======================================================='
\echo 'Testing historical_weather (1 row expected)'
\echo 'Using timestamp: 2024-01-01 00:00:00 UTC'
\echo '======================================================='
SELECT
  observation_time,
  temperature_temp,
  weather_condition,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE latitude = 52.52 AND longitude = 13.405
  AND observation_time = '2024-01-01 00:00:00+00';

\echo ''
\echo '============================================'
\echo 'Testing daily_summary (1 row expected)'
\echo 'Using date: 2024-01-15'
\echo '============================================'
SELECT
  summary_date,
  temperature_min_temp,
  temperature_max_temp,
  temperature_afternoon_temp,
  precipitation_total_mm,
  wind_max_speed_m_s
FROM fdw_open_weather.daily_summary
WHERE latitude = 52.52 AND longitude = 13.405
  AND summary_date = '2024-01-15';

\echo ''
\echo '============================================'
\echo 'Testing weather_overview (1 row expected)'
\echo 'Using today or custom date'
\echo '============================================'
SELECT
  overview_date,
  LEFT(weather_overview, 100) || '...' as overview_preview
FROM fdw_open_weather.weather_overview
WHERE latitude = 52.52 AND longitude = 13.405;

\echo ''
\echo '========================================='
\echo 'All 8 endpoints tested!'
\echo '========================================='
