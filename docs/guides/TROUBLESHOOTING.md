# Troubleshooting Guide

Common issues and solutions for OpenWeather WASM FDW.

## Quick Diagnostics

Run these checks first:

```sql
-- 1. Verify foreign server exists
SELECT * FROM pg_foreign_server WHERE srvname = 'openweather_server';

-- 2. Verify foreign tables exist
SELECT foreign_table_schema, foreign_table_name
FROM information_schema.foreign_tables
WHERE foreign_table_schema = 'fdw_open_weather';

-- 3. Test basic query
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405
LIMIT 1;
```

---

## Common Issues

### Issue 1: 401 Unauthorized (Invalid API Key) 🔑

**Symptom:**
```
ERROR: HTTP 401: Invalid API key. Please see http://openweathermap.org/faq#error401 for more info.
```

**Cause:** Missing, incorrect, or inactive OpenWeather API key

**Solution:**

1. **Verify API key exists:**
   - Log in to https://home.openweathermap.org/api_keys
   - Check if key is active (activation can take a few hours)

2. **Check server configuration:**
```sql
SELECT unnest(srvoptions) as option
FROM pg_foreign_server
WHERE srvname = 'openweather_server';
-- Look for api_key option
```

3. **Update API key:**
```sql
ALTER SERVER openweather_server OPTIONS (
  SET api_key 'your_new_api_key_here'
);
```

4. **Test API key directly:**
```bash
curl "https://api.openweathermap.org/data/3.0/onecall?lat=52.52&lon=13.405&appid=YOUR_KEY"
# Should return JSON, not 401 error
```

---

### Issue 2: Missing Required Parameters ⚠️

**Symptom:**
```
ERROR: WHERE clause must include 'lat' (latitude) between -90 and 90.
Example: WHERE lat = 52.52 AND lon = 13.405
```

**Cause:** Query missing required lat/lon parameters

**Solution:**

All OpenWeather endpoints require `lat` and `lon`:

```sql
-- ❌ Bad: Missing parameters
SELECT * FROM fdw_open_weather.current_weather;

-- ✅ Good: Include required parameters
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

**Endpoint-specific requirements:**

```sql
-- historical_weather requires dt (Unix timestamp)
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1609459200;

-- daily_summary requires date (YYYY-MM-DD)
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-01';
```

---

### Issue 3: Invalid Coordinates 🌍

**Symptom:**
```
ERROR: lat must be between -90 and 90, got 100
```

**Cause:** Latitude or longitude out of valid range

**Solution:**

**Valid ranges:**
- Latitude: -90 to 90 (decimal degrees)
- Longitude: -180 to 180 (decimal degrees)

```sql
-- ❌ Bad: Invalid coordinates
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 100 AND lon = 200;

-- ✅ Good: Valid coordinates
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

**Common coordinate mistakes:**
- Using degrees/minutes/seconds format (convert to decimal degrees)
- Swapping lat/lon (lat is N/S, lon is E/W)
- Missing negative sign for southern/western hemispheres

**Example conversions:**
- 52°31'12"N, 13°24'18"E → lat=52.52, lon=13.405
- Sydney, Australia → lat=-33.8688, lon=151.2093
- São Paulo, Brazil → lat=-23.5505, lon=-46.6333

---

### Issue 4: Rate Limit Exceeded 🚫

**Symptom:**
```
ERROR: HTTP 429: Too Many Requests
```

**Cause:** Exceeded OpenWeather API limits (1,000 calls/day for free tier)

**Solution:**

1. **Check current usage:**
   - Visit https://home.openweathermap.org/api_keys
   - View "API calls" section

2. **Implement caching with materialized views:**
```sql
-- Create cached view (reduces API calls by 90%+)
CREATE MATERIALIZED VIEW mv_berlin_weather AS
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every 10 minutes with pg_cron
SELECT cron.schedule('refresh-weather', '*/10 * * * *',
  'REFRESH MATERIALIZED VIEW mv_berlin_weather');

-- Query the cache (0 API calls)
SELECT * FROM mv_berlin_weather;
```

3. **Upgrade API plan:**
   - Visit https://openweathermap.org/price
   - Choose plan with higher limits

**Rate limit tracking:**
```sql
-- Track API usage
CREATE TABLE api_usage_log (
  id SERIAL PRIMARY KEY,
  endpoint TEXT,
  lat NUMERIC,
  lon NUMERIC,
  called_at TIMESTAMPTZ DEFAULT NOW()
);

-- Monitor daily usage
SELECT
  DATE(called_at) as date,
  COUNT(*) as calls,
  1000 - COUNT(*) as remaining
FROM api_usage_log
WHERE called_at >= CURRENT_DATE - INTERVAL '7 days'
GROUP BY DATE(called_at)
ORDER BY date DESC;
```

