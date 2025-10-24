# Historical Weather Endpoint

**Endpoint:** `historical_weather`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall/timemachine
**Section:** `data[0]` from `/onecall/timemachine` response

Point-in-time historical weather data from January 1, 1979 onwards. Returns actual weather conditions for any specific timestamp in the past.

## Schema

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
```

## Columns

| Column | Type | Description | Units (metric) | Nullable |
|--------|------|-------------|----------------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees | No |
| `lon` | numeric | Longitude of location | Decimal degrees | No |
| `dt` | bigint | Historical timestamp | Unix timestamp (UTC) | No |
| `temp` | numeric | Temperature | °C / °F / K | No |
| `feels_like` | numeric | Perceived temperature | °C / °F / K | No |
| `pressure` | bigint | Atmospheric pressure | hPa | No |
| `humidity` | bigint | Humidity | % (0-100) | No |
| `dew_point` | numeric | Dew point temperature | °C / °F / K | No |
| `clouds` | bigint | Cloudiness | % (0-100) | No |
| `visibility` | bigint | Visibility | meters | No |
| `wind_speed` | numeric | Wind speed | m/s / mph | No |
| `wind_deg` | bigint | Wind direction | Degrees (0=North) | No |
| `weather_main` | text | Weather category | "Clear", "Rain", etc. | No |
| `weather_description` | text | Detailed description | "clear sky", etc. | No |
| `weather_icon` | text | Weather icon code | "01d", "10n", etc. | No |

**Note:** Fewer fields than current weather (no UV index, wind gusts not always available).

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |
| `dt` | bigint | **Yes** | - | Unix timestamp (must be in past) |

### Optional Parameters (Server OPTIONS)
- `units`: "metric" (default), "imperial", or "standard"
- `lang`: Language code (default: "en")

### Date Range
- **Earliest:** January 1, 1979
- **Latest:** 5 days ago (use current weather or hourly forecast for more recent data)

## Basic Usage

```sql
-- Weather on January 1, 2024 at midnight UTC (Berlin)
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;

-- Historical weather with readable date
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  weather_main,
  weather_description,
  humidity,
  ROUND(wind_speed, 1) as wind_ms
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;

-- Weather exactly one year ago
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52
  AND lon = 13.405
  AND dt = EXTRACT(EPOCH FROM (CURRENT_DATE - INTERVAL '1 year'))::bigint;
```

## Advanced Queries

### Compare Historical with Current Forecast

```sql
-- Compare today's forecast with same date last year
WITH forecast_today AS (
  SELECT
    temp_max,
    temp_min,
    weather_main
  FROM fdw_open_weather.daily_forecast
  WHERE lat = 52.52 AND lon = 13.405
  LIMIT 1
),
historical_last_year AS (
  SELECT
    temp,
    weather_main
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = EXTRACT(EPOCH FROM (CURRENT_DATE - INTERVAL '1 year'))::bigint
)
SELECT
  ROUND(f.temp_max, 1) as forecast_max_c,
  ROUND(f.temp_min, 1) as forecast_min_c,
  f.weather_main as forecast_conditions,
  ROUND(h.temp, 1) as last_year_temp_c,
  h.weather_main as last_year_conditions,
  ROUND(f.temp_max - h.temp, 1) as temp_difference_c
FROM forecast_today f, historical_last_year h;
```

### Historical Temperature Analysis

```sql
-- Query multiple historical dates (create temp table first)
CREATE TEMP TABLE historical_dates (
  query_date DATE,
  unix_timestamp BIGINT
);

INSERT INTO historical_dates
SELECT
  date,
  EXTRACT(EPOCH FROM date)::bigint
FROM generate_series(
  '2024-01-01'::date,
  '2024-01-07'::date,
  '1 day'::interval
) AS date;

-- Get historical weather for all dates
SELECT
  d.query_date,
  ROUND(h.temp, 1) as temp_c,
  h.weather_main,
  h.humidity,
  ROUND(h.wind_speed, 1) as wind_ms
