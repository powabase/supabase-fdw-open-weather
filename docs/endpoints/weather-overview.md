# Weather Overview

AI-generated human-readable weather summary in natural language.

**API:** `/onecall/overview`

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_overview (
  latitude numeric,
  longitude numeric,
  overview_date text,  -- v0.3.1: Use overview_date, not date!
  timezone text,
  timezone_offset bigint,
  date_text text,
  weather_overview text
)
SERVER openweather_server
OPTIONS (object 'weather_overview');
```

## Example Query

```sql
-- Get AI weather overview (v0.3.1)
SELECT weather_overview
FROM fdw_open_weather.weather_overview
WHERE latitude = 52.52 AND longitude = 13.405
  AND overview_date = '2025-10-29';
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **Migration:** See [MIGRATION.md](../../MIGRATION.md) for v0.3.0 → v0.3.1 changes
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
