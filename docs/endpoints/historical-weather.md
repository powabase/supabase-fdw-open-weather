# Historical Weather

Historical weather data from 1979 onwards using native PostgreSQL TIMESTAMPTZ.

**API:** `/onecall/timemachine`

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.historical_weather (
  latitude numeric,
  longitude numeric,
  observation_time timestamptz,  -- v0.3.1: Use TIMESTAMPTZ, not dt!
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
OPTIONS (object 'historical_weather');
```

## Example Query

```sql
-- Get historical weather (v0.3.1 with native TIMESTAMPTZ)
SELECT observation_time, temp_celsius, weather_description
FROM fdw_open_weather.historical_weather
WHERE latitude = 52.52 AND longitude = 13.405
  AND observation_time = '2024-01-01 00:00:00+00';
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **Migration:** See [MIGRATION.md](../../MIGRATION.md) for v0.3.0 → v0.3.1 changes
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