---

### Issue 5: NULL Values in Results 💀

**Symptom:**
```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
-- Returns: NULL | NULL | NULL | ...
```

**Cause:** Wrong WASM binary (built with `wasm32-wasip1` instead of `wasm32-unknown-unknown`)

**Solution:**

1. **Verify WASM URL and checksum:**
```bash
curl -I https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm
# Should return: HTTP 200

# Download and verify checksum
curl -L https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm \
  -o /tmp/open_weather_fdw.wasm
shasum -a 256 /tmp/open_weather_fdw.wasm
# Compare with checksum in server options
```

2. **Rebuild with correct target:**
```bash
cargo component build --release --target wasm32-unknown-unknown

# Verify zero WASI CLI imports
wasm-tools component wit target/wasm32-unknown-unknown/release/open_weather_fdw.wasm | grep wasi:cli
# Expected: (no output - zero WASI CLI imports)
```

3. **Update server with correct binary:**
```sql
ALTER SERVER openweather_server OPTIONS (
  SET fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm',
  SET fdw_package_checksum 'NEW_CHECKSUM_HERE'
);
```

---

### Issue 6: Minutely Forecast Not Available 🌧️

**Symptom:**
```sql
SELECT COUNT(*) FROM fdw_open_weather.minutely_forecast
WHERE lat = 35.6762 AND lon = 139.6503;
-- Returns: 0 rows (expected: ~60 rows)
```

**Cause:** Minutely precipitation forecast not available for all locations

**Solution:**

Minutely forecast has limited global coverage (primarily Europe and North America).

**Check availability:**
```sql
-- Try query, handle gracefully if empty
SELECT
  CASE WHEN COUNT(*) > 0
    THEN 'Minutely forecast available'
    ELSE 'Minutely forecast not available for this location'
  END as status
FROM fdw_open_weather.minutely_forecast
WHERE lat = YOUR_LAT AND lon = YOUR_LON;
```

**Alternative:** Use hourly forecast with interpolation:
```sql
SELECT
  TO_TIMESTAMP(dt) as time,
  ROUND(pop * 100, 0) as rain_probability_pct,
  COALESCE(rain_1h, 0) as rain_mm
FROM fdw_open_weather.hourly_forecast
WHERE lat = 35.6762 AND lon = 139.6503
ORDER BY dt
LIMIT 2;  -- Next 2 hours
```

---

### Issue 7: No Weather Alerts 📢

**Symptom:**
```sql
SELECT * FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
-- Returns: 0 rows
```

**Cause:** This is **expected behavior** when no active weather alerts exist

**Solution:**

Weather alerts are optional - most locations have no active alerts most of the time.

**Check gracefully:**
```sql
-- Count alerts (0 is normal)
SELECT COUNT(*) as alert_count
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;

-- Show alerts or "None active"
SELECT
  COALESCE(event, 'No active alerts') as alert_status,
  COALESCE(sender_name, 'N/A') as source
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
```

---

### Issue 8: WASM Module Load Failed 🔧

**Symptom:**
```
ERROR: failed to load WASM module
```

**Causes:**
1. Incorrect URL
2. Checksum mismatch
3. Network connectivity issue
4. WASI import error (wrong build target)

**Solution:**

1. **Verify URL is accessible:**
```bash
curl -I https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm
# Should return: HTTP 200
```

2. **Check server options:**
```sql
SELECT
  srvname,
  unnest(srvoptions) as option
FROM pg_foreign_server
WHERE srvname = 'openweather_server';
```

3. **Verify checksum matches:**
```bash
curl -L YOUR_WASM_URL -o /tmp/open_weather_fdw.wasm
shasum -a 256 /tmp/open_weather_fdw.wasm
# Compare with fdw_package_checksum option
```

4. **Update if needed:**
```sql
ALTER SERVER openweather_server OPTIONS (
  SET fdw_package_url 'https://github.com/powabase/supabase-fdw-open-weather/releases/download/v0.1.0/open_weather_fdw.wasm',
  SET fdw_package_checksum 'CORRECT_CHECKSUM_HERE'
);
```

---

### Issue 9: Permission Denied 🔐

**Symptom:**
```
ERROR: permission denied for schema fdw_open_weather
```

**Solution:**
```sql
-- Grant schema usage
GRANT USAGE ON SCHEMA fdw_open_weather TO postgres;

-- Grant table access
GRANT SELECT ON ALL TABLES IN SCHEMA fdw_open_weather TO postgres;

-- Grant on future tables
ALTER DEFAULT PRIVILEGES IN SCHEMA fdw_open_weather
  GRANT SELECT ON TABLES TO postgres;
```

---

