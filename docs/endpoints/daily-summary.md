# Daily Summary

Aggregated daily weather statistics for any historical date.

**API:** `/onecall/day_summary`

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.daily_summary (
  latitude numeric,
  longitude numeric,
  summary_date text,  -- v0.3.1: Use summary_date, not date!
  temp_min_celsius numeric,
  temp_max_celsius numeric,
  temp_avg_celsius numeric,
  precipitation_mm numeric,
  -- ... 10 more columns (see full schema via IMPORT FOREIGN SCHEMA)
)
SERVER openweather_server
OPTIONS (object 'daily_summary');
```

## Example Query

```sql
-- Get daily weather summary (v0.3.1)
SELECT summary_date, temp_min_celsius, temp_max_celsius, precipitation_mm
FROM fdw_open_weather.daily_summary
WHERE latitude = 52.52 AND longitude = 13.405
  AND summary_date = '2024-01-15';
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **Migration:** See [MIGRATION.md](../../MIGRATION.md) for v0.3.0 → v0.3.1 changes
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
