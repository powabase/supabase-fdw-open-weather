# Daily Forecast Endpoint

**Endpoint:** `daily_forecast`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall
**Section:** `daily` array from `/onecall` response

8-day weather forecast with comprehensive daily summaries including min/max temperatures, sunrise/sunset times, moon phases, and precipitation probabilities.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.daily_forecast (
  lat numeric,
  lon numeric,
  dt bigint,
  sunrise bigint,
  sunset bigint,
  moonrise bigint,
  moonset bigint,
  moon_phase numeric,
  temp_day numeric,
  temp_min numeric,
  temp_max numeric,
  temp_night numeric,
  temp_eve numeric,
  temp_morn numeric,
  feels_like_day numeric,
  feels_like_night numeric,
  feels_like_eve numeric,
  feels_like_morn numeric,
  pressure bigint,
  humidity bigint,
  dew_point numeric,
  wind_speed numeric,
  wind_deg bigint,
  wind_gust numeric,
  clouds bigint,
  pop numeric,
  rain numeric,
  snow numeric,
  uvi numeric,
  weather_main text,
  weather_description text,
  weather_icon text
)
SERVER openweather_server
OPTIONS (object 'daily_forecast');
```

## Columns

| Column | Type | Description | Units (metric) | Nullable |
|--------|------|-------------|----------------|----------|
| `lat` | numeric | Latitude | Decimal degrees | No |
| `lon` | numeric | Longitude | Decimal degrees | No |
| `dt` | bigint | Date (midday timestamp) | Unix timestamp (UTC) | No |
| `sunrise` | bigint | Sunrise time | Unix timestamp (UTC) | No |
| `sunset` | bigint | Sunset time | Unix timestamp (UTC) | No |
| `moonrise` | bigint | Moonrise time | Unix timestamp (UTC) | No |
| `moonset` | bigint | Moonset time | Unix timestamp (UTC) | No |
| `moon_phase` | numeric | Moon phase | 0-1 (see below) | No |
| `temp_day` | numeric | Daytime temperature | °C / °F / K | No |
| `temp_min` | numeric | Minimum temperature | °C / °F / K | No |
| `temp_max` | numeric | Maximum temperature | °C / °F / K | No |
| `temp_night` | numeric | Nighttime temperature | °C / °F / K | No |
| `temp_eve` | numeric | Evening temperature | °C / °F / K | No |
| `temp_morn` | numeric | Morning temperature | °C / °F / K | No |
| `feels_like_day` | numeric | Daytime feels-like | °C / °F / K | No |
| `feels_like_night` | numeric | Nighttime feels-like | °C / °F / K | No |
| `feels_like_eve` | numeric | Evening feels-like | °C / °F / K | No |
| `feels_like_morn` | numeric | Morning feels-like | °C / °F / K | No |
| `pressure` | bigint | Atmospheric pressure | hPa | No |
| `humidity` | bigint | Humidity | % (0-100) | No |
| `dew_point` | numeric | Dew point | °C / °F / K | No |
| `wind_speed` | numeric | Wind speed | m/s / mph | No |
| `wind_deg` | bigint | Wind direction | Degrees (0=North) | No |
| `wind_gust` | numeric | Wind gust speed | m/s / mph | **Yes** |
| `clouds` | bigint | Cloudiness | % (0-100) | No |
| `pop` | numeric | Precipitation probability | 0-1 (0%=0, 100%=1) | No |
| `rain` | numeric | Rain volume (daily total) | mm | **Yes** |
| `snow` | numeric | Snow volume (daily total) | mm | **Yes** |
| `uvi` | numeric | Max UV index | Scale 0-11+ | No |
| `weather_main` | text | Weather category | "Clear", "Rain", etc. | No |
| `weather_description` | text | Detailed description | "light rain", etc. | No |
| `weather_icon` | text | Weather icon code | "10d", "01n", etc. | No |

### Moon Phase Values

- `0` and `1` - New moon
- `0.25` - First quarter
- `0.5` - Full moon
- `0.75` - Last quarter

Intermediate values indicate waxing/waning phases.

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
-- 8-day forecast for Berlin
SELECT * FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Weekly summary with readable dates
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_min, 1) as min_c,
  ROUND(temp_max, 1) as max_c,
  weather_description,
  ROUND(pop * 100, 0) as rain_prob_pct
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;

-- Sunrise/sunset times
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  TO_TIMESTAMP(sunrise) AT TIME ZONE 'UTC' as sunrise_utc,
  TO_TIMESTAMP(sunset) AT TIME ZONE 'UTC' as sunset_utc,
  ROUND(EXTRACT(EPOCH FROM (TO_TIMESTAMP(sunset) - TO_TIMESTAMP(sunrise)))/3600, 1) as daylight_hours
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

## Advanced Queries

### Find Best Day for Outdoor Activities

```sql
-- Score each day for outdoor suitability
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_max, 1) as max_temp_c,
  ROUND(pop * 100, 0) as rain_prob_pct,
  weather_main,
  moon_phase,
  CASE
    WHEN temp_max BETWEEN 18 AND 28
         AND pop < 0.2
         AND weather_main = 'Clear'
         AND uvi < 8
      THEN '⭐⭐⭐ Excellent'
    WHEN temp_max BETWEEN 15 AND 30
         AND pop < 0.4
         AND weather_main IN ('Clear', 'Clouds')
      THEN '⭐⭐ Good'
    WHEN pop > 0.7 OR weather_main IN ('Rain', 'Thunderstorm')
      THEN '❌ Poor'
    ELSE '⭐ Fair'
  END as day_rating,
  CASE
    WHEN moon_phase BETWEEN 0.45 AND 0.55 THEN '🌕 Full moon'
    WHEN moon_phase < 0.05 OR moon_phase > 0.95 THEN '🌑 New moon'
    WHEN moon_phase BETWEEN 0.2 AND 0.3 THEN '🌓 First quarter'
    WHEN moon_phase BETWEEN 0.7 AND 0.8 THEN '🌗 Last quarter'
    ELSE '🌘 Crescent/Gibbous'
  END as moon_desc
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Temperature Range Analysis

