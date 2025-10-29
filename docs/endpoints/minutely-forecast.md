# Minutely Forecast

Minute-by-minute precipitation forecast for the next hour (60 data points).

**API:** `/onecall` (minutely array)

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.minutely_forecast (
  latitude numeric,
  longitude numeric,
  forecast_time timestamptz,
  precipitation_mm numeric
)
SERVER openweather_server
OPTIONS (object 'minutely_forecast');
```

## Example Query

```sql
-- Get next hour precipitation forecast
SELECT forecast_time, precipitation_mm
FROM fdw_open_weather.minutely_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_time;
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **All Endpoints:** See [README.md](../README.md)
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
