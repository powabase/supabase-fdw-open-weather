# Daily Summary Endpoint

**Endpoint:** `daily_summary`
**Status:** ✅ Production Ready (v0.2.0)
**API:** https://api.openweathermap.org/data/3.0/onecall/day_summary
**Section:** Aggregated daily weather statistics

Returns aggregated weather statistics for a specific date, covering 46+ years of historical data and 1.5 years of future forecasts.

## Schema

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
```

## Columns

| Column | Type | Description | Units (metric) | Nullable |
|--------|------|-------------|----------------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees (-90 to 90) | No |
| `lon` | numeric | Longitude of location | Decimal degrees (-180 to 180) | No |
| `tz` | text | Timezone offset | Format: +/-HH:MM (e.g., "+01:00") | No |
| `date` | text | Date of summary | Format: YYYY-MM-DD | No |
| `units` | text | Unit system used | "metric", "imperial", or "standard" | No |
| `temp_min` | numeric | Minimum temperature | °C (metric), °F (imperial), K (standard) | No |
| `temp_max` | numeric | Maximum temperature | °C (metric), °F (imperial), K (standard) | No |
| `temp_morning` | numeric | Morning temperature (06:00) | °C (metric), °F (imperial), K (standard) | No |
| `temp_afternoon` | numeric | Afternoon temperature (12:00) | °C (metric), °F (imperial), K (standard) | No |
| `temp_evening` | numeric | Evening temperature (18:00) | °C (metric), °F (imperial), K (standard) | No |
| `temp_night` | numeric | Night temperature (00:00) | °C (metric), °F (imperial), K (standard) | No |
| `cloud_cover_afternoon` | numeric | Afternoon cloud coverage | % (0-100) | No |
| `humidity_afternoon` | numeric | Afternoon relative humidity | % (0-100) | No |
| `pressure_afternoon` | numeric | Afternoon atmospheric pressure | hPa (hectopascal) | No |
| `precipitation_total` | numeric | Total precipitation | mm (metric), inches (imperial) | No |
| `wind_max_speed` | numeric | Maximum wind speed | m/s (metric), mph (imperial) | No |
| `wind_max_direction` | numeric | Wind direction at max speed | Degrees (meteorological, 0=North) | No |

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |
| `date` | text | **Yes** | - | Date in YYYY-MM-DD format |
| `tz` | text | No | UTC | Timezone offset (+/-HHMM) |

### Optional Parameters (Server OPTIONS)
- `units`: "metric" (default), "imperial", or "standard"
- `lang`: Language code (default: "en")

## Basic Usage

```sql
-- Daily summary for Berlin on January 15, 2024
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52
  AND lon = 13.405
  AND date = '2024-01-15';

-- Daily summary with formatted output
SELECT
  date,
  ROUND(temp_min, 1) || '°C' as min_temp,
  ROUND(temp_max, 1) || '°C' as max_temp,
  ROUND(temp_afternoon, 1) || '°C' as afternoon_temp,
  ROUND(precipitation_total, 1) || ' mm' as rainfall,
  ROUND(wind_max_speed, 1) || ' m/s' as max_wind
FROM fdw_open_weather.daily_summary
WHERE lat = 52.52
  AND lon = 13.405
  AND date = '2024-01-15';
```

## Advanced Queries

### Historical Weather Analysis

```sql
-- Analyze temperature trends over multiple days
WITH daily_temps AS (
  SELECT
    date,
    temp_min,
    temp_max,
    temp_afternoon
  FROM fdw_open_weather.daily_summary
  WHERE lat = 52.52
    AND lon = 13.405
    AND date BETWEEN '2024-01-01' AND '2024-01-31'
)
SELECT
  AVG(temp_min) as avg_min_temp,
  AVG(temp_max) as avg_max_temp,
  MAX(temp_max) as highest_temp,
  MIN(temp_min) as lowest_temp
FROM daily_temps;
```

### Agricultural Planning

```sql
-- Find days suitable for outdoor work (no rain, moderate temps)
SELECT
  date,
  temp_min,
  temp_max,
  precipitation_total,
  wind_max_speed,
  CASE
    WHEN precipitation_total = 0
      AND temp_max BETWEEN 15 AND 25
      AND wind_max_speed < 10
    THEN 'Ideal'
    WHEN precipitation_total < 2
      AND temp_max BETWEEN 10 AND 30
    THEN 'Good'
    ELSE 'Poor'
  END as work_conditions
FROM fdw_open_weather.daily_summary
WHERE lat = 40.7128
  AND lon = -74.0060
  AND date BETWEEN '2024-06-01' AND '2024-06-30'
ORDER BY date;
```

### Energy Demand Forecasting

```sql
-- Daily weather impact on energy consumption
SELECT
  date,
  temp_afternoon as peak_temp,
  temp_night as low_temp,
  -- Cooling degree days (base 18°C)
  GREATEST(0, temp_afternoon - 18) as cooling_degree_day,
  -- Heating degree days (base 18°C)
  GREATEST(0, 18 - temp_night) as heating_degree_day,
  cloud_cover_afternoon,
  wind_max_speed
FROM fdw_open_weather.daily_summary
WHERE lat = 52.52
  AND lon = 13.405
  AND date BETWEEN '2024-01-01' AND '2024-12-31'