```sql
-- Analyze daily temperature variations
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_min, 1) as min_c,
  ROUND(temp_max, 1) as max_c,
  ROUND(temp_max - temp_min, 1) as range_c,
  ROUND(temp_morn, 1) as morning_c,
  ROUND(temp_day, 1) as afternoon_c,
  ROUND(temp_eve, 1) as evening_c,
  ROUND(temp_night, 1) as night_c,
  CASE
    WHEN temp_max - temp_min > 15 THEN '📊 High variation'
    WHEN temp_max - temp_min > 10 THEN '📊 Moderate variation'
    ELSE '📊 Low variation'
  END as temp_variability
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Precipitation Forecast

```sql
-- Days with rain expected
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(pop * 100, 0) as rain_probability_pct,
  COALESCE(ROUND(rain, 1), 0) as expected_rain_mm,
  COALESCE(ROUND(snow, 1), 0) as expected_snow_mm,
  weather_description,
  CASE
    WHEN pop > 0.8 THEN '☔ Rain very likely'
    WHEN pop > 0.5 THEN '🌧️ Rain probable'
    WHEN pop > 0.2 THEN '⛅ Rain possible'
    ELSE '☀️ Rain unlikely'
  END as forecast_summary
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
  AND pop > 0.2
ORDER BY dt;
```

### Weekly Daylight Tracking

```sql
-- Track sunrise/sunset changes over the week
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  TO_CHAR(TO_TIMESTAMP(sunrise), 'HH24:MI') as sunrise_time,
  TO_CHAR(TO_TIMESTAMP(sunset), 'HH24:MI') as sunset_time,
  ROUND(EXTRACT(EPOCH FROM (TO_TIMESTAMP(sunset) - TO_TIMESTAMP(sunrise)))/3600, 2) as daylight_hours,
  ROUND(
    EXTRACT(EPOCH FROM (TO_TIMESTAMP(sunset) - TO_TIMESTAMP(sunrise)))/3600 -
    LAG(EXTRACT(EPOCH FROM (TO_TIMESTAMP(sunset) - TO_TIMESTAMP(sunrise)))/3600)
      OVER (ORDER BY dt),
    2
  ) as daylight_change_hours
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Feels-Like vs Actual Temperature

```sql
-- Compare actual temps with feels-like
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_day, 1) as actual_day_c,
  ROUND(feels_like_day, 1) as feels_day_c,
  ROUND(feels_like_day - temp_day, 1) as day_difference_c,
  humidity,
  ROUND(wind_speed, 1) as wind_ms,
  CASE
    WHEN feels_like_day - temp_day > 3 THEN '🔥 Feels much warmer'
    WHEN feels_like_day - temp_day < -3 THEN '❄️ Feels much colder'
    ELSE '≈ Feels similar'
  END as perception
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Moon Phase Calendar

```sql
-- Generate moon phase calendar
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  moon_phase,
  CASE
    WHEN moon_phase < 0.05 OR moon_phase > 0.95 THEN '🌑 New Moon'
    WHEN moon_phase >= 0.05 AND moon_phase < 0.20 THEN '🌒 Waxing Crescent'
    WHEN moon_phase >= 0.20 AND moon_phase < 0.30 THEN '🌓 First Quarter'
    WHEN moon_phase >= 0.30 AND moon_phase < 0.45 THEN '🌔 Waxing Gibbous'
    WHEN moon_phase >= 0.45 AND moon_phase < 0.55 THEN '🌕 Full Moon'
    WHEN moon_phase >= 0.55 AND moon_phase < 0.70 THEN '🌖 Waning Gibbous'
    WHEN moon_phase >= 0.70 AND moon_phase < 0.80 THEN '🌗 Last Quarter'
    ELSE '🌘 Waning Crescent'
  END as moon_phase_name,
  TO_CHAR(TO_TIMESTAMP(moonrise), 'HH24:MI') as moonrise_time,
  TO_CHAR(TO_TIMESTAMP(moonset), 'HH24:MI') as moonset_time
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

