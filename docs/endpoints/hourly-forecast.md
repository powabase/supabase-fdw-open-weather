# Hourly Forecast Endpoint

**Endpoint:** `hourly_forecast`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall
**Section:** `hourly` array from `/onecall` response

Detailed hourly weather forecast for the next 48 hours. Includes temperature, precipitation probability, rain/snow volumes, and full weather conditions.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.hourly_forecast (
  lat numeric,
  lon numeric,
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
  pop numeric,
  rain_1h numeric,
  snow_1h numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'hourly_forecast');
```

## Columns

| Column | Type | Description | Units (metric) | Nullable |
|--------|------|-------------|----------------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees | No |
| `lon` | numeric | Longitude of location | Decimal degrees | No |
| `dt` | bigint | Forecast time | Unix timestamp (UTC) | No |
| `temp` | numeric | Temperature | °C / °F / K | No |
| `feels_like` | numeric | Perceived temperature | °C / °F / K | No |
| `pressure` | bigint | Atmospheric pressure | hPa | No |
| `humidity` | bigint | Humidity | % (0-100) | No |
| `dew_point` | numeric | Dew point temperature | °C / °F / K | No |
| `uvi` | numeric | UV index | Scale 0-11+ | No |
| `clouds` | bigint | Cloudiness | % (0-100) | No |
| `visibility` | bigint | Visibility | meters | No |
| `wind_speed` | numeric | Wind speed | m/s / mph | No |
| `wind_deg` | bigint | Wind direction | Degrees (0=North) | No |
| `wind_gust` | numeric | Wind gust speed | m/s / mph | **Yes** |
| `pop` | numeric | Probability of precipitation | 0-1 (0%=0, 100%=1) | No |
| `rain_1h` | numeric | Rain volume (last hour) | mm | **Yes** |
| `snow_1h` | numeric | Snow volume (last hour) | mm | **Yes** |
| `weather_main` | text | Weather category | "Clear", "Rain", etc. | No |
| `weather_description` | text | Detailed description | "light rain", etc. | No |
| `weather_icon` | text | Weather icon code | "10d", "01n", etc. | No |

### Special Notes on Nullable Fields

- `wind_gust`: NULL when no gusts expected
- `rain_1h`: NULL when no rain expected (use `pop` for probability)
- `snow_1h`: NULL when no snow expected

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
-- Next 48 hours forecast for Berlin
SELECT * FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Next 24 hours with readable time
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  weather_description,
  ROUND(pop * 100, 0) as rain_probability_pct
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '24 hours')::bigint
ORDER BY dt;

-- Find warmest/coldest hours
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY temp DESC
LIMIT 5;
```

## Advanced Queries

### Find Next Rain Event

```sql
-- When will it rain? (pop > 50%)
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(pop * 100, 0) as rain_prob_pct,
  COALESCE(ROUND(rain_1h, 1), 0) as expected_rain_mm,
  weather_description
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND pop > 0.5
ORDER BY dt
LIMIT 5;
```

### Comfortable Outdoor Hours