FROM historical_dates d
CROSS JOIN LATERAL (
  SELECT *
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = d.unix_timestamp
) h
ORDER BY d.query_date;
```

### Year-over-Year Comparison

```sql
-- Compare same date across multiple years
WITH target_date_time AS (
  -- June 15 at noon for last 5 years
  SELECT
    EXTRACT(YEAR FROM CURRENT_DATE) - n as year,
    EXTRACT(EPOCH FROM (
      (EXTRACT(YEAR FROM CURRENT_DATE) - n)::text || '-06-15 12:00:00'
    )::timestamp)::bigint as timestamp
  FROM generate_series(1, 5) as n
)
SELECT
  t.year,
  ROUND(h.temp, 1) as temp_c,
  h.weather_main,
  h.weather_description,
  h.humidity
FROM target_date_time t
CROSS JOIN LATERAL (
  SELECT *
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = t.timestamp
) h
ORDER BY t.year DESC;
```

### Birthday Weather Lookup

```sql
-- What was the weather on your birthday?
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as date_time,
  ROUND(temp, 1) as temp_c,
  weather_main,
  weather_description,
  humidity as humidity_pct,
  ROUND(wind_speed, 1) as wind_ms,
  CASE
    WHEN weather_main IN ('Clear', 'Clouds') AND temp BETWEEN 15 AND 25
      THEN '🎉 Beautiful birthday weather!'
    WHEN weather_main IN ('Rain', 'Thunderstorm')
      THEN '🌧️ Rainy birthday'
    WHEN weather_main = 'Snow'
      THEN '❄️ Snowy birthday'
    WHEN temp > 30
      THEN '🔥 Hot birthday'
    WHEN temp < 5
      THEN '🥶 Cold birthday'
    ELSE '🌤️ Typical birthday weather'
  END as birthday_weather
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52
  AND lon = 13.405
  AND dt = EXTRACT(EPOCH FROM '1990-06-15 12:00:00'::timestamp)::bigint;
```

### Historical Event Weather

```sql
-- Weather during specific historical events
WITH historical_events AS (
  SELECT
    'New Year 2024' as event_name,
    EXTRACT(EPOCH FROM '2024-01-01 00:00:00'::timestamp)::bigint as event_time
  UNION ALL
  SELECT
    'Summer Solstice 2023',
    EXTRACT(EPOCH FROM '2023-06-21 12:00:00'::timestamp)::bigint
  UNION ALL
  SELECT
    'Halloween 2023',
    EXTRACT(EPOCH FROM '2023-10-31 18:00:00'::timestamp)::bigint
)
SELECT
  e.event_name,
  TO_TIMESTAMP(h.dt) AT TIME ZONE 'UTC' as time,
  ROUND(h.temp, 1) as temp_c,
  h.weather_description,
  h.humidity
FROM historical_events e
CROSS JOIN LATERAL (
  SELECT *
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = e.event_time
) h;
```

## Response Characteristics

- **Rows:** Always 1 row per query
- **Data Range:** January 1, 1979 to 5 days ago
- **Response Size:** ~2-3 KB
- **Query Time:** < 2 seconds
- **Worldwide Availability:** Yes
- **Granularity:** Hourly data points

## Data Quality Notes

### Timestamp Requirements

**Must be in the past:**
- Minimum: `315532800` (Jan 1, 1979 00:00:00 UTC)
- Maximum: 5 days ago from current date

**Error if:**
- `dt` is in the future
- `dt` is within last 5 days
- `dt` is before 1979

### Converting Dates to Timestamps

```sql
-- Using PostgreSQL EXTRACT
SELECT EXTRACT(EPOCH FROM '2024-01-01 00:00:00'::timestamp)::bigint;
-- Returns: 1704067200

