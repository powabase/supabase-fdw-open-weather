# OpenWeather One Call API 3.0 Overview

**Provider:** OpenWeather Ltd
**Base URL:** https://api.openweathermap.org/data/3.0
**Authentication:** API key required (appid parameter)
**Coverage:** Worldwide
**Update Frequency:** Every 10 minutes
**Documentation:** https://openweathermap.org/api/one-call-3

---

## About OpenWeather One Call API 3.0

OpenWeather provides comprehensive weather data through the One Call API 3.0, offering current conditions, forecasts, historical data, and AI-powered weather summaries for any location worldwide.

**Key Features:**
- Current weather conditions updated every 10 minutes
- Minute-by-minute precipitation forecast (1 hour)
- Hourly forecast (48 hours ahead)
- Daily forecast (8 days ahead)
- Historical weather data (46+ years archive)
- Daily aggregations and statistics
- AI-generated weather summaries
- Government weather alerts
- Multilingual support (50+ languages)

---

## API Characteristics

### Authentication

**Required:** API key (appid parameter)

```bash
# Example API call
curl "https://api.openweathermap.org/data/3.0/onecall?lat=52.52&lon=13.405&appid=YOUR_API_KEY"
```

**Getting an API Key:**
1. Sign up at https://openweathermap.org/api
2. Subscribe to "One Call by Call" plan
3. Free tier: 1,000 calls/day
4. Paid plans available for higher usage

### Rate Limits

- **Free Plan:** 1,000 calls/day, 60 calls/minute
- **Startup Plan:** 100,000 calls/day
- **Developer Plan:** 1,000,000 calls/day
- **Professional Plan:** Custom limits

**Recommendation:** Cache data locally with materialized views to stay within limits

**Best Practice:** Refresh materialized views every 10-15 minutes (matching API update frequency)

### Response Format

All endpoints return JSON with nested structures. The WASM FDW flattens these into PostgreSQL rows.

**Example /onecall Response Structure:**
```json
{
  "lat": 52.52,
  "lon": 13.405,
  "timezone": "Europe/Berlin",
  "timezone_offset": 7200,
  "current": { ... },      // Single object
  "minutely": [ ... ],     // 61 objects (1 per minute)
  "hourly": [ ... ],       // 48 objects (1 per hour)
  "daily": [ ... ],        // 8 objects (1 per day)
  "alerts": [ ... ]        // Variable (0-N alerts)
}
```

### Error Handling

**HTTP 401 - Unauthorized:**
```json
{
  "cod": 401,
  "message": "Invalid API key"
}
```
**Cause:** Missing or incorrect API key

**HTTP 404 - Not Found:**
```json
{
  "cod": "404",
  "message": "data not found"
}
```
**Cause:** Invalid coordinates or timestamp

**HTTP 429 - Too Many Requests:**
```json
{
  "cod": 429,
  "message": "rate limit exceeded"
}
```
**Cause:** Exceeded daily or per-minute rate limit

---

## Supported Parameters

### Geographic Coordinates (All Endpoints)

**Required:**
- `lat` (numeric): Latitude in decimal degrees (-90 to 90)
- `lon` (numeric): Longitude in decimal degrees (-180 to 180)

**Examples:**
- Berlin: lat=52.52, lon=13.405
- New York: lat=40.7128, lon=-74.0060
- Tokyo: lat=35.6762, lon=139.6503
- Sydney: lat=-33.8688, lon=151.2093

### Units (Optional)

- `standard` - Kelvin (default)
- `metric` - Celsius, meter/sec
- `imperial` - Fahrenheit, miles/hour

**Default in FDW:** `metric`

### Languages (Optional)

Weather descriptions available in 50+ languages:

```
en - English (default)
de - German
es - Spanish
fr - French
it - Italian
pt - Portuguese
ru - Russian
zh_cn - Chinese Simplified
zh_tw - Chinese Traditional
ja - Japanese
ko - Korean
ar - Arabic
hi - Hindi
```

**Full list:** https://openweathermap.org/api/one-call-3#multi

### Temporal Parameters

**historical_weather endpoint:**
- `dt` (integer): Unix timestamp for specific date/time

**daily_summary endpoint:**
- `date` (string): ISO 8601 date (YYYY-MM-DD)
- `tz` (string): Timezone offset (+/-HHMM)

