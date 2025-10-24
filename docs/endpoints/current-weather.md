# Current Weather Endpoint

**Endpoint:** `current_weather`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall
**Section:** `current` object from `/onecall` response

Returns instantaneous weather conditions for any location worldwide, updated every 10 minutes.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.current_weather (
  lat numeric,
  lon numeric,
  timezone text,
  dt bigint,
  temp numeric,
  feels_like numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  uvi numeric,
  clouds bigint,
  visibility bigint,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'current_weather');
```

## Columns

| Column | Type | Description | Units (metric) | Nullable |
|--------|------|-------------|----------------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees (-90 to 90) | No |
| `lon` | numeric | Longitude of location | Decimal degrees (-180 to 180) | No |
| `timezone` | text | Timezone name | IANA timezone (e.g., "Europe/Berlin") | No |
| `dt` | bigint | Current time | Unix timestamp (UTC) | No |
| `temp` | numeric | Temperature | °C (metric), °F (imperial), K (standard) | No |
| `feels_like` | numeric | Human perception of temperature | °C (metric), °F (imperial), K (standard) | No |
| `pressure` | bigint | Atmospheric pressure at sea level | hPa (hectopascal) | No |
| `humidity` | bigint | Humidity | % (0-100) | No |
| `dew_point` | numeric | Dew point temperature | °C (metric), °F (imperial), K (standard) | No |
| `uvi` | numeric | UV index | Scale 0-11+ | No |
| `clouds` | bigint | Cloudiness | % (0-100) | No |
| `visibility` | bigint | Visibility distance | meters | No |
| `wind_speed` | numeric | Wind speed | m/s (metric), mph (imperial) | No |
| `wind_deg` | bigint | Wind direction | Degrees (meteorological, 0=North) | No |
| `wind_gust` | numeric | Wind gust speed | m/s (metric), mph (imperial) | **Yes** |
| `weather_main` | text | Weather condition category | "Clear", "Rain", "Snow", etc. | No |
| `weather_description` | text | Weather condition description | "clear sky", "light rain", etc. | No |
| `weather_icon` | text | Weather icon code | "01d", "10n", etc. | No |

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |

### Optional Parameters (Server OPTIONS)
- `units`: "metric" (default), "imperial", or "standard"
- `lang`: Language code (default: "en")

## Basic Usage

```sql
-- Current weather for Berlin
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;

-- Current weather for New York
SELECT * FROM fdw_open_weather.current_weather
WHERE lat = 40.7128 AND lon = -74.0060;

-- Current weather with human-readable time
SELECT
  timezone,
  TO_TIMESTAMP(dt) AT TIME ZONE timezone as local_time,
  ROUND(temp, 1) as temp_c,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

## Advanced Queries

### Multi-Location Current Weather

```sql
-- Create locations table
CREATE TABLE locations (
  city_name TEXT,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO locations VALUES
  ('Berlin', 52.52, 13.405),
  ('London', 51.5074, -0.1278),
  ('Paris', 48.8566, 2.3522),
  ('New York', 40.7128, -74.0060),
  ('Tokyo', 35.6762, 139.6503);

-- Get current weather for all cities
SELECT
  l.city_name,
  ROUND(c.temp, 1) as temp_c,
  ROUND(c.feels_like, 1) as feels_like_c,
  c.humidity,
  c.weather_description,
  TO_TIMESTAMP(c.dt) as time_utc
FROM locations l
CROSS JOIN LATERAL (
  SELECT * FROM fdw_open_weather.current_weather
  WHERE lat = l.lat AND lon = l.lon
) c
ORDER BY c.temp DESC;
```

### Weather Condition Monitoring

```sql
-- Find cities with extreme conditions
SELECT
  l.city_name,
  ROUND(c.temp, 1) as temp_c,
  c.humidity,
  c.uvi,
  c.weather_main,
  CASE
    WHEN c.temp > 30 THEN '🔥 Hot'
    WHEN c.temp < 0 THEN '❄️ Freezing'
    WHEN c.uvi > 8 THEN '☀️ High UV'
    WHEN c.humidity > 80 THEN '💧 Humid'
    ELSE '✅ Normal'
  END as condition_alert
FROM locations l
CROSS JOIN LATERAL (
  SELECT * FROM fdw_open_weather.current_weather
  WHERE lat = l.lat AND lon = l.lon
) c
WHERE c.temp > 30 OR c.temp < 0 OR c.uvi > 8 OR c.humidity > 80;
```

### Temperature Feels-Like Analysis

```sql
-- Compare actual vs feels-like temperature
SELECT
  ROUND(temp, 1) as actual_temp_c,
  ROUND(feels_like, 1) as feels_like_c,
  ROUND(feels_like - temp, 1) as difference_c,
  CASE
    WHEN feels_like - temp > 3 THEN 'Feels much warmer'
    WHEN feels_like - temp < -3 THEN 'Feels much colder'
    ELSE 'Feels similar'
  END as perception,
  weather_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Wind Conditions

```sql
-- Analyze wind conditions
SELECT
  ROUND(wind_speed, 1) as wind_speed_ms,
  wind_deg,
  CASE
    WHEN wind_deg >= 337.5 OR wind_deg < 22.5 THEN 'N'
    WHEN wind_deg >= 22.5 AND wind_deg < 67.5 THEN 'NE'
    WHEN wind_deg >= 67.5 AND wind_deg < 112.5 THEN 'E'
    WHEN wind_deg >= 112.5 AND wind_deg < 157.5 THEN 'SE'
    WHEN wind_deg >= 157.5 AND wind_deg < 202.5 THEN 'S'
    WHEN wind_deg >= 202.5 AND wind_deg < 247.5 THEN 'SW'
    WHEN wind_deg >= 247.5 AND wind_deg < 292.5 THEN 'W'
    WHEN wind_deg >= 292.5 AND wind_deg < 337.5 THEN 'NW'
  END as wind_direction,
  ROUND(COALESCE(wind_gust, 0), 1) as wind_gust_ms,
  CASE
    WHEN wind_speed < 0.5 THEN 'Calm'
    WHEN wind_speed < 5.5 THEN 'Light breeze'
    WHEN wind_speed < 10.8 THEN 'Moderate breeze'
    WHEN wind_speed < 17.2 THEN 'Strong breeze'
    ELSE 'High wind'
  END as beaufort_description
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

