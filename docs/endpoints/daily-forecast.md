# Daily Forecast

Daily weather forecast for the next 8 days with min/max temperatures and precipitation.

**API:** `/onecall` (daily array)

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.daily_forecast (
  latitude numeric,
  longitude numeric,
  forecast_date timestamptz,
  temp_min_celsius numeric,
  temp_max_celsius numeric,
  temp_morning_celsius numeric,
  temp_day_celsius numeric,
  temp_evening_celsius numeric,
  temp_night_celsius numeric,
  -- ... 25 more columns (see full schema via IMPORT FOREIGN SCHEMA)
)
SERVER openweather_server
OPTIONS (object 'daily_forecast');
```

## Example Query

```sql
-- Get 8-day forecast
SELECT forecast_date, temp_min_celsius, temp_max_celsius, weather_description
FROM fdw_open_weather.daily_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_date;
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **All Endpoints:** See [README.md](../README.md)
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
