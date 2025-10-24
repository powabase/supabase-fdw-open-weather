# SQL Examples

Comprehensive SQL query patterns for OpenWeather WASM FDW.

## Quick Reference

| Endpoint | Rows | Key Parameters | Use Case |
|----------|------|----------------|----------|
| [current_weather](#current-weather) | 1 | lat, lon | Real-time conditions |
| [minutely_forecast](#minutely-forecast) | 60 | lat, lon | Rain in next hour |
| [hourly_forecast](#hourly-forecast) | 48 | lat, lon | 48-hour forecast |
| [daily_forecast](#daily-forecast) | 8 | lat, lon | 8-day forecast |
| [weather_alerts](#weather-alerts) | 0-N | lat, lon | Active warnings |
| [historical_weather](#historical-weather) | 1 | lat, lon, dt | Past weather |
| [daily_summary](#daily-summary) | 1 | lat, lon, date | Daily statistics |
| [weather_overview](#weather-overview) | 1 | lat, lon | AI summary |

---

## Current Weather

### Basic Query

```sql
SELECT
  timezone,
  TO_TIMESTAMP(dt) as observation_time,
  temp as temperature_celsius,
  feels_like,
  humidity,
  pressure,
  wind_speed,
  weather_main,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;  -- Berlin
```

### Temperature Conversion

```sql
SELECT
  timezone,
  temp as celsius,
  ROUND((temp * 9/5) + 32, 1) as fahrenheit,
  ROUND(temp + 273.15, 1) as kelvin,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 40.7128 AND lon = -74.0060;  -- New York
```

### Multiple Locations

```sql
-- Union multiple queries for different cities
SELECT 'Berlin' as city, temp, weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405

UNION ALL

SELECT 'London' as city, temp, weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 51.5074 AND lon = -0.1278

UNION ALL

SELECT 'Paris' as city, temp, weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 48.8566 AND lon = 2.3522;
```

### Weather Conditions

```sql
SELECT
  timezone,
  temp,
  CASE
    WHEN temp < 0 THEN '🥶 Freezing'
    WHEN temp < 10 THEN '❄️ Cold'
    WHEN temp < 20 THEN '😊 Comfortable'
    WHEN temp < 30 THEN '☀️ Warm'
    ELSE '🔥 Hot'
  END as comfort_level,
  humidity,
  CASE
    WHEN humidity < 30 THEN 'Dry'
    WHEN humidity < 60 THEN 'Comfortable'
    ELSE 'Humid'
  END as humidity_level
FROM fdw_open_weather.current_weather
WHERE lat = 25.7617 AND lon = -80.1918;  -- Miami
```

---

## Minutely Forecast

### Next Hour Precipitation

```sql
SELECT
  TO_TIMESTAMP(dt) as time,
  precipitation as mm_per_hour,
  CASE
    WHEN precipitation = 0 THEN 'No rain'
    WHEN precipitation < 2.5 THEN 'Light rain'
    WHEN precipitation < 10 THEN 'Moderate rain'
    ELSE 'Heavy rain'
  END as intensity
FROM fdw_open_weather.minutely_forecast
WHERE lat = 51.5074 AND lon = -0.1278  -- London
ORDER BY dt
LIMIT 10;
```

### Rain Probability

```sql
SELECT
  COUNT(*) as total_minutes,
  SUM(CASE WHEN precipitation > 0 THEN 1 ELSE 0 END) as rainy_minutes,
  ROUND(
    100.0 * SUM(CASE WHEN precipitation > 0 THEN 1 ELSE 0 END) / COUNT(*),
    1
  ) as rain_percentage
FROM fdw_open_weather.minutely_forecast
WHERE lat = 48.8566 AND lon = 2.3522;  -- Paris
```

---

## Hourly Forecast

### 24-Hour Forecast

```sql
SELECT
  TO_TIMESTAMP(dt) as forecast_time,
  temp,
  ROUND(pop * 100, 0) as rain_probability_pct,
  COALESCE(rain_1h, 0) as rain_mm,
  COALESCE(snow_1h, 0) as snow_mm,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 35.6762 AND lon = 139.6503  -- Tokyo
ORDER BY dt
LIMIT 24;
```

### Temperature Trends

```sql
SELECT
  TO_TIMESTAMP(dt)::date as forecast_date,
  MIN(temp) as min_temp,
  MAX(temp) as max_temp,
  ROUND(AVG(temp), 1) as avg_temp,
  MAX(temp) - MIN(temp) as temp_range
FROM fdw_open_weather.hourly_forecast
WHERE lat = 37.7749 AND lon = -122.4194  -- San Francisco
GROUP BY TO_TIMESTAMP(dt)::date
ORDER BY forecast_date;
```

### Outdoor Activity Planning

```sql
SELECT
  TO_TIMESTAMP(dt) as time,
  temp,
  pop * 100 as rain_prob_pct,
  wind_speed,
  uvi,
  CASE
    WHEN pop < 0.2 AND temp BETWEEN 15 AND 25 AND wind_speed < 5 AND uvi < 7
      THEN '✅ Perfect for outdoor activity'
    WHEN pop < 0.3 AND temp BETWEEN 10 AND 30
      THEN '⚠️ Good, but watch conditions'
    ELSE '❌ Not ideal for outdoor activity'
  END as recommendation
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405  -- Berlin
  AND TO_TIMESTAMP(dt) BETWEEN NOW() AND NOW() + INTERVAL '12 hours'
ORDER BY dt;
```

---

## Daily Forecast

### Week Ahead

```sql
SELECT
  TO_TIMESTAMP(dt)::date as date,
  TO_CHAR(TO_TIMESTAMP(dt), 'Day') as day_of_week,
  temp_min,
  temp_max,
  ROUND((temp_max + temp_min) / 2, 1) as avg_temp,
  ROUND(pop * 100, 0) as rain_prob_pct,
  weather_description
FROM fdw_open_weather.daily_forecast
WHERE lat = 40.7128 AND lon = -74.0060  -- New York
ORDER BY dt;
```

### Sunrise/Sunset Times

```sql
SELECT
  TO_TIMESTAMP(dt)::date as date,
  TO_TIMESTAMP(sunrise) as sunrise,
  TO_TIMESTAMP(sunset) as sunset,
  EXTRACT(EPOCH FROM (TO_TIMESTAMP(sunset) - TO_TIMESTAMP(sunrise))) / 3600 as daylight_hours,
  moon_phase,
  CASE
    WHEN moon_phase = 0 THEN '🌑 New Moon'
    WHEN moon_phase < 0.25 THEN '🌒 Waxing Crescent'
    WHEN moon_phase = 0.25 THEN '🌓 First Quarter'
    WHEN moon_phase < 0.5 THEN '🌔 Waxing Gibbous'
    WHEN moon_phase = 0.5 THEN '🌕 Full Moon'
    WHEN moon_phase < 0.75 THEN '🌖 Waning Gibbous'
    WHEN moon_phase = 0.75 THEN '🌗 Last Quarter'
    ELSE '🌘 Waning Crescent'
  END as moon_phase_name
FROM fdw_open_weather.daily_forecast
WHERE lat = 51.5074 AND lon = -0.1278  -- London
ORDER BY dt;
```

### Rain Forecast

```sql
SELECT
  TO_TIMESTAMP(dt)::date as date,
  temp_max,
  ROUND(pop * 100, 0) as rain_probability_pct,
  COALESCE(rain, 0) as expected_rain_mm,
  CASE
    WHEN pop < 0.2 THEN '☀️ No rain expected'
    WHEN pop < 0.5 THEN '🌤️ Slight chance of rain'
    WHEN pop < 0.8 THEN '🌧️ Rain likely'
    ELSE '⛈️ Rain very likely'
  END as rain_forecast
FROM fdw_open_weather.daily_forecast
WHERE lat = 48.8566 AND lon = 2.3522  -- Paris
ORDER BY dt;
```

---

## Weather Alerts

### Active Alerts

```sql
SELECT
  sender_name,
  event,
  TO_TIMESTAMP(start) as alert_start,
  TO_TIMESTAMP("end") as alert_end,
  EXTRACT(EPOCH FROM (TO_TIMESTAMP("end") - TO_TIMESTAMP(start))) / 3600 as duration_hours,
  description,
  tags
FROM fdw_open_weather.weather_alerts
WHERE lat = 25.7617 AND lon = -80.1918  -- Miami (hurricane zone)
ORDER BY start DESC;
```

### Alert Summary

```sql
SELECT
  event as alert_type,
  COUNT(*) as alert_count,
  MIN(TO_TIMESTAMP(start)) as earliest,
  MAX(TO_TIMESTAMP("end")) as latest
FROM fdw_open_weather.weather_alerts
WHERE lat = 35.6762 AND lon = 139.6503  -- Tokyo (typhoon zone)
GROUP BY event;
```

---

## Historical Weather

### Historical Comparison

```sql
-- Weather on specific historical dates
SELECT
  '2024-01-01' as date,
  temp,
  weather_main,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405
  AND dt = 1704067200  -- Jan 1, 2024 00:00:00 UTC

UNION ALL

SELECT
  '2024-07-01' as date,
  temp,
  weather_main,
  weather_description
FROM fdw_open_weather.historical_weather
WHERE lat = 52.52 AND lon = 13.405
  AND dt = 1719792000;  -- July 1, 2024 00:00:00 UTC
```

### Historical Analysis

```sql
-- Store results in temp table for analysis
WITH historical_data AS (
  SELECT
    TO_TIMESTAMP(dt) as observation_time,
    temp,
    humidity,
    pressure,
    wind_speed
  FROM fdw_open_weather.historical_weather
  WHERE lat = 40.7128 AND lon = -74.0060
    AND dt = 1704067200  -- Specific timestamp
)
SELECT
  observation_time,
  temp,
  humidity,
  CASE
    WHEN temp < 0 THEN 'Below freezing'
    WHEN temp < 15 THEN 'Cool'
    WHEN temp < 25 THEN 'Moderate'
    ELSE 'Warm'
  END as temp_category
FROM historical_data;
```

---

## Daily Summary

### Daily Aggregates

```sql
SELECT
  date,
  temp_min,
  temp_max,
  temp_max - temp_min as temp_range,
  temp_afternoon,
  humidity_afternoon,
  precipitation_total,
  wind_max_speed,
  CASE
    WHEN precipitation_total = 0 THEN 'Dry day'
    WHEN precipitation_total < 5 THEN 'Light rain'
    WHEN precipitation_total < 15 THEN 'Moderate rain'
    ELSE 'Heavy rain'
  END as rain_summary
FROM fdw_open_weather.daily_summary
WHERE lat = 37.7749 AND lon = -122.4194  -- San Francisco
  AND date = '2024-01-15';
```

### Temperature Distribution

```sql
SELECT
  date,
  temp_morning as "06:00-12:00",
  temp_afternoon as "12:00-18:00",
  temp_evening as "18:00-00:00",
  temp_night as "00:00-06:00",
  GREATEST(temp_morning, temp_afternoon, temp_evening, temp_night) as warmest_period_temp,
  LEAST(temp_morning, temp_afternoon, temp_evening, temp_night) as coolest_period_temp
FROM fdw_open_weather.daily_summary
WHERE lat = 51.5074 AND lon = -0.1278  -- London
  AND date = '2024-06-15';
```

---

## Weather Overview

### AI Summary

```sql
SELECT
  date,
  weather_overview as summary
FROM fdw_open_weather.weather_overview
WHERE lat = 35.6762 AND lon = 139.6503;  -- Tokyo
```

### Combined with Forecast

```sql
SELECT
  'AI Overview' as source,
  wo.weather_overview as summary
FROM fdw_open_weather.weather_overview wo
WHERE wo.lat = 52.52 AND wo.lon = 13.405

UNION ALL

SELECT
  'Daily Forecast' as source,
  'High: ' || df.temp_max || '°C, Low: ' || df.temp_min || '°C, ' || df.weather_description as summary
FROM fdw_open_weather.daily_forecast df
WHERE df.lat = 52.52 AND df.lon = 13.405
ORDER BY source;
```

---

## Advanced Queries

### Weather Dashboard

```sql
WITH current AS (
  SELECT temp, humidity, weather_description
  FROM fdw_open_weather.current_weather
  WHERE lat = 40.7128 AND lon = -74.0060
),
today_forecast AS (
  SELECT temp_min, temp_max, pop
  FROM fdw_open_weather.daily_forecast
  WHERE lat = 40.7128 AND lon = -74.0060
  ORDER BY dt
  LIMIT 1
)
SELECT
  c.temp as current_temp,
  c.weather_description,
  c.humidity,
  tf.temp_min as today_min,
  tf.temp_max as today_max,
  ROUND(tf.pop * 100, 0) as rain_probability_pct
FROM current c
CROSS JOIN today_forecast tf;
```

### Multi-Day Analysis

```sql
SELECT
  TO_TIMESTAMP(dt)::date as date,
  temp_max,
  temp_min,
  SUM(temp_max) OVER (ORDER BY dt ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) / 3 as three_day_avg_max,
  CASE
    WHEN temp_max > LAG(temp_max) OVER (ORDER BY dt) THEN '↗️ Warming'
    WHEN temp_max < LAG(temp_max) OVER (ORDER BY dt) THEN '↘️ Cooling'
    ELSE '→ Stable'
  END as trend
FROM fdw_open_weather.daily_forecast
WHERE lat = 48.8566 AND lon = 2.3522  -- Paris
ORDER BY dt;
```

### Precipitation Timeline

```sql
SELECT
  'Next Hour' as timeframe,
  COALESCE(SUM(precipitation), 0) as total_mm
FROM fdw_open_weather.minutely_forecast
WHERE lat = 51.5074 AND lon = -0.1278

UNION ALL

SELECT
  'Next 24 Hours' as timeframe,
  COALESCE(SUM(rain_1h), 0) as total_mm
FROM fdw_open_weather.hourly_forecast
WHERE lat = 51.5074 AND lon = -0.1278
  AND dt < EXTRACT(EPOCH FROM NOW() + INTERVAL '24 hours')

UNION ALL

SELECT
  'Next 7 Days' as timeframe,
  COALESCE(SUM(rain), 0) as total_mm
FROM fdw_open_weather.daily_forecast
WHERE lat = 51.5074 AND lon = -0.1278
  AND dt < EXTRACT(EPOCH FROM NOW() + INTERVAL '7 days');
```

---

## Best Practices

### Always Include Coordinates

```sql
-- ❌ Wrong - missing WHERE clause
SELECT * FROM fdw_open_weather.current_weather;

-- ✅ Correct - includes lat/lon
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Use Proper Timestamp Conversions

```sql
-- Convert Unix timestamp to readable datetime
SELECT
  dt,
  TO_TIMESTAMP(dt) as datetime,
  TO_TIMESTAMP(dt)::date as date_only,
  TO_TIMESTAMP(dt)::time as time_only
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Handle NULL Values

```sql
-- Rain/snow may be NULL when not present
SELECT
  TO_TIMESTAMP(dt) as time,
  temp,
  COALESCE(rain_1h, 0) as rain_mm,
  COALESCE(snow_1h, 0) as snow_mm,
  CASE
    WHEN rain_1h IS NULL AND snow_1h IS NULL THEN 'No precipitation'
    WHEN rain_1h > 0 THEN 'Rain'
    WHEN snow_1h > 0 THEN 'Snow'
    ELSE 'Dry'
  END as conditions
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 12;
```

---

## Common Use Cases

### Travel Planning

```sql
SELECT
  TO_TIMESTAMP(dt)::date as travel_date,
  temp_max as high_temp,
  ROUND(pop * 100, 0) as rain_chance_pct,
  CASE
    WHEN pop < 0.2 AND temp_max BETWEEN 15 AND 28 THEN '✅ Great for travel'
    WHEN pop < 0.4 AND temp_max BETWEEN 10 AND 32 THEN '⚠️ OK for travel'
    ELSE '❌ Consider rescheduling'
  END as recommendation
FROM fdw_open_weather.daily_forecast
WHERE lat = 25.7617 AND lon = -80.1918  -- Destination
ORDER BY dt;
```

### Energy Demand Forecasting

```sql
SELECT
  TO_TIMESTAMP(dt) as hour,
  temp,
  CASE
    WHEN temp < 10 THEN 'High heating demand'
    WHEN temp > 30 THEN 'High cooling demand'
    ELSE 'Normal demand'
  END as energy_forecast,
  wind_speed  -- For wind power generation
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Agricultural Planning

```sql
SELECT
  TO_TIMESTAMP(dt)::date as date,
  temp_min,
  temp_max,
  precipitation_total,
  CASE
    WHEN temp_min < 0 THEN '⚠️ Frost risk'
    WHEN precipitation_total > 20 THEN '⚠️ Heavy rain - delay planting'
    WHEN temp_max > 35 THEN '⚠️ Heat stress risk'
    ELSE '✅ Normal conditions'
  END as farming_alert
FROM fdw_open_weather.daily_summary
WHERE lat = 37.7749 AND lon = -122.4194
  AND date BETWEEN '2024-06-01' AND '2024-06-07';
```

---

## Reference

- **Quickstart Guide:** [QUICKSTART.md](../../QUICKSTART.md)
- **Deployment Guide:** [DEPLOYMENT_GUIDE.md](../guides/DEPLOYMENT_GUIDE.md)
- **OpenWeather API:** https://openweathermap.org/api/one-call-3
- **Endpoint Docs:** [docs/endpoints/](../endpoints/)

**Last Updated:** October 24, 2025