-- Using TO_TIMESTAMP reverse
SELECT TO_TIMESTAMP(1704067200);
-- Returns: 2024-01-01 00:00:00+00

-- Current time minus 1 year
SELECT EXTRACT(EPOCH FROM (NOW() - INTERVAL '1 year'))::bigint;
```

### Missing vs Limited Data

**Compared to current weather, historical lacks:**
- UV index
- Rain/snow volumes
- Wind gusts (not always available)

This is expected - historical data has fewer fields than real-time forecasts.

### Timezone Handling

All timestamps are UTC. For local time comparisons:

```sql
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as utc_time,
  TO_TIMESTAMP(dt) AT TIME ZONE 'Europe/Berlin' as local_time,
  ROUND(temp, 1) as temp_c
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52
  AND lon = 13.405
  AND dt = 1704067200;
```

## Common Use Cases

### Climate Research

```sql
-- Analyze historical temperature trends (monthly averages)
-- Note: Requires multiple queries or temp table with pre-calculated timestamps

CREATE TEMP TABLE monthly_snapshots AS
SELECT
  year,
  month,
  EXTRACT(EPOCH FROM (year || '-' || month || '-15 12:00:00')::timestamp)::bigint as timestamp
FROM (
  SELECT
    EXTRACT(YEAR FROM date)::text as year,
    LPAD(EXTRACT(MONTH FROM date)::text, 2, '0') as month
  FROM generate_series(
    '2020-01-01'::date,
    '2024-01-01'::date,
    '1 month'::interval
  ) AS date
) dates;

SELECT
  s.year,
  s.month,
  ROUND(AVG(h.temp), 1) as avg_temp_c,
  ROUND(AVG(h.humidity), 0) as avg_humidity_pct
