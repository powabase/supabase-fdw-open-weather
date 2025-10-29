# Troubleshooting Guide

Common issues and solutions for OpenWeather WASM FDW.

## Top 5 Issues

### 1. Column "lat" or "lon" Does Not Exist

**Error:**
```
ERROR: column "lat" does not exist
```

**Cause:** Using old v0.2.0 column names

**Solution:** Use v0.3.1 column names
```sql
-- ❌ Old (v0.2.0)
WHERE lat = 52.52 AND lon = 13.405

-- ✅ New (v0.3.1)
WHERE latitude = 52.52 AND longitude = 13.405
```

See [MIGRATION.md](../../MIGRATION.md) for full upgrade guide.

### 2. Missing Required Parameter

**Error:**
```
ERROR: Missing required parameter: latitude
```

**Cause:** WHERE clause missing required parameters

**Solution:** All endpoints require `latitude` and `longitude`
```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;  -- Both required!
```

### 3. Invalid API Key

**Error:**
```
ERROR: Invalid API key
```

**Cause:** API key not set or incorrect

**Solution:** Check your API key in server options
```sql
-- View current server options
SELECT * FROM pg_foreign_server WHERE srvname = 'openweather_server';

-- Update API key
ALTER SERVER openweather_server OPTIONS (SET api_key 'your_actual_api_key');
```

Get your API key at: https://openweathermap.org/api/one-call-3

### 4. Wrong Checksum

**Error:**
```
ERROR: Package checksum mismatch
```

**Cause:** Checksum doesn't match WASM binary

**Solution:** Use correct checksum from [README.md](../../README.md#release-information)
```sql
ALTER SERVER openweather_server OPTIONS (
  SET fdw_package_checksum '0abb03a28bce499c1fdeedd0b64c461b226a907c3bcfc6542eb6d36e951f9eee'
);
```

### 5. Parameter Column Does Not Exist (v0.3.1)

**Error:**
```
ERROR: column "dt" does not exist
ERROR: column "date" does not exist
```

**Cause:** Using old API parameter names instead of semantic columns

**Solution:** Use v0.3.1 semantic parameter names
```sql
-- historical_weather: Use observation_time (not dt)
WHERE observation_time = '2024-01-01 00:00:00+00'

-- daily_summary: Use summary_date (not date)
WHERE summary_date = '2024-01-15'

-- weather_overview: Use overview_date (not date)
WHERE overview_date = '2025-10-29'
```

## Need More Help?

- **Documentation:** [README.md](../../README.md)
- **Examples:** [SQL Examples](../reference/SQL_EXAMPLES.md)
- **Deployment:** [Deployment Guide](DEPLOYMENT_GUIDE.md)
- **Issues:** [GitHub Issues](https://github.com/powabase/supabase-fdw-open-weather/issues)