## Response Characteristics

- **Rows:** Always 1 row per query
- **Update Frequency:** Every 10 minutes
- **Response Size:** ~2-3 KB
- **Query Time:** < 2 seconds
- **Cache Duration:** 10 minutes recommended

## Data Quality Notes

### Guaranteed Fields
All fields will always have values except:
- `wind_gust`: NULL when no wind gusts present

### Time Zones
- `dt`: Always in UTC (Unix timestamp)
- `timezone`: IANA timezone name for the location
- Convert using `TO_TIMESTAMP(dt) AT TIME ZONE timezone`

### Weather Icons
Icon codes correspond to OpenWeather icon set:
- `01d`, `01n` - Clear sky (day/night)
- `02d`, `02n` - Few clouds
- `03d`, `03n` - Scattered clouds
- `04d`, `04n` - Broken clouds
- `09d`, `09n` - Shower rain
- `10d`, `10n` - Rain
- `11d`, `11n` - Thunderstorm
- `13d`, `13n` - Snow
- `50d`, `50n` - Mist

Full icon documentation: https://openweathermap.org/weather-conditions

### Units Conversion

**Temperature:**
- Metric: Celsius (°C)
- Imperial: Fahrenheit (°F)
- Standard: Kelvin (K)
- Conversion: `°F = (°C × 9/5) + 32`, `K = °C + 273.15`

**Wind Speed:**
- Metric: meters per second (m/s)
- Imperial: miles per hour (mph)
- Conversion: `1 m/s = 2.237 mph`

**Visibility:**
- Always in meters
- Max value: typically 10,000 meters

## Caching Strategy

Since current weather updates every 10 minutes, cache for that duration:

```sql
-- Create materialized view
CREATE MATERIALIZED VIEW mv_current_weather_cache AS
SELECT
  'Berlin' as city_name,
  c.*
FROM fdw_open_weather.current_weather c
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every 10 minutes
SELECT cron.schedule('refresh-current-weather', '*/10 * * * *',
  'REFRESH MATERIALIZED VIEW mv_current_weather_cache');

-- Query the cache (instant response, 0 API calls)
SELECT * FROM mv_current_weather_cache;
```

**Benefits:**
- Reduces API calls from potentially thousands to 144/day
- Sub-millisecond query response
- Matches API update frequency

## Common Use Cases

### Smart Home Automation

```sql
-- Check if outdoor temperature comfortable for window opening
SELECT
  CASE
    WHEN temp BETWEEN 18 AND 24 AND humidity < 70
      THEN 'Open windows - conditions ideal'
    WHEN temp > 24
      THEN 'Consider cooling'
    WHEN temp < 18
      THEN 'Keep windows closed'
    ELSE 'Monitor conditions'
  END as recommendation,
  ROUND(temp, 1) as temp_c,
  humidity as humidity_pct
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Outdoor Activity Planning

```sql
-- Check if conditions suitable for outdoor activities
SELECT
  weather_main,
  weather_description,
  ROUND(temp, 1) as temp_c,
  ROUND(wind_speed, 1) as wind_ms,
  uvi,
  CASE
    WHEN weather_main IN ('Rain', 'Thunderstorm', 'Snow') THEN '❌ Not recommended'
    WHEN wind_speed > 10 THEN '⚠️ Windy - exercise caution'
    WHEN uvi > 8 THEN '☀️ High UV - sun protection needed'
    WHEN temp < 5 OR temp > 35 THEN '🌡️ Extreme temperature'
    ELSE '✅ Good conditions'
  END as activity_recommendation
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

### Energy Management

```sql
-- Estimate heating/cooling needs based on feels-like temperature
SELECT
  ROUND(temp, 1) as actual_temp_c,
  ROUND(feels_like, 1) as feels_like_c,
  CASE
    WHEN feels_like < 18 THEN ROUND((18 - feels_like) * 0.5, 1)
    ELSE 0
  END as heating_demand_kw,
  CASE
    WHEN feels_like > 24 THEN ROUND((feels_like - 24) * 0.5, 1)
    ELSE 0
  END as cooling_demand_kw
FROM fdw_open_weather.current_weather
WHERE lat = 52.52 AND lon = 13.405;
```

## Troubleshooting

### No Results Returned

**Check:**
1. Are lat/lon within valid ranges?
2. Is API key active?
3. Is network connectivity working?

```sql
-- Verify server configuration
SELECT unnest(srvoptions)
FROM pg_foreign_server
WHERE srvname = 'openweather_server';
```

### NULL Values

**If all values are NULL:**
- Check WASM binary target (must be `wasm32-unknown-unknown`)
- Verify API key is correct
- Check Supabase logs: `supabase logs --db`

**If only `wind_gust` is NULL:**
- This is expected when no wind gusts are present

### Stale Data

Current weather updates every 10 minutes. If data seems stale:
- Check `dt` timestamp
- Compare with current time
- Refresh materialized views if using caching

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Hourly Forecast](hourly-forecast.md)** - 48-hour detailed forecast
- **[Daily Forecast](daily-forecast.md)** - 8-day summary forecast
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[Troubleshooting](../guides/TROUBLESHOOTING.md)** - Common issues
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
