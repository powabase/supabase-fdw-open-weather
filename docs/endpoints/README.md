# Endpoints Overview - OpenWeather WASM FDW

**Version:** v0.2.0
**Status:** ✅ Production Ready
**Total Endpoints:** 8
**Total Columns:** 101
**Last Updated:** October 24, 2025

This directory contains detailed documentation for all OpenWeather WASM FDW endpoints.

---

## Quick Comparison

| Endpoint | Rows | Columns | Data Type | Update Freq | Response Size | Use Case |
|----------|------|---------|-----------|-------------|---------------|----------|
| **[current-weather](current-weather.md)** | 1 | 18 | Current | 10 min | 2-3 KB | 🌤️ Current conditions for real-time monitoring |
| **[minutely-forecast](minutely-forecast.md)** | 60 | 4 | Forecast | 10 min | 1-2 KB | ☔ Minute-by-minute precipitation (next hour) |
| **[hourly-forecast](hourly-forecast.md)** | 48 | 19 | Forecast | Hourly | 8-10 KB | 📊 Detailed 48-hour forecast with rain/snow |
| **[daily-forecast](daily-forecast.md)** | 8 | 32 | Forecast | Multiple/day | 6-8 KB | 📅 8-day forecast with moon/sun times |
| **[weather-alerts](weather-alerts.md)** | 0-N | 8 | Real-time | Real-time | Variable | 🚨 Government weather alerts |
| **[historical-weather](historical-weather.md)** | 1 | 15 | Historical | Static | 2-3 KB | 📜 Point-in-time historical data (1979+) |
| **[daily-summary](daily-summary.md)** | 1 | 17 | Historical | Daily | ~350 B | 📊 Daily aggregated statistics (1979+) |
| **[weather-overview](weather-overview.md)** | 1 | 6 | AI Summary | 10 min | ~650 B | 🤖 Human-readable AI weather summaries |

---

## Endpoint Categories

### Real-Time Data (10-Minute Updates)

These endpoints provide current and near-future data updated every 10 minutes:

- **current_weather** - Instantaneous conditions
- **minutely_forecast** - Precipitation next hour
- **hourly_forecast** - Next 48 hours (updated hourly)
- **daily_forecast** - Next 8 days (updated multiple times daily)
- **weather_alerts** - Active government alerts

### Historical Data

- **historical_weather** - Any specific timestamp since 1979
- **daily_summary** - Daily aggregated statistics for specific dates (1979+)

### AI-Generated Content

- **weather_overview** - Natural language weather summaries for today/tomorrow

---

## Column Summary

**Total: 101 columns across 8 endpoints**
- current_weather: 18 columns
- minutely_forecast: 4 columns
- hourly_forecast: 19 columns
- daily_forecast: 32 columns
- weather_alerts: 8 columns
- historical_weather: 15 columns
- daily_summary: 17 columns
- weather_overview: 6 columns

### Common Columns (Required in WHERE Clause)
- `lat` - Latitude (-90 to 90)
- `lon` - Longitude (-180 to 180)

### Time Columns
- `dt` - Unix timestamp (all endpoints except weather_alerts which uses start/end)

### Weather Metrics (Appear in Multiple Endpoints)
- `temp`, `feels_like` - Temperature values
- `pressure`, `humidity` - Atmospheric conditions
- `wind_speed`, `wind_deg`, `wind_gust` - Wind data
- `clouds`, `visibility` - Sky conditions
- `uvi` - UV index
- `dew_point` - Dew point temperature

### Weather Descriptions
- `weather_main` - Main weather category ("Rain", "Clear", etc.)
- `weather_description` - Detailed description ("light rain", "clear sky")
- `weather_icon` - Icon code ("10d", "01n")

### Precipitation
- `precipitation` - Minutely forecast (mm)
- `pop` - Probability of precipitation (0-1)
- `rain_1h`, `snow_1h` - Hourly volumes (optional)
- `rain`, `snow` - Daily totals (optional)

### Daily-Specific Columns
- `sunrise`, `sunset` - Sun times
- `moonrise`, `moonset`, `moon_phase` - Moon data
- `temp_min`, `temp_max`, `temp_day`, `temp_night`, `temp_eve`, `temp_morn` - Temperature ranges

### Alert-Specific Columns
- `sender_name` - Alert source
- `event` - Event type
- `start`, `end_time` - Alert time range
- `description` - Full alert text
- `tags` - Alert categories

