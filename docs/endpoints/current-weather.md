# Current Weather

Real-time weather conditions for any location, updated every 10 minutes.

**API:** `/onecall` (current object)

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.current_weather (
  latitude numeric,
  longitude numeric,
  timezone text,
  observation_time timestamptz,
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
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'current_weather');
```

## Example Query

```sql
-- Get current weather for Berlin
SELECT
  observation_time,
  temp_celsius,
  weather_description,
  humidity_pct
FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **All Endpoints:** See [README.md](../README.md)
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