ORDER BY date;
```

### Climate Comparison

```sql
-- Compare same date across multiple years
SELECT
  EXTRACT(YEAR FROM date::date) as year,
  AVG(temp_max) as avg_high,
  AVG(temp_min) as avg_low,
  SUM(precipitation_total) as total_precip
FROM fdw_open_weather.daily_summary
WHERE lat = 52.52
  AND lon = 13.405
  AND EXTRACT(MONTH FROM date::date) = 1  -- January
GROUP BY EXTRACT(YEAR FROM date::date)
ORDER BY year DESC
LIMIT 10;
```

## Use Cases

### 1. Historical Weather Research
Query decades of daily aggregated data for climate studies:
```sql
-- Get daily summaries for entire year 1979
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 40.7128
  AND lon = -74.0060
  AND date >= '1979-01-02'
  AND date <= '1979-12-31';
```

### 2. Agricultural Planning
Determine optimal planting/harvesting dates:
```sql
-- Find last frost date (temp_min < 0)
SELECT date, temp_min
FROM fdw_open_weather.daily_summary
WHERE lat = 48.8566
  AND lon = 2.3522
  AND date BETWEEN '2024-03-01' AND '2024-05-31'
  AND temp_min < 0
ORDER BY date DESC
LIMIT 1;
```

### 3. Construction Scheduling
Identify suitable weather windows:
```sql
-- Find dry days with moderate temperatures
SELECT date
FROM fdw_open_weather.daily_summary
WHERE lat = 51.5074
  AND lon = -0.1278
  AND date BETWEEN '2024-06-01' AND '2024-09-30'
  AND precipitation_total < 1
  AND temp_max BETWEEN 15 AND 28
  AND wind_max_speed < 15
ORDER BY date;
```

### 4. Event Planning
Assess weather risks for outdoor events:
```sql
-- Weather summary for specific event date
SELECT
  date,
  CASE
    WHEN precipitation_total = 0 AND temp_max BETWEEN 18 AND 26
    THEN 'Perfect'
    WHEN precipitation_total < 2 AND temp_max BETWEEN 15 AND 30
    THEN 'Good'
    WHEN precipitation_total < 5
    THEN 'Fair'
    ELSE 'Poor'
  END as event_weather_rating,
  temp_min || '-' || temp_max || '°C' as temp_range,
  precipitation_total || ' mm' as rain,
  wind_max_speed || ' m/s' as max_wind
FROM fdw_open_weather.daily_summary
WHERE lat = 37.7749
  AND lon = -122.4194
  AND date = '2024-08-15';
```

## Data Coverage

- **Historical:** From January 2, 1979 to present
- **Forecast:** Up to 1.5 years in the future
- **Update Frequency:** Daily after day completion
- **Geographic Coverage:** Worldwide

## Performance Tips

### 1. Use Materialized Views for Repeated Queries

```sql
-- Cache historical data that won't change
CREATE MATERIALIZED VIEW mv_berlin_jan_2024 AS
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52
  AND lon = 13.405
  AND date BETWEEN '2024-01-01' AND '2024-01-31';

-- Query the cached data
SELECT * FROM mv_berlin_jan_2024;
```

### 2. Limit Date Ranges

```sql
-- Bad: Querying entire year requires 365 API calls
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405
  AND date >= '2024-01-01' AND date <= '2024-12-31';

-- Good: Query specific dates only when needed
SELECT * FROM fdw_open_weather.daily_summary
WHERE lat = 52.52 AND lon = 13.405
  AND date IN ('2024-01-15', '2024-02-15', '2024-03-15');
```

### 3. Batch Location Queries

```sql
-- Query multiple locations efficiently
SELECT l.city_name, d.*
FROM locations l
CROSS JOIN LATERAL (
  SELECT * FROM fdw_open_weather.daily_summary
  WHERE lat = l.lat AND lon = l.lon AND date = '2024-01-15'
) d;
```

## Troubleshooting

### Missing date Parameter

**Error:**
```
date parameter required for daily_summary (YYYY-MM-DD format)
```

**Solution:**
```sql
-- Include date in WHERE clause
WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-15'
```

### Invalid Date Format

**Error:**
```
Invalid date format
```

**Solution:**
Use YYYY-MM-DD format exactly:
```sql
-- Correct
WHERE date = '2024-01-15'

-- Incorrect
WHERE date = '2024/01/15'  -- Wrong separator
WHERE date = '15-01-2024'  -- Wrong order
```

### Date Out of Range

**Error:**
```
data not found
```

**Possible causes:**
- Date before 1979-01-02 (earliest available)
- Date too far in future (> 1.5 years)
- Invalid date (e.g., '2024-02-30')

## Related Endpoints

- **[historical_weather](historical-weather.md)** - Hourly historical data for specific timestamp
- **[daily_forecast](daily-forecast.md)** - 8-day daily forecast (future only)
- **[current_weather](current-weather.md)** - Real-time conditions

## API Rate Limits

- **Free Plan:** 1,000 calls/day
- **Note:** Each query = 1 API call
- **Recommendation:** Cache results with materialized views

## See Also

- [OpenWeather One Call API 3.0 Documentation](https://openweathermap.org/api/one-call-3#history_daily_aggregation)
- [API Overview](../reference/API_OVERVIEW.md)
- [Troubleshooting Guide](../guides/TROUBLESHOOTING.md)