## Response Characteristics

- **Rows:** Always 8
- **Update Frequency:** Multiple times per day
- **Response Size:** ~6-8 KB
- **Query Time:** < 2 seconds
- **Time Interval:** 86400 seconds (24 hours) between rows
- **Worldwide Availability:** Yes

## Data Quality Notes

### Guaranteed Fields

All fields have values except:
- `wind_gust`: NULL when no gusts expected
- `rain`: NULL when no rain expected
- `snow`: NULL when no snow expected

### Temperature Fields Explained

**Actual Temperatures:**
- `temp_min`: Lowest temperature of the day
- `temp_max`: Highest temperature of the day
- `temp_morn`: Morning temperature (~6am)
- `temp_day`: Afternoon temperature (~12pm)
- `temp_eve`: Evening temperature (~6pm)
- `temp_night`: Night temperature (~12am)

**Feels-Like Temperatures:**
Factors in humidity and wind to calculate perceived temperature for morning, day, evening, and night.

### Sun/Moon Times

All times are Unix timestamps in UTC. Convert using `TO_TIMESTAMP()` and apply timezone as needed.

### Time Validation

```sql
-- Verify daily intervals
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  (dt - LAG(dt) OVER (ORDER BY dt))/86400 as days_between
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405;
-- Expected: 1 day between rows
```

## Common Use Cases

### Event Planning

```sql
-- Find best day next week for outdoor event
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_day, 1) as day_temp_c,
  ROUND(pop * 100, 0) as rain_prob_pct,
  weather_description,
  TO_CHAR(TO_TIMESTAMP(sunset), 'HH24:MI') as sunset_time,
  CASE
    WHEN pop < 0.2 AND temp_day BETWEEN 18 AND 28 AND weather_main = 'Clear'
      THEN 1
    WHEN pop < 0.3 AND temp_day BETWEEN 15 AND 30
      THEN 2
    WHEN pop > 0.7
      THEN 99
    ELSE 3
  END as priority_rank
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY priority_rank, dt
LIMIT 3;
```

### Agriculture Planning

```sql
-- Optimal planting/harvesting days (dry, moderate temp)
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  ROUND(temp_min, 1) as min_temp_c,
  ROUND(temp_max, 1) as max_temp_c,
  ROUND(pop * 100, 0) as rain_prob_pct,
  humidity,
  CASE
    WHEN pop < 0.1
         AND temp_min > 5
         AND temp_max < 30
         AND humidity < 80
      THEN '✅ Excellent for field work'
    WHEN pop < 0.3 AND temp_min > 0
      THEN '⚠️ Acceptable, monitor conditions'
    ELSE '❌ Not recommended'
  END as field_work_suitability
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

### Photography Planning (Golden Hour)

```sql
-- Calculate golden hour times (1 hour after sunrise, 1 hour before sunset)
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  TO_TIMESTAMP(sunrise + 3600) AT TIME ZONE 'UTC' as morning_golden_hour,
  TO_TIMESTAMP(sunset - 3600) AT TIME ZONE 'UTC' as evening_golden_hour,
  clouds,
  weather_description,
  CASE
    WHEN clouds < 30 THEN '📷 Clear skies - great lighting'
    WHEN clouds BETWEEN 30 AND 60 THEN '📷 Partly cloudy - interesting sky'
    ELSE '📷 Overcast - diffused light'
  END as photo_conditions
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
ORDER BY dt;
```

## Caching Strategy

```sql
-- Create materialized view (refreshed 4x/day)
CREATE MATERIALIZED VIEW mv_daily_forecast_cache AS
SELECT *
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every 6 hours
SELECT cron.schedule('refresh-daily-forecast', '0 */6 * * *',
  'REFRESH MATERIALIZED VIEW mv_daily_forecast_cache');
```

**Benefits:**
- Reduces API calls from 100s/day to 4/day
- Sub-millisecond query response

## Troubleshooting

### Unexpected NULL in rain/snow

Normal - indicates no precipitation expected. Use `pop` for probability.

### Sunrise/Sunset Times Seem Wrong

Times are in UTC. Convert to local timezone:

```sql
SELECT
  TO_TIMESTAMP(dt)::DATE as date,
  TO_TIMESTAMP(sunrise) AT TIME ZONE 'Europe/Berlin' as sunrise_local,
  TO_TIMESTAMP(sunset) AT TIME ZONE 'Europe/Berlin' as sunset_local
FROM fdw_open_weather.daily_forecast
WHERE lat = 52.52 AND lon = 13.405
LIMIT 1;
```

### Row Count Not 8

Should always return 8 rows. If not, check API key and lat/lon validity.

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Current Weather](current-weather.md)** - Current conditions
- **[Hourly Forecast](hourly-forecast.md)** - 48-hour detailed forecast
- **[Weather Alerts](weather-alerts.md)** - Government weather alerts
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
