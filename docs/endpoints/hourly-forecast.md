# Hourly Forecast

Hour-by-hour weather forecast for the next 48 hours.

**API:** `/onecall` (hourly array)

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.hourly_forecast (
  latitude numeric,
  longitude numeric,
  forecast_time timestamptz,
  temp_celsius numeric,
  feels_like_celsius numeric,
  pressure_hpa bigint,
  humidity_pct bigint,
  dew_point_celsius numeric,
  uv_index numeric,
  clouds_pct bigint,
  visibility_m bigint,
  wind_speed_m_s numeric,
  wind_direction_deg bigint,
  wind_gust_m_s numeric,
  precipitation_probability_pct bigint,
  rain_1h_mm numeric,
  snow_1h_mm numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'hourly_forecast');
```

## Example Query

```sql
-- Get 48-hour forecast
SELECT forecast_time, temp_celsius, weather_description, precipitation_probability_pct
FROM fdw_open_weather.hourly_forecast
WHERE latitude = 52.52 AND longitude = 13.405
ORDER BY forecast_time;
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **All Endpoints:** See [README.md](../README.md)
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