```sql
-- Find best hours for outdoor activities (temp 15-25°C, no rain, low wind)
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(wind_speed, 1) as wind_ms,
  ROUND(pop * 100, 0) as rain_prob_pct,
  uvi,
  weather_description,
  CASE
    WHEN temp BETWEEN 15 AND 25
         AND pop < 0.2
         AND wind_speed < 5
         AND weather_main = 'Clear'
      THEN '⭐ Perfect conditions'
    WHEN temp BETWEEN 15 AND 25 AND pop < 0.3
      THEN '✅ Good conditions'
    WHEN pop > 0.7 OR weather_main IN ('Rain', 'Thunderstorm')
      THEN '❌ Not recommended'
    ELSE '⚠️ Fair conditions'
  END as activity_rating
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Temperature Trend Analysis

```sql
-- Track temperature changes hour-by-hour
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(temp - LAG(temp) OVER (ORDER BY dt), 1) as temp_change_c,
  CASE
    WHEN temp - LAG(temp) OVER (ORDER BY dt) > 2 THEN '↗️ Warming quickly'
    WHEN temp - LAG(temp) OVER (ORDER BY dt) < -2 THEN '↘️ Cooling quickly'
    WHEN temp - LAG(temp) OVER (ORDER BY dt) > 0 THEN '→ Slight warming'
    WHEN temp - LAG(temp) OVER (ORDER BY dt) < 0 THEN '→ Slight cooling'
    ELSE '= Stable'
  END as trend
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt
LIMIT 24;
```

### Precipitation Probability Over Time

```sql
-- Aggregate precipitation chance by time of day
SELECT
  TO_CHAR(TO_TIMESTAMP(dt), 'YYYY-MM-DD') as date,
  TO_CHAR(TO_TIMESTAMP(dt), 'HH24') as hour,
  ROUND(AVG(pop) * 100, 0) as avg_rain_prob_pct,
  ROUND(SUM(COALESCE(rain_1h, 0)), 1) as total_rain_mm,
  COUNT(*) as forecast_count
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
GROUP BY TO_CHAR(TO_TIMESTAMP(dt), 'YYYY-MM-DD'),
         TO_CHAR(TO_TIMESTAMP(dt), 'HH24')
ORDER BY date, hour;
```

### Wind Forecast Analysis

```sql
-- Find high wind periods (>10 m/s)
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(wind_speed, 1) as wind_ms,
  ROUND(COALESCE(wind_gust, 0), 1) as gust_ms,
  wind_deg,
  CASE
    WHEN wind_deg >= 337.5 OR wind_deg < 22.5 THEN 'N'
    WHEN wind_deg >= 22.5 AND wind_deg < 67.5 THEN 'NE'
    WHEN wind_deg >= 67.5 AND wind_deg < 112.5 THEN 'E'
    WHEN wind_deg >= 112.5 AND wind_deg < 157.5 THEN 'SE'
    WHEN wind_deg >= 157.5 AND wind_deg < 202.5 THEN 'S'
    WHEN wind_deg >= 202.5 AND wind_deg < 247.5 THEN 'SW'
    WHEN wind_deg >= 247.5 AND wind_deg < 292.5 THEN 'W'
    ELSE 'NW'
  END as wind_direction,
  CASE
    WHEN wind_speed > 24 THEN '🌪️ Storm'
    WHEN wind_speed > 17 THEN '💨 Very windy'
    WHEN wind_speed > 10 THEN '🍃 Windy'
    ELSE '🌤️ Calm/Moderate'
  END as wind_category
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND wind_speed > 10
ORDER BY dt;
```

## Response Characteristics

- **Rows:** Always 48
- **Update Frequency:** Hourly
- **Response Size:** ~8-10 KB
- **Query Time:** < 2 seconds
- **Time Interval:** 3600 seconds (1 hour) between rows
- **Worldwide Availability:** Yes

## Data Quality Notes

### Guaranteed Fields
All fields have values except:
- `wind_gust`: NULL when no gusts
- `rain_1h`: NULL when no rain expected
- `snow_1h`: NULL when no snow expected

### Precipitation Probability

`pop` (Probability of Precipitation):
- `0.0` = 0% chance (no rain)
- `0.5` = 50% chance
- `1.0` = 100% chance (certain rain)

**Important:** `pop` is a probability. `rain_1h` is actual expected volume.

### Time Validation

```sql
-- Verify hourly intervals
SELECT
  dt,
  (dt - LAG(dt) OVER (ORDER BY dt))/3600 as hours_between
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
LIMIT 10;
-- Expected: 1 hour between rows
```

### UV Index Interpretation

- `0-2`: Low
- `3-5`: Moderate
- `6-7`: High
- `8-10`: Very High
- `11+`: Extreme

## Common Use Cases

### Smart Irrigation Scheduling

```sql
-- Find best watering windows (no rain, moderate temp, low wind)
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(pop * 100, 0) as rain_prob_pct,
  ROUND(wind_speed, 1) as wind_ms,
  CASE
    WHEN pop < 0.2
         AND temp BETWEEN 10 AND 30
         AND wind_speed < 5
         AND EXTRACT(HOUR FROM TO_TIMESTAMP(dt)) BETWEEN 6 AND 9
      THEN '⭐ Optimal watering time'
    WHEN pop < 0.3 AND temp < 35
      THEN '✅ Good watering time'
    WHEN pop > 0.5
      THEN '❌ Skip - rain expected'
    ELSE '⚠️ Acceptable'
  END as irrigation_recommendation
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND dt <= EXTRACT(EPOCH FROM NOW() + INTERVAL '24 hours')::bigint
ORDER BY dt;
```

### HVAC Optimization

```sql
-- Predict heating/cooling needs
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(temp, 1) as temp_c,
  ROUND(feels_like, 1) as feels_like_c,
  humidity,
  CASE
    WHEN feels_like < 18 THEN 'Heating needed'
    WHEN feels_like > 24 THEN 'Cooling needed'
    ELSE 'Comfortable - no HVAC'
  END as hvac_mode,
  CASE
    WHEN feels_like < 18 THEN ROUND((18 - feels_like) * 0.5, 1)
    WHEN feels_like > 24 THEN ROUND((feels_like - 24) * 0.5, 1)
    ELSE 0
  END as estimated_load_kw
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Solar Power Forecasting

