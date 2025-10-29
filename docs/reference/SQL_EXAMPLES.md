# SQL Examples

Essential query patterns for OpenWeather WASM FDW.

## Basic Queries (One Per Endpoint)

### Current Weather
```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;
```

### Minutely Forecast
```sql
SELECT forecast_time, precipitation_mm
FROM fdw_open_weather.minutely_forecast
WHERE latitude = 52.52 AND longitude = 13.405;
```

### Hourly Forecast
```sql
SELECT forecast_time, temp_celsius, precipitation_probability_pct
FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_time;
```

### Daily Forecast
```sql
SELECT forecast_date, temp_min_celsius, temp_max_celsius
FROM fdw_open_weather.daily_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_date;
```

### Weather Alerts
```sql
SELECT event, alert_start, alert_end, description
FROM fdw_open_weather.weather_alerts
WHERE latitude = 52.52 AND longitude = 13.405;
```

### Historical Weather (v0.3.1)
```sql
-- Native TIMESTAMPTZ support
SELECT observation_time, temp_celsius, weather_description
FROM fdw_open_weather.historical_weather
WHERE latitude = 52.52 AND longitude = 13.405
  AND observation_time = '2024-01-01 00:00:00+00';
```

### Daily Summary (v0.3.1)
```sql
SELECT summary_date, temp_min_celsius, temp_max_celsius, precipitation_mm
FROM fdw_open_weather.daily_summary
WHERE latitude = 52.52 AND longitude = 13.405
  AND summary_date = '2024-01-15';
```

### Weather Overview (v0.3.1)
```sql
SELECT weather_overview
FROM fdw_open_weather.weather_overview
WHERE latitude = 52.52 AND longitude = 13.405
  AND overview_date = '2025-10-29';
```

## Advanced Patterns

### Interval Arithmetic (v0.3.1 Benefit)
```sql
-- Get weather from 7 days ago using PostgreSQL intervals
SELECT observation_time, temp_celsius
FROM fdw_open_weather.historical_weather
WHERE latitude = 52.52 AND longitude = 13.405
  AND observation_time = NOW() - INTERVAL '7 days';
```

### Temperature Range Filtering
```sql
-- Find hours with comfortable temperature
SELECT forecast_time, temp_celsius
FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405
  AND temp_celsius BETWEEN 18 AND 24;
```

### Aggregations
```sql
-- Calculate average forecasted temperature
SELECT AVG(temp_celsius) as avg_temp
FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405;
```

### Cross-Source Joins
```sql
-- Join weather with your local data
SELECT l.city_name, w.temp_celsius, w.weather_description
FROM my_cities l
JOIN fdw_open_weather.current_weather w
  ON l.latitude = w.latitude AND l.longitude = w.longitude;
```

## Common Coordinates

| Location | Latitude | Longitude |
|----------|----------|-----------|
| Berlin, Germany | 52.52 | 13.405 |
| New York, USA | 40.7128 | -74.0060 |
| Tokyo, Japan | 35.6762 | 139.6503 |
| Sydney, Australia | -33.8688 | 151.2093 |

For more examples, see endpoint-specific documentation in [docs/endpoints/](../endpoints/).