---

## Query Patterns

### Basic Queries (Required lat/lon)

All endpoints require `lat` and `lon` in the WHERE clause:

```sql
-- Current weather for Berlin
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Hourly forecast for New York
SELECT * FROM fdw_open_weather.hourly_forecast
WHERE lat = 40.7128 AND lon = -74.0060;
```

### Historical Queries (Additional Parameters)

Some endpoints require additional parameters:

```sql
-- Historical weather (requires dt - Unix timestamp)
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1609459200;

-- Daily summary (requires date - YYYY-MM-DD)
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-01';
```

### Cross-Endpoint Queries

Combine multiple endpoints for richer analysis:

```sql
-- Compare current conditions with hourly forecast
SELECT
  'current' as type,
  c.temp,
  c.weather_description
FROM fdw_open_weather.current_weather c
WHERE c.lat = 52.52 AND c.lon = 13.405

UNION ALL

SELECT
  'forecast' as type,
  h.temp,
  h.weather_description
FROM fdw_open_weather.hourly_forecast h
WHERE h.lat = 52.52 AND h.lon = 13.405
ORDER BY temp DESC
LIMIT 5;
```

### Multi-Location Queries

Query weather for multiple locations:

```sql
-- Create locations table
CREATE TABLE locations (
  city_name TEXT,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO locations VALUES
  ('Berlin', 52.52, 13.405),
  ('London', 51.5074, -0.1278),
  ('Paris', 48.8566, 2.3522);

-- Get current weather for all cities
SELECT
  l.city_name,
  c.temp,
  c.weather_description
FROM locations l
CROSS JOIN LATERAL (
  SELECT temp, weather_description
  FROM fdw_open_weather.current_weather
  WHERE lat = l.lat AND lon = l.lon
) c;
```

---

## WHERE Clause Parameters

### All Endpoints (Required)
- `lat` (numeric): Latitude in decimal degrees (-90 to 90)
- `lon` (numeric): Longitude in decimal degrees (-180 to 180)

### Optional Parameters (Handled by Server Options)
- `units`: "metric" (default), "imperial", or "standard"
- `lang`: Language code (default: "en")

### Endpoint-Specific Required Parameters

**historical_weather:**
- `dt` (bigint): Unix timestamp

**Note:** `daily_summary` endpoint deferred to v0.2.0

---

## Performance Characteristics

### Query Response Times

- **Single endpoint:** < 2 seconds
- **Multi-location (3 cities):** 3-5 seconds
- **Cross-endpoint JOINs:** 2-4 seconds

### API Call Optimization

Each query = 1 API call. Use materialized views to cache:

```sql
-- Cache current weather for Berlin (refresh every 10 min)
CREATE MATERIALIZED VIEW mv_berlin_current AS
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh with pg_cron
SELECT cron.schedule('refresh-berlin-weather', '*/10 * * * *',
  'REFRESH MATERIALIZED VIEW mv_berlin_current');
```

**Cost Savings:**
- Without cache: 1,440 API calls/day (1 query/min)
- With cache: 144 API calls/day (1 refresh/10 min)
- **Savings: 90%**

---

## Use Case Matrix

| Use Case | Recommended Endpoints | Query Type |
|----------|----------------------|------------|
| **Smart Home Automation** | current_weather + hourly_forecast | Current + short-term forecast |
| **Agriculture Planning** | daily_forecast + historical_weather | Forecast + historical patterns |
| **Event Planning** | daily_forecast + weather_alerts | Multi-day forecast + alerts |
| **Energy Management** | hourly_forecast + daily_forecast | Short and long-term planning |
| **Emergency Response** | weather_alerts + current_weather | Alerts + current conditions |
| **Outdoor Activities** | minutely_forecast + current_weather | Precipitation + conditions |
| **Climate Research** | historical_weather + daily_summary | Long-term data analysis |
| **Weather App** | current_weather + hourly_forecast + daily_forecast + weather_alerts | Comprehensive view |

---

## Common Query Examples

### Find Next Rain Event

```sql
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(pop * 100, 0) as rain_probability_pct,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND pop > 0.3  -- More than 30% chance of rain
ORDER BY dt
LIMIT 5;
```

### Compare Today's Forecast with Historical