## Local Testing Issues

### Issue 10: Docker Networking 🐳

**Symptom:**
```
ERROR: failed to fetch WASM binary from localhost:8000
```

**Cause:** Using `localhost` or `file://` URL with Docker

**Solution:**

Supabase runs in Docker - use `host.docker.internal` instead of `localhost`:

```sql
-- ❌ Bad: Won't work from Docker
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://localhost:8000/open_weather_fdw.wasm',
    -- ...
  );

-- ✅ Good: Works from Docker
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://host.docker.internal:8000/open_weather_fdw.wasm',
    -- ...
  );
```

**Start HTTP server:**
```bash
cd target/wasm32-unknown-unknown/release
python3 -m http.server 8000
```

---

## Build Issues

### Issue 11: WASI CLI Import Error 🛠️

**Symptom:**
```
ERROR: component imports instance 'wasi:cli/environment@0.2.0'
```

**Cause:** Built with `wasm32-wasip1` instead of `wasm32-unknown-unknown`

**Solution:**

**CRITICAL:** Always use `wasm32-unknown-unknown` target:

```bash
# ❌ Wrong: Creates WASI CLI imports
cargo component build --release --target wasm32-wasip1

# ✅ Correct: No WASI CLI imports
cargo component build --release --target wasm32-unknown-unknown

# Verify zero WASI imports
wasm-tools component wit target/wasm32-unknown-unknown/release/open_weather_fdw.wasm | grep wasi:cli
# Expected: (no output)
```

---

### Issue 12: Binary Size Too Large 📦

**Symptom:** WASM binary > 200 KB

**Target:** < 150 KB

**Solution:**

Check `Cargo.toml` has size optimizations:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Enable link-time optimization
strip = "debuginfo"  # Remove debug info
codegen-units = 1    # Slower compile, better optimization
```

**Verify size:**
```bash
ls -lh target/wasm32-unknown-unknown/release/open_weather_fdw.wasm
# Should show < 150 KB
```

---

## Debugging Workflow

If issues persist, follow this workflow:

### 1. Check Supabase Logs

```bash
supabase logs --db
```

Look for errors related to:
- WASM module loading
- Foreign table access
- API connectivity

### 2. Verify Installation

```sql
-- Check WASM wrapper exists
SELECT * FROM pg_foreign_data_wrapper WHERE fdwname = 'wasm_wrapper';

-- Check foreign server configuration
SELECT * FROM pg_foreign_server WHERE srvname = 'openweather_server';

-- Check foreign tables
SELECT foreign_table_schema, foreign_table_name, foreign_server_name
FROM information_schema.foreign_tables
WHERE foreign_table_schema = 'fdw_open_weather';
```

### 3. Test API Connectivity

```bash
# Test OpenWeather API directly
curl "https://api.openweathermap.org/data/3.0/onecall?lat=52.52&lon=13.405&appid=YOUR_KEY"
# Should return JSON with weather data
```

### 4. Check Query Plan

```sql
-- Analyze query execution
EXPLAIN (ANALYZE, VERBOSE, BUFFERS)
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Expected:
-- Planning Time: < 1ms
-- Execution Time: 500-2000ms
-- Rows: 1
```

---

## Rollback Procedure

If all else fails, rollback and reinstall:

```sql
-- 1. Drop foreign tables
DROP SCHEMA IF EXISTS fdw_open_weather CASCADE;

-- 2. Drop foreign server
DROP SERVER IF EXISTS openweather_server CASCADE;

-- 3. Optionally drop FDW wrapper
DROP FOREIGN DATA WRAPPER IF EXISTS wasm_wrapper CASCADE;

-- 4. Reinstall following QUICKSTART.md
```

---

## Getting Help

### Information to Collect

When reporting issues, include:

1. **Query that's failing:**
```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

2. **Error message:** Full error text

3. **Server configuration:**
```sql
SELECT unnest(srvoptions)
FROM pg_foreign_server
WHERE srvname = 'openweather_server';
```

4. **Supabase version:**
```bash
supabase --version
```

5. **Logs:** Output from `supabase logs --db`

### Where to Get Help

- **GitHub Issues:** https://github.com/powabase/supabase-fdw-open-weather/issues
- **Supabase Discord:** https://discord.supabase.com
- **Documentation:** [README.md](../../README.md)
- **OpenWeather Support:** https://openweathermap.desk.com

---

## Related Documentation

- **[QUICKSTART.md](../../QUICKSTART.md)** - Setup guide
- **[DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)** - Deployment instructions
- **[API Overview](../reference/API_OVERVIEW.md)** - API details
- **[Endpoints](../endpoints/)** - Endpoint documentation
- **[CLAUDE.md](../../CLAUDE.md)** - Development guide
