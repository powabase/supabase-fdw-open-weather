# Minutely Forecast Endpoint

**Endpoint:** `minutely_forecast`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall
**Section:** `minutely` array from `/onecall` response

Minute-by-minute precipitation forecast for the next hour (60 data points). Not available for all locations worldwide.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.minutely_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  precipitation numeric
)
SERVER openweather_server
OPTIONS (object 'minutely_forecast');
```

## Columns

| Column | Type | Description | Units | Nullable |
|--------|------|-------------|-------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees (-90 to 90) | No |
| `lon` | numeric | Longitude of location | Decimal degrees (-180 to 180) | No |
| `dt` | bigint | Forecast time | Unix timestamp (UTC) | No |
| `precipitation` | numeric | Precipitation volume | mm (millimeters) | Yes |

### Precipitation Values
- `0` - No precipitation
- `> 0` - Precipitation volume in mm
- NULL - Data not available

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |

### Optional Parameters (Server OPTIONS)
- `units`: Does not affect precipitation (always in mm)
- `lang`: Language code (default: "en")

## Basic Usage

```sql
-- Next hour precipitation for Berlin
SELECT * FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Next hour with human-readable time
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(precipitation, 2) as precip_mm
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;

-- Check if rain expected in next 30 minutes
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(precipitation, 2) as precip_mm,
  CASE
    WHEN precipitation > 0 THEN 'Rain expected'
    ELSE 'No rain'
  END as forecast
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '30 minutes')::bigint
ORDER BY dt;
```

## Advanced Queries

### Find Next Rain Event

```sql
-- Detect when rain will start
WITH rain_events AS (
  SELECT
    TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
    dt,
    ROUND(precipitation, 2) as precip_mm,
    LAG(precipitation) OVER (ORDER BY dt) as prev_precip
  FROM fdw_open_weather.minutely_forecast
  WHERE lat = 52.52 AND lon = 13.405
)
SELECT
  time,
  precip_mm,
  EXTRACT(EPOCH FROM (time - NOW()))/60 as minutes_from_now
FROM rain_events
WHERE precipitation > 0 AND (prev_precip IS NULL OR prev_precip = 0)
ORDER BY dt
LIMIT 1;
```

### Precipitation Intensity Analysis

```sql
-- Categorize precipitation intensity
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(precipitation, 2) as precip_mm,
  CASE
    WHEN precipitation = 0 THEN 'No rain'
    WHEN precipitation < 0.1 THEN 'Drizzle'
    WHEN precipitation < 2.5 THEN 'Light rain'
    WHEN precipitation < 10 THEN 'Moderate rain'
    WHEN precipitation < 50 THEN 'Heavy rain'
    ELSE 'Violent rain'
  END as intensity
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND precipitation > 0
ORDER BY dt;
```

### Total Expected Precipitation

```sql
-- Calculate total precipitation next hour
SELECT
  ROUND(SUM(precipitation), 2) as total_precip_mm,
  ROUND(AVG(precipitation), 2) as avg_precip_mm,
  ROUND(MAX(precipitation), 2) as peak_precip_mm,
  COUNT(CASE WHEN precipitation > 0 THEN 1 END) as minutes_with_rain
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405;
```

## Response Characteristics

- **Rows:** ~60 (one per minute)
- **Availability:** Europe, North America, and select regions only
- **Update Frequency:** Every 10 minutes
- **Response Size:** ~1-2 KB
- **Query Time:** < 2 seconds
- **Time Interval:** 60 seconds between rows

## Data Quality Notes

### Availability

**Available Regions:**
- Most of Europe
- United States and Canada
- Parts of Asia and Middle East

**Not Available:**
- Some rural/remote areas
- Parts of Africa, South America, Oceania

**Check Availability:**
```sql
-- If this returns 0 rows, minutely forecast not available for location
SELECT COUNT(*) as row_count
FROM fdw_open_weather.minutely_forecast
WHERE lat = YOUR_LAT AND lon = YOUR_LON;
```

### Guaranteed Fields
- `lat`, `lon`, `dt` always present
- `precipitation` may be NULL if data unavailable

### Time Validation

```sql
-- Verify 60-second intervals
SELECT
  dt,
  dt - LAG(dt) OVER (ORDER BY dt) as interval_seconds
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
LIMIT 10;
-- Expected: 60 seconds between rows
```

## Common Use Cases

### Smart Irrigation System

```sql
-- Check if watering needed (no rain next 30 min)
SELECT
  CASE
    WHEN SUM(precipitation) > 2 THEN 'Skip watering - rain expected'
    WHEN SUM(precipitation) BETWEEN 0.1 AND 2 THEN 'Partial watering recommended'
    ELSE 'Water plants - no rain expected'
  END as irrigation_advice,
  ROUND(SUM(precipitation), 2) as expected_rain_mm
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '30 minutes')::bigint;
```

### Outdoor Event Planning

```sql
-- Will it rain during my event? (next 45 minutes)
SELECT
  CASE
    WHEN MAX(precipitation) > 0 THEN
      'Rain expected at ' ||
      TO_CHAR(
        TO_TIMESTAMP(MIN(CASE WHEN precipitation > 0 THEN dt END)),
        'HH24:MI'
      )
    ELSE 'No rain expected - safe to proceed'
  END as event_recommendation,
  ROUND(SUM(precipitation), 2) as total_expected_mm
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '45 minutes')::bigint;
```

### Window Closing Automation

```sql
-- Alert if rain coming soon
SELECT
  TO_TIMESTAMP(MIN(dt)) AT TIME ZONE 'UTC' as rain_starts_at,
  ROUND(EXTRACT(EPOCH FROM (TO_TIMESTAMP(MIN(dt)) - NOW()))/60, 0) as minutes_away,
  CASE
    WHEN MIN(dt) - EXTRACT(EPOCH FROM NOW())::bigint < 300 THEN
      '🚨 URGENT: Close windows now!'
    WHEN MIN(dt) - EXTRACT(EPOCH FROM NOW())::bigint < 600 THEN
      '⚠️ Rain in <10 min - prepare to close windows'
    ELSE '✅ No immediate action needed'
  END as alert
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND precipitation > 0;
```

## Caching Strategy

Updates every 10 minutes, cache accordingly:

```sql
-- Create materialized view
CREATE MATERIALIZED VIEW mv_minutely_forecast_cache AS
SELECT *
FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every 10 minutes
SELECT cron.schedule('refresh-minutely-forecast', '*/10 * * * *',
  'REFRESH MATERIALIZED VIEW mv_minutely_forecast_cache');
```

**Benefits:**
- Reduces API calls from ~6/hour to 6/hour (no additional savings, but faster queries)
- Sub-millisecond query response vs 1-2 seconds

## Troubleshooting

### No Results Returned

**Common Causes:**
1. **Location not supported** - Minutely forecast only available in select regions
2. **Invalid lat/lon** - Check coordinate ranges
3. **API key issue** - Verify server configuration

```sql
-- Test with known working location (Berlin)
SELECT COUNT(*) FROM fdw_open_weather.minutely_forecast
WHERE lat = 52.52 AND lon = 13.405;
-- Should return ~60 rows
```

### All Precipitation Values Zero

This is normal - it means no rain expected in next hour.

### Irregular Row Count

May return 59, 60, or 61 rows depending on API response timing. This is expected.

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Current Weather](current-weather.md)** - Current conditions
- **[Hourly Forecast](hourly-forecast.md)** - 48-hour forecast with precipitation probability
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