**weather_overview endpoint:**
- `date` (string): ISO 8601 date (defaults to today)

---

## Data Quality & Accuracy

### Data Sources

- **Current weather:** Weather stations, satellites, radar
- **Forecasts:** Proprietary OpenWeather Model
- **Historical:** Quality-controlled weather archive (1979-present)
- **Alerts:** National weather agencies and government sources

### Update Latency

- **Current weather:** 10-minute updates
- **Minutely forecast:** 10-minute updates
- **Hourly forecast:** Hourly updates
- **Daily forecast:** Multiple times per day
- **Historical:** Static (archive)
- **Alerts:** Real-time as issued

### Known Limitations

1. **Minutely precipitation:** Only available for supported locations
2. **Weather alerts:** Availability varies by country/region
3. **Historical data:** Archive starts from 1979 (January 1, 1979, 00:00 GMT)
4. **Forecast accuracy:** Decreases with time horizon (48h hourly less accurate than 24h)
5. **Time zones:** All timestamps in Unix UTC (convert as needed)
6. **API response caching:** OpenWeather caches responses for ~10 minutes

---

## API Endpoints Mapping

### One Call API 3.0 → FDW Tables

| API Endpoint | HTTP Path | FDW Tables | Row Count |
|--------------|-----------|------------|-----------|
| **Current & Forecasts** | /onecall | 5 tables | 1, 60, 48, 8, 0-N |
| **Historical** | /onecall/timemachine | 1 table | 1 |
| **Daily Aggregation** | /onecall/day_summary | 1 table | 1 |
| **AI Overview** | /onecall/overview | 1 table | 1 |

### /onecall Endpoint → 5 Foreign Tables

The main `/onecall` endpoint returns multiple data types in one response. The FDW splits this into separate tables:

1. **current_weather** - Current conditions (1 row)
2. **minutely_forecast** - Precipitation (60 rows, 1 per minute)
3. **hourly_forecast** - Detailed forecast (48 rows, 1 per hour)
4. **daily_forecast** - Summary forecast (8 rows, 1 per day)
5. **weather_alerts** - Government alerts (0-N rows)

**Why separate tables?**
- Cleaner schema (no mixing of different temporal resolutions)
- Easier querying (no need for complex filtering)
- Better performance (only parse what you need)

---

## Cost Optimization

### Materialized Views Strategy

**Problem:** Each query = 1 API call = 1 count toward rate limit

**Solution:** Cache data with materialized views

```sql
-- Current weather for Berlin (refresh every 10 min)
CREATE MATERIALIZED VIEW mv_berlin_current AS
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh with pg_cron
SELECT cron.schedule('refresh-berlin-weather', '*/10 * * * *',
  'REFRESH MATERIALIZED VIEW mv_berlin_current');
```

**Benefits:**
- Sub-millisecond query response
- Reduces API calls by ~99% (600 queries/day → 144 API calls/day)
- Matches API update frequency

### Multi-Location Caching

```sql
-- Cache weather for multiple cities
CREATE TABLE locations (
  city_name TEXT PRIMARY KEY,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO locations VALUES
  ('Berlin', 52.52, 13.405),
  ('London', 51.5074, -0.1278),
  ('Paris', 48.8566, 2.3522);

-- Create unified cache
CREATE MATERIALIZED VIEW mv_cities_weather AS
SELECT
  l.city_name,
  c.*
FROM locations l
CROSS JOIN LATERAL (
  SELECT * FROM fdw_open_weather.current_weather
  WHERE lat = l.lat AND lon = l.lon
) c;

-- Refresh all cities with one command
REFRESH MATERIALIZED VIEW mv_cities_weather;
```

**Cost:** 3 API calls per refresh (once per city)

### Rate Limit Monitoring

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
  endpoint,
  COUNT(*) as calls_today,
  1000 - COUNT(*) as remaining_calls
FROM api_usage_log
WHERE called_at >= CURRENT_DATE
GROUP BY endpoint;
```

---

## Best Practices

### 1. Always Validate Coordinates

```sql
-- Bad: Invalid coordinates cause 404 errors
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 100 AND lon = 200;  -- ERROR!