```sql
-- Estimate solar generation potential (simplified)
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  clouds,
  uvi,
  CASE
    WHEN clouds < 20 AND uvi > 5 THEN 'High generation (80-100%)'
    WHEN clouds < 50 AND uvi > 3 THEN 'Good generation (60-80%)'
    WHEN clouds < 75 THEN 'Moderate generation (30-60%)'
    ELSE 'Low generation (<30%)'
  END as solar_forecast,
  -- Rough estimate: (100 - clouds) * uvi * 10 = watts per panel
  ROUND((100 - clouds) * uvi * 0.1, 0) as relative_output_pct
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND EXTRACT(HOUR FROM TO_TIMESTAMP(dt)) BETWEEN 6 AND 20
ORDER BY dt;
```

## Caching Strategy

```sql
-- Create materialized view (refreshed hourly)
CREATE MATERIALIZED VIEW mv_hourly_forecast_cache AS
SELECT *
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every hour
SELECT cron.schedule('refresh-hourly-forecast', '0 * * * *',
  'REFRESH MATERIALIZED VIEW mv_hourly_forecast_cache');
```

**Benefits:**
- Reduces API calls from potentially 100s/day to 24/day
- Sub-millisecond query response
- Matches API update frequency

## Troubleshooting

### Unexpected NULL in rain_1h/snow_1h

This is **normal** - these fields are NULL when no precipitation expected. Check `pop` for probability.

```sql
-- Use COALESCE to handle NULLs
SELECT
  TO_TIMESTAMP(dt) AT TIME ZONE 'UTC' as time,
  ROUND(pop * 100, 0) as rain_prob_pct,
  COALESCE(ROUND(rain_1h, 2), 0) as rain_mm,
  COALESCE(ROUND(snow_1h, 2), 0) as snow_mm
FROM fdw_open_weather.hourly_forecast
WHERE lat = 52.52 AND lon = 13.405
LIMIT 5;
```

### Row Count Not 48

Should always return 48 rows. If not:
- Check API key validity
- Verify lat/lon parameters
- Check Supabase logs for errors

### Stale Forecast Data

Hourly forecast updates hourly. Check `dt` of first row - should be within 1-2 hours of current time.

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Current Weather](current-weather.md)** - Current conditions
- **[Minutely Forecast](minutely-forecast.md)** - Minute-by-minute precipitation
- **[Daily Forecast](daily-forecast.md)** - 8-day summary forecast
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
