# Weather Alerts

Government weather alerts (warnings) for the specified location.

**API:** `/onecall` (alerts array)

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_alerts (
  latitude numeric,
  longitude numeric,
  sender_name text,
  event text,
  alert_start timestamptz,
  alert_end timestamptz,
  description text,
  tags text
)
SERVER openweather_server
OPTIONS (object 'weather_alerts');
```

## Example Query

```sql
-- Check for active weather alerts
SELECT event, alert_start, alert_end, description
FROM fdw_open_weather.weather_alerts
WHERE latitude = 52.52 AND longitude = 13.405;
```

## More Information

- **Setup:** See [QUICKSTART.md](../../QUICKSTART.md)
- **All Endpoints:** See [README.md](../README.md)
- **API Details:** [OpenWeather One Call API](https://openweathermap.org/api/one-call-3)