-- Good: Use valid ranges
SELECT * FROM fdw_open_weather.current_weather
WHERE lat BETWEEN -90 AND 90
  AND lon BETWEEN -180 AND 180;
```

### 2. Use Materialized Views for Repeated Queries

```sql
-- Bad: Each query hits API
SELECT * FROM fdw_open_weather.hourly_forecast WHERE lat = 52.52 AND lon = 13.405;
SELECT * FROM fdw_open_weather.hourly_forecast WHERE lat = 52.52 AND lon = 13.405;
-- Result: 2 API calls

-- Good: Query cached data
REFRESH MATERIALIZED VIEW mv_berlin_hourly;  -- 1 API call
SELECT * FROM mv_berlin_hourly;  -- 0 API calls
SELECT * FROM mv_berlin_hourly;  -- 0 API calls
-- Result: 1 API call
```

### 3. Handle Missing Data Gracefully

```sql
-- Some locations don't have minutely precipitation data
SELECT
  COALESCE(precipitation, 0) as precipitation_mm
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Weather alerts may not exist
SELECT COUNT(*) as alert_count
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
-- Returns: 0 if no alerts
```

### 4. Convert Unix Timestamps

```sql
-- Display human-readable times
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time_utc,
  TO_TIMESTAMP(dt) AT TIME ZONE 'Europe/Berlin' as time_local,
  temp,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 24;
```

---

## Use Cases

### Smart Home Automation

Use current weather and forecasts to automate home systems:
- **Heating/cooling:** Adjust based on temperature forecast
- **Irrigation:** Skip watering if rain predicted
- **Window blinds:** Close if high UV or hot weather
- **Ventilation:** Open windows during ideal conditions

### Agriculture

Plan farming activities based on weather:
- **Planting:** Check 8-day forecast for optimal timing
- **Harvesting:** Avoid rain days
- **Irrigation scheduling:** Use precipitation forecasts
- **Frost protection:** Monitor temperature minimums

### Energy Management

Optimize energy consumption:
- **Solar forecast:** Plan battery charging/discharging
- **HVAC optimization:** Pre-heat/cool before weather changes
- **EV charging:** Schedule during favorable conditions
- **Demand response:** Reduce load during extreme weather

### Event Planning

Make informed decisions for outdoor events:
- **Date selection:** Analyze historical weather patterns
- **Risk assessment:** Check daily forecasts and alerts
- **Backup planning:** Monitor precipitation probability
- **Attendee communication:** Share AI weather summaries

### Research & Analysis

Access comprehensive weather data:
- **Climate studies:** Historical data analysis (46+ years)
- **Weather pattern recognition:** Daily aggregations
- **Forecast accuracy:** Compare predictions vs actual
- **Alert analysis:** Study severe weather events

---

## API Versioning & Changes

### Current Status

- **API Version:** 3.0 (stable)
- **Breaking Changes:** Rare, announced via blog/email
- **Deprecation:** Deprecated features documented in API docs
- **Migration:** One Call API 2.5 deprecated in favor of 3.0

### Version History

- **2023:** One Call API 3.0 released (current)
- **2021:** One Call API 2.5 introduced
- **2019:** One Call API 1.0 (initial release)

---

## Support & Resources

- **API Documentation:** https://openweathermap.org/api/one-call-3
- **FAQ:** https://openweathermap.org/faq
- **Pricing:** https://openweathermap.org/price
- **Support:** support@openweathermap.org
- **Community:** https://openweathermap.desk.com
- **Status Page:** https://openweather.statuspage.io

---

## License & Terms

- **Free Tier:** 1,000 calls/day for personal/non-commercial use
- **Commercial Use:** Paid subscription required
- **Attribution:** Not required but appreciated
- **Data Rights:** OpenWeather retains all rights
- **Redistribution:** Prohibited without permission

**Terms of Service:** https://openweathermap.org/terms

---

## See Also

- **[Endpoints Overview](../endpoints/README.md)** - Comparison of all 9 FDW tables
- **[SQL Examples](SQL_EXAMPLES.md)** - Query patterns and examples
- **[Troubleshooting Guide](../guides/TROUBLESHOOTING.md)** - Common API issues
- **[CLAUDE.md](../../CLAUDE.md)** - Project quick reference