FROM monthly_snapshots s
CROSS JOIN LATERAL (
  SELECT temp, humidity
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = s.timestamp
) h
GROUP BY s.year, s.month
ORDER BY s.year, s.month;
```

### Weather on Important Dates

```sql
-- Application: "What was the weather on your wedding day?"
CREATE TABLE user_events (
  user_id INT,
  event_name TEXT,
  event_date TIMESTAMP,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO user_events VALUES
  (1, 'Wedding', '2022-06-15 14:00:00', 52.52, 13.405),
  (2, 'First Date', '2019-03-20 19:00:00', 51.5074, -0.1278);

SELECT
  e.user_id,
  e.event_name,
  e.event_date,
  ROUND(h.temp, 1) as temp_c,
  h.weather_description,
  CASE
    WHEN h.weather_main = 'Clear' THEN '☀️ Perfect weather!'
    WHEN h.weather_main = 'Rain' THEN '🌧️ Rainy day'
    WHEN h.weather_main = 'Snow' THEN '❄️ Snowy day'
    ELSE '🌤️ ' || h.weather_main
  END as weather_summary
FROM user_events e
CROSS JOIN LATERAL (
  SELECT *
  FROM fdw_open_weather.historical_weather
  WHERE lat = e.lat
    AND lon = e.lon
    AND dt = EXTRACT(EPOCH FROM e.event_date)::bigint
) h;
```

### Historical vs Forecast Accuracy

```sql
-- Compare yesterday's forecast with actual historical weather
-- (Requires storing forecast data beforehand for comparison)

WITH yesterdays_forecast AS (
  -- Assume we saved yesterday's forecast to a table
  SELECT temp_max, temp_min, weather_main
  FROM saved_forecasts
  WHERE forecast_date = CURRENT_DATE - 1
    AND lat = 52.52 AND lon = 13.405
),
actual_weather AS (
  SELECT temp, weather_main
  FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52
    AND lon = 13.405
    AND dt = EXTRACT(EPOCH FROM (CURRENT_DATE - 1))::bigint
)
SELECT
  ROUND(f.temp_max, 1) as forecasted_max,
  ROUND(a.temp, 1) as actual_temp,
  ROUND(a.temp - f.temp_max, 1) as forecast_error_c,
  f.weather_main as forecast_conditions,
  a.weather_main as actual_conditions,
  CASE
    WHEN f.weather_main = a.weather_main THEN '✅ Accurate'
    ELSE '❌ Inaccurate'
  END as forecast_accuracy
FROM yesterdays_forecast f, actual_weather a;
```

## Performance Notes

### API Call Costs

Each unique `dt` value = 1 API call. To query multiple dates:

**Option 1: LATERAL JOIN (multiple API calls)**
```sql
-- Each date = 1 API call (total: N calls)
SELECT d.date, h.temp
FROM dates_table d
CROSS JOIN LATERAL (
  SELECT temp FROM fdw_open_weather.historical_weather
  WHERE lat = 52.52 AND lon = 13.405 AND dt = d.timestamp
) h;
```

**Option 2: Cache historical data**
```sql
-- Query once, store results
CREATE TABLE historical_weather_cache AS
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;
```

### Caching Strategy

Historical data never changes - cache indefinitely:

```sql
-- Create permanent cache table
CREATE TABLE historical_weather_archive (
  cached_at TIMESTAMP DEFAULT NOW(),
  lat NUMERIC,
  lon NUMERIC,
  dt BIGINT,
  temp NUMERIC,
  feels_like NUMERIC,
  pressure BIGINT,
  humidity BIGINT,
  dew_point NUMERIC,
  clouds BIGINT,
  visibility BIGINT,
  wind_speed NUMERIC,
  wind_deg BIGINT,
  weather_main TEXT,
  weather_description TEXT,
  weather_icon TEXT,
  UNIQUE(lat, lon, dt)
);

-- Function to get historical weather (checks cache first)
CREATE OR REPLACE FUNCTION get_historical_weather(
  p_lat NUMERIC,
  p_lon NUMERIC,
  p_dt BIGINT
) RETURNS TABLE (
  temp NUMERIC,
  weather_main TEXT,
  weather_description TEXT
) AS $$
BEGIN
  -- Check cache first
  RETURN QUERY
  SELECT h.temp, h.weather_main, h.weather_description
  FROM historical_weather_archive h
  WHERE h.lat = p_lat AND h.lon = p_lon AND h.dt = p_dt;

  -- If not in cache, query FDW and cache result
  IF NOT FOUND THEN
    INSERT INTO historical_weather_archive (lat, lon, dt, temp, weather_main, weather_description)
    SELECT lat, lon, dt, temp, weather_main, weather_description
    FROM fdw_open_weather.historical_weather
    WHERE lat = p_lat AND lon = p_lon AND dt = p_dt;

    RETURN QUERY
    SELECT h.temp, h.weather_main, h.weather_description
    FROM historical_weather_archive h
    WHERE h.lat = p_lat AND h.lon = p_lon AND h.dt = p_dt;
  END IF;
END;
$$ LANGUAGE plpgsql;
```

## Troubleshooting

### Error: "WHERE clause must include 'dt'"

The `dt` parameter is required. Example:

```sql
-- ❌ Wrong: Missing dt
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405;

-- ✅ Correct: Includes dt
SELECT * FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405 AND dt = 1704067200;
```

### Error: "dt must be in the past"

Cannot query future dates or very recent dates (< 5 days ago).

```sql
-- Use current weather instead
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Invalid Timestamp Format

Ensure `dt` is a Unix timestamp (bigint), not a date string:

```sql
-- ❌ Wrong: String date
WHERE dt = '2024-01-01'

-- ✅ Correct: Unix timestamp
WHERE dt = 1704067200

-- ✅ Correct: Converted from date
WHERE dt = EXTRACT(EPOCH FROM '2024-01-01'::timestamp)::bigint
```

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Current Weather](current-weather.md)** - Current conditions
- **[Daily Forecast](daily-forecast.md)** - 8-day forecast for comparison
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