```sql
-- Today's forecast max temp
WITH forecast_today AS (
  SELECT MAX(temp_max) as max_temp
  FROM fdw_open_weather.daily_forecast
  WHERE lat = 52.52 AND lon = 13.405
  LIMIT 1
),
-- Historical temp for same date last year
historical_last_year AS (
  SELECT temp as max_temp
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = EXTRACT(EPOCH FROM (CURRENT_DATE - INTERVAL '1 year'))::bigint
)
SELECT
  ROUND(f.max_temp, 1) as forecast_today_c,
  ROUND(h.max_temp, 1) as historical_last_year_c,
  ROUND(f.max_temp - h.max_temp, 1) as difference_c
FROM forecast_today f, historical_last_year h;
```

### Monitor Extreme Weather

```sql
-- Combine daily forecast with alerts
SELECT
  TO_TIMESTAMP(d.dt) AT TIME ZONE 'UTC' as date,
  ROUND(d.temp_max, 1) as max_temp_c,
  ROUND(d.temp_min, 1) as min_temp_c,
  d.weather_main,
  COALESCE(a.event, 'No alerts') as alert_type
FROM fdw_open_weather.daily_forecast d
LEFT JOIN fdw_open_weather.weather_alerts a
  ON d.lat = a.lat AND d.lon = a.lon
WHERE d.lat = 52.52 AND d.lon = 13.405
ORDER BY d.dt;
```

### Calculate Average Conditions

```sql
-- Average conditions for next 24 hours
SELECT
  ROUND(AVG(temp), 1) as avg_temp_c,
  ROUND(AVG(humidity), 0) as avg_humidity_pct,
  ROUND(AVG(wind_speed), 1) as avg_wind_ms,
  ROUND(AVG(pop) * 100, 0) as avg_rain_prob_pct,
  COUNT(*) as hours_counted
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '24 hours')::bigint;
```

---

## Data Quality & Validation

### Expected Row Counts

- **current_weather:** Always 1 row
- **minutely_forecast:** ~60 rows (may vary by location, not available worldwide)
- **hourly_forecast:** Always 48 rows
- **daily_forecast:** Always 8 rows
- **weather_alerts:** 0-N rows (depends on active alerts)
- **historical_weather:** Always 1 row

### Common Data Issues

**NULL Values:**
- `wind_gust`: NULL when no gusts
- `rain_1h`, `snow_1h`, `rain`, `snow`: NULL when no precipitation
- `minutely_forecast`: May not be available for all locations

**Missing Endpoints:**
- `minutely_forecast`: Not available worldwide (primarily Europe, North America)
- `weather_alerts`: Availability varies by country/region

**Timestamp Validation:**
```sql
-- Verify timestamps are sequential (minutely)
SELECT
  dt,
  dt - LAG(dt) OVER (ORDER BY dt) as interval_seconds
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
LIMIT 10;
-- Expected: 60 seconds between rows
```

---

## Testing Checklist

When testing endpoint integration:

- [ ] Query returns expected row count
- [ ] All columns present (no unexpected NULLs in required fields)
- [ ] Timestamps are valid Unix timestamps
- [ ] WHERE clause parameters filter correctly
- [ ] Optional parameters work (units, lang)
- [ ] Temperature values match selected units
- [ ] Weather descriptions in correct language
- [ ] Aggregations complete without errors (GROUP BY, AVG, etc.)
- [ ] JOINs with other endpoints work
- [ ] Performance acceptable (< 5 seconds for most queries)

---

## API Limits & Caching

### Rate Limits

- **Free Plan:** 1,000 calls/day
- **Each query = 1 API call**

### Caching Strategy

```sql
-- Example: Cache weather for 5 cities with 10-min refresh
-- Cost: 5 cities × 6 refreshes/hour × 24 hours = 720 calls/day
-- Savings: Unlimited queries against cache with 0 additional API calls

CREATE MATERIALIZED VIEW mv_multi_city_weather AS
SELECT
  l.city_name,
  c.*
FROM locations l
CROSS JOIN LATERAL (
  SELECT * FROM fdw_open_weather.current_weather
  WHERE lat = l.lat AND lon = l.lon
) c;
```

---

## See Also

- **[CLAUDE.md](../../CLAUDE.md)** - Quick reference and development guide
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[SQL Examples](../reference/SQL_EXAMPLES.md)** - Advanced query patterns
- **[Troubleshooting Guide](../guides/TROUBLESHOOTING.md)** - Common issues and solutions
- **[Implementation Plan](../IMPLEMENTATION_PLAN.md)** - Development roadmap
