# Deployment Guide

Complete deployment guide for OpenWeather WASM FDW to Supabase.

## Prerequisites

- Supabase project (local or hosted)
- Supabase CLI ≥ 1.187.10
- OpenWeather API key ([Get one free](https://openweathermap.org/api/one-call-3))
- OpenWeather FDW release from GitHub

## Overview

**Deployment Steps:**
1. Get FDW release information
2. Obtain OpenWeather API key
3. Create Supabase migration
4. Test locally
5. Deploy to production
6. Verify installation

---

## Step 1: Get FDW Release Info

**Latest Release:** v0.2.0

Copy these values for your migration:
```
URL: https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm
SHA256: aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee
Version: 0.2.0
Binary Size: 143 KB
```

Verify the URL is accessible:
```bash
curl -I https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm
# Should return: HTTP 200
```

---

## Step 2: Get OpenWeather API Key

1. Sign up at https://openweathermap.org/api
2. Subscribe to "One Call by Call" subscription
   - **Free tier:** 1,000 calls/day included
   - **Pricing:** Pay-per-call after free tier
3. Navigate to API Keys page
4. Copy your API key (32-character hex string)
5. **IMPORTANT:** Store securely (never commit to git)

**Using Supabase Secrets:**
```bash
# Set API key as secret (recommended for production)
supabase secrets set OPENWEATHER_API_KEY=your_api_key_here
```

---

## Step 3: Create Migration

```bash
supabase migration new add_openweather_fdw
```

**Migration Template:**

```sql
-- Enable wrappers extension
CREATE EXTENSION IF NOT EXISTS wrappers WITH SCHEMA extensions;

-- Create WASM FDW wrapper
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_foreign_data_wrapper WHERE fdwname = 'wasm_wrapper') THEN
    CREATE FOREIGN DATA WRAPPER wasm_wrapper
      HANDLER wasm_fdw_handler VALIDATOR wasm_fdw_validator;
  END IF;
END $$;

-- Create foreign server
-- NOTE: Replace 'your_openweather_api_key_here' with your actual API key
CREATE SERVER IF NOT EXISTS openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum 'aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_openweather_api_key_here'
  );

-- Create schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;
GRANT USAGE ON SCHEMA fdw_open_weather TO postgres;

-- Create foreign table: current_weather
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

GRANT SELECT ON fdw_open_weather.current_weather TO postgres;

-- Create foreign table: hourly_forecast
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

GRANT SELECT ON fdw_open_weather.hourly_forecast TO postgres;

-- Create foreign table: daily_forecast
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

GRANT SELECT ON fdw_open_weather.daily_forecast TO postgres;

-- Grant on future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA fdw_open_weather
  GRANT SELECT ON TABLES TO postgres;
```

**Add more endpoints** as needed. See [QUICKSTART.md](../../QUICKSTART.md) for all CREATE TABLE statements.

**Available endpoints:**
- `current_weather` - Current conditions (1 row)
- `minutely_forecast` - 60-minute precipitation (60 rows)
- `hourly_forecast` - 48-hour forecast (48 rows)
- `daily_forecast` - 8-day forecast (8 rows)
- `weather_alerts` - Active alerts (0-N rows)
- `historical_weather` - Historical data (1 row)
- `daily_summary` - Daily aggregates (1 row)
- `weather_overview` - AI summaries (1 row)

---

## Step 4: Test Locally

```bash
# Reset database with migration
supabase db reset

# Connect to database
supabase db psql
```

**Verification queries:**

```sql
-- Test current_weather
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405
LIMIT 5;
-- Expected: 1 row with current weather

-- Test hourly_forecast
SELECT COUNT(*) FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405;
-- Expected: 48 rows

-- Test daily_forecast
SELECT COUNT(*) FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405;
-- Expected: 8 rows

-- Verify foreign server
SELECT * FROM pg_foreign_server WHERE srvname = 'openweather_server';

-- Verify all foreign tables
SELECT foreign_table_schema, foreign_table_name
FROM information_schema.foreign_tables
WHERE foreign_table_schema = 'fdw_open_weather';
```

---

## Step 5: Deploy to Production

### Option A: Supabase CLI (Recommended)

```bash
# Link to production project
supabase link --project-ref <your-project-ref>

# Push migration
supabase db push
```

### Option B: SQL Editor

1. Copy migration SQL
2. Open Supabase Dashboard → SQL Editor
3. **IMPORTANT:** Replace placeholder API key with your actual key
4. Paste and execute
5. Verify no errors

### Option C: Environment-Specific Configs

For organizations with multiple environments:

```bash
# Development
supabase link --project-ref dev-project-ref
supabase db push

# Staging
supabase link --project-ref staging-project-ref
supabase db push

# Production
supabase link --project-ref prod-project-ref
supabase db push
```

---

## Step 6: Verify Production

```sql
-- Quick health check
SELECT
  temp,
  humidity,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 40.7128 AND lon = -74.0060;  -- New York
-- Expected: 1 row with real-time weather

-- Check API key is working
SELECT COUNT(*) FROM fdw_open_weather.hourly_forecast
WHERE lat = 51.5074 AND lon = -0.1278;  -- London
-- Expected: 48 rows (no errors)

-- Verify historical access
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405
  AND dt = 1704067200;  -- Jan 1, 2024
-- Expected: 1 row with historical data
```

---

## Security Best Practices

### API Key Management

**❌ DON'T:**
- Commit API keys to git
- Share API keys in documentation
- Use same key across all environments
- Store keys in plaintext

**✅ DO:**
- Use Supabase Secrets for production
- Rotate keys regularly
- Monitor API usage
- Use separate keys per environment

**Using Vault/Secrets Manager:**
```sql
-- Production migration with vault reference
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    fdw_package_checksum 'aac5c2b1893b742cf1f919c8e7cae8d0ef8709964228ea368681c0755100c1ee',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key current_setting('app.openweather_api_key')  -- From vault
  );
```

### Access Control

Grant permissions based on needs:

```sql
-- Read-only access for analysts
GRANT USAGE ON SCHEMA fdw_open_weather TO analyst_role;
GRANT SELECT ON ALL TABLES IN SCHEMA fdw_open_weather TO analyst_role;

-- No access to modify server
REVOKE ALL ON FOREIGN SERVER openweather_server FROM PUBLIC;
```

---

## Monitoring & Maintenance

### API Usage Tracking

Monitor your API consumption:

```sql
-- Log queries (optional)
CREATE TABLE IF NOT EXISTS weather_query_log (
  id serial PRIMARY KEY,
  query_time timestamp DEFAULT now(),
  endpoint text,
  lat numeric,
  lon numeric,
  user_id text
);

-- Track in application layer
-- INSERT INTO weather_query_log (endpoint, lat, lon, user_id)
-- VALUES ('current_weather', 52.52, 13.405, 'user123');
```

Check usage at: https://home.openweathermap.org/api_keys

### Rate Limiting

Implement application-level rate limiting:

```sql
-- Rate limit example (using pg_cron)
CREATE OR REPLACE FUNCTION check_api_quota()
RETURNS boolean AS $$
DECLARE
  daily_count integer;
BEGIN
  SELECT COUNT(*) INTO daily_count
  FROM weather_query_log
  WHERE query_time > now() - interval '1 day';

  RETURN daily_count < 1000;  -- Free tier limit
END;
$$ LANGUAGE plpgsql;
```

### Health Checks

Add periodic health checks:

```sql
-- Simple availability check
SELECT 'OpenWeather FDW' as service,
       CASE
         WHEN COUNT(*) > 0 THEN 'healthy'
         ELSE 'unhealthy'
       END as status
FROM fdw_open_weather.current_weather
WHERE lat = 0 AND lon = 0;
```

---

## Upgrading

### To New Version

```bash
# 1. Download new release info
# From: https://github.com/powabase/supabase-fdw-open-weather/releases

# 2. Create upgrade migration
supabase migration new upgrade_openweather_fdw_v0_3_0

# 3. Update server options
```

**Upgrade migration template:**

```sql
-- Drop existing server (safe - doesn't drop data)
DROP SERVER IF EXISTS openweather_server CASCADE;

-- Recreate with new version
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.3.0/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.3.0',
    fdw_package_checksum '<new-checksum>',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key_here'
  );

-- Recreate foreign tables
-- (Use CREATE FOREIGN TABLE statements)
```

### Rollback Plan

```sql
-- If upgrade fails, rollback to previous version
DROP SERVER IF EXISTS openweather_server CASCADE;

-- Recreate with v0.2.0
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
```

---

## Troubleshooting Deployment

### Migration Fails

**Error:** `extension "wrappers" does not exist`
```sql
-- Solution: Enable wrappers extension first
CREATE EXTENSION wrappers WITH SCHEMA extensions;
```

**Error:** `foreign-data wrapper "wasm_wrapper" does not exist`
```sql
-- Solution: Create WASM wrapper first
CREATE FOREIGN DATA WRAPPER wasm_wrapper
  HANDLER wasm_fdw_handler
  VALIDATOR wasm_fdw_validator;
```

### NULL Results

**Cause:** Invalid API key or network issue

**Debug:**
```sql
-- Check server options
SELECT srvoptions FROM pg_foreign_server
WHERE srvname = 'openweather_server';

-- Verify API key format (should be 32 hex characters)
```

**Solution:** Update API key:
```sql
ALTER SERVER openweather_server
OPTIONS (SET api_key 'correct_api_key_here');
```

### Permission Issues

**Error:** `permission denied for schema fdw_open_weather`

**Solution:**
```sql
GRANT USAGE ON SCHEMA fdw_open_weather TO your_role;
GRANT SELECT ON ALL TABLES IN SCHEMA fdw_open_weather TO your_role;
```

### Binary Download Fails

**Error:** `could not download WASM binary`

**Cause:** Network restrictions or invalid URL

**Debug:**
```bash
# Test URL accessibility
curl -I https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.2.0/open_weather_fdw.wasm
```

**Solution:** Ensure firewall allows GitHub access

---

## Performance Optimization

### Caching Strategy

The FDW doesn't cache results. Implement caching in application layer:

```sql
-- Example: Materialized view for hourly refresh
CREATE MATERIALIZED VIEW IF NOT EXISTS weather_cache AS
SELECT * FROM fdw_open_weather.current_weather
WHERE lat IN (52.52, 40.7128, 51.5074)
  AND lon IN (13.405, -74.0060, -0.1278);

-- Refresh hourly
REFRESH MATERIALIZED VIEW weather_cache;
```

### Query Optimization

```sql
-- ❌ Avoid - makes multiple API calls
SELECT * FROM fdw_open_weather.hourly_forecast
WHERE lat IN (52.52, 40.71)
  AND lon IN (13.40, -74.00);

-- ✅ Better - single API call
SELECT * FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405;
```

---

## Reference

- **GitHub Repository:** https://github.com/powabase/supabase-fdw-open-weather
- **OpenWeather API:** https://openweathermap.org/api/one-call-3
- **Supabase Wrappers:** https://fdw.dev
- **Quickstart Guide:** [QUICKSTART.md](../../QUICKSTART.md)
- **Troubleshooting:** [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

**Last Updated:** October 24, 2025
**Version:** v0.2.0
**Maintainer:** Christian Fuerst
