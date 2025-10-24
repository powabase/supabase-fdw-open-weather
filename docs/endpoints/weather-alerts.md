# Weather Alerts Endpoint

**Endpoint:** `weather_alerts`
**Status:** ✅ Production Ready (v0.1.0)
**API:** https://api.openweathermap.org/data/3.0/onecall
**Section:** `alerts` array from `/onecall` response

Government-issued weather alerts for severe weather events, hazards, and warnings. Returns 0-N rows depending on active alerts for the location.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_alerts (
  lat numeric,
  lon numeric,
  sender_name text,
  event text,
  start bigint,
  "end" bigint,
  description text,
  tags text
)
SERVER openweather_server
OPTIONS (object 'weather_alerts');
```

## Columns

| Column | Type | Description | Format | Nullable |
|--------|------|-------------|--------|----------|
| `lat` | numeric | Latitude of location | Decimal degrees | No |
| `lon` | numeric | Longitude of location | Decimal degrees | No |
| `sender_name` | text | Alert source organization | e.g., "NWS Philadelphia" | No |
| `event` | text | Alert type/event name | e.g., "Coastal Flood Warning" | No |
| `start` | bigint | Alert start time | Unix timestamp (UTC) | No |
| `end` | bigint | Alert end time | Unix timestamp (UTC) | No |
| `description` | text | Full alert description | Multi-paragraph text | No |
| `tags` | text | Alert categories | Comma-separated tags | Yes |

### Common Alert Types

**Severe Weather:**
- Thunderstorm Warning
- Tornado Warning/Watch
- Severe Thunderstorm Warning

**Winter Weather:**
- Winter Storm Warning
- Ice Storm Warning
- Blizzard Warning

**Heat/Cold:**
- Excessive Heat Warning
- Heat Advisory
- Wind Chill Warning

**Flooding:**
- Flood Warning
- Flash Flood Warning
- Coastal Flood Warning

**Wind:**
- High Wind Warning
- Gale Warning
- Hurricane Warning

**Other:**
- Air Quality Alert
- Fire Weather Watch
- Dense Fog Advisory

### Tags Format

Comma-separated categories such as:
- "Flood", "Extreme temperature", "Wind"
- "Rain", "Snow", "Ice"
- "Fog", "Air quality"

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |

### Optional Parameters (Server OPTIONS)
- `lang`: Language code for alert descriptions (default: "en")

## Basic Usage

```sql
-- Check for active alerts in Berlin
SELECT * FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;

-- Active alerts with human-readable times
SELECT
  sender_name,
  event,
  TO_TIMESTAMP(start) AT TIME ZONE 'UTC' as start_time,
  TO_TIMESTAMP("end") AT TIME ZONE 'UTC' as end_time,
  description
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;

-- Count active alerts
SELECT
  COUNT(*) as active_alerts,
  STRING_AGG(DISTINCT event, ', ') as alert_types
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
```

## Advanced Queries

### Alert Duration Analysis

```sql
-- Calculate alert duration and urgency
SELECT
  event,
  sender_name,
  TO_TIMESTAMP(start) AT TIME ZONE 'UTC' as start_time,
  TO_TIMESTAMP("end") AT TIME ZONE 'UTC' as end_time,
  ROUND(EXTRACT(EPOCH FROM (TO_TIMESTAMP("end") - TO_TIMESTAMP(start)))/3600, 1) as duration_hours,
  ROUND(EXTRACT(EPOCH FROM (TO_TIMESTAMP(start) - NOW()))/3600, 1) as starts_in_hours,
  CASE
    WHEN start <= EXTRACT(EPOCH FROM NOW())::bigint THEN '🚨 ACTIVE NOW'
    WHEN start - EXTRACT(EPOCH FROM NOW())::bigint < 3600 THEN '⚠️ Starting within 1 hour'
    WHEN start - EXTRACT(EPOCH FROM NOW())::bigint < 10800 THEN '⏰ Starting within 3 hours'
    ELSE '📅 Starts later'
  END as urgency
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405
ORDER BY start;
```

### Filter by Alert Severity

```sql
-- Categorize alerts by severity keywords
SELECT
  event,
  sender_name,
  TO_TIMESTAMP(start) AT TIME ZONE 'UTC' as start_time,
  CASE
    WHEN event ILIKE '%warning%' OR event ILIKE '%emergency%' THEN '🔴 High Severity'
    WHEN event ILIKE '%watch%' THEN '🟡 Medium Severity'
    WHEN event ILIKE '%advisory%' OR event ILIKE '%statement%' THEN '🟢 Low Severity'
    ELSE '⚪ Unknown Severity'
  END as severity_level,
  LEFT(description, 200) || '...' as description_preview
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405
ORDER BY
  CASE
    WHEN event ILIKE '%warning%' OR event ILIKE '%emergency%' THEN 1
    WHEN event ILIKE '%watch%' THEN 2
    WHEN event ILIKE '%advisory%' OR event ILIKE '%statement%' THEN 3
    ELSE 4
  END,
  start;
```

### Multi-Location Alert Monitoring

```sql
-- Monitor alerts for multiple cities
CREATE TABLE monitored_locations (
  city_name TEXT,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO monitored_locations VALUES
  ('Berlin', 52.52, 13.405),
  ('London', 51.5074, -0.1278),
  ('Paris', 48.8566, 2.3522);

SELECT
  l.city_name,
  COUNT(*) as alert_count,
  STRING_AGG(a.event, ', ') as alert_types
FROM monitored_locations l
LEFT JOIN LATERAL (
  SELECT event
  FROM fdw_open_weather.weather_alerts
  WHERE lat = l.lat AND lon = l.lon
) a ON TRUE
GROUP BY l.city_name;
```

### Extract Alert Tags

```sql
-- Parse and analyze alert tags
SELECT
  event,
  tags,
  CASE
    WHEN tags ILIKE '%flood%' THEN TRUE ELSE FALSE
  END as is_flood_related,
  CASE
    WHEN tags ILIKE '%extreme temperature%' THEN TRUE ELSE FALSE
  END as is_temp_related,
  CASE
    WHEN tags ILIKE '%wind%' THEN TRUE ELSE FALSE
  END as is_wind_related,
  description
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
```

## Response Characteristics

- **Rows:** 0-N (variable, depends on active alerts)
  - Most locations: 0 rows (no alerts)
  - During severe weather: 1-5 rows typical
  - Extreme events: 5+ rows possible
- **Update Frequency:** Real-time (as alerts issued/updated)
- **Response Size:** Variable (100 bytes - 10+ KB)
- **Query Time:** < 2 seconds
- **Availability:** Varies by country and region

## Data Quality Notes

### Availability by Region

**Good Coverage:**
- United States (NWS)
- Canada
- Most of Europe
- Japan, South Korea
- Australia

**Limited Coverage:**
- Parts of Asia
- Africa
- South America
- Remote/rural areas

### No Alerts vs No Data

**0 rows returned can mean:**
1. **No active alerts** - Good! (most common)
2. **No alert system** - Region doesn't have government alerts integrated
3. **Data unavailable** - Temporary API issue

To distinguish:
- Check a known location with alerts (e.g., US coastal cities during hurricane season)
- Verify with current weather endpoint

### Description Field

- Can be very long (500-2000 characters)
- May contain multiple paragraphs
- Includes detailed instructions and affected areas
- Use `LEFT(description, 200)` for previews

### Time Zones

Alert times are in UTC. Convert to local timezone for display:

```sql
SELECT
  event,
  TO_TIMESTAMP(start) AT TIME ZONE 'America/New_York' as start_local,
  TO_TIMESTAMP("end") AT TIME ZONE 'America/New_York' as end_local
FROM fdw_open_weather.weather_alerts
WHERE lat = 40.7128 AND lon = -74.0060;
```

## Common Use Cases

### Emergency Alert System

```sql
-- Real-time emergency alert dashboard
WITH active_alerts AS (
  SELECT
    event,
    sender_name,
    TO_TIMESTAMP(start) AT TIME ZONE 'UTC' as start_time,
    TO_TIMESTAMP("end") AT TIME ZONE 'UTC' as end_time,
    description,
    CASE
      WHEN event ILIKE '%tornado%' OR event ILIKE '%hurricane%' THEN 1
      WHEN event ILIKE '%warning%' THEN 2
      WHEN event ILIKE '%watch%' THEN 3
      ELSE 4
    END as priority
  FROM fdw_open_weather.weather_alerts
  WHERE lat = 52.52 AND lon = 13.405
    AND "end" > EXTRACT(EPOCH FROM NOW())::bigint
)
SELECT
  CASE priority
    WHEN 1 THEN '🚨 EXTREME ALERT'
    WHEN 2 THEN '🔴 WARNING'
    WHEN 3 THEN '🟡 WATCH'
    ELSE '🔵 ADVISORY'
  END as alert_level,
  event,
  sender_name,
  start_time,
  end_time,
  LEFT(description, 300) || '...' as summary
FROM active_alerts
ORDER BY priority, start_time;
```

### SMS/Email Alert Notifications

```sql
-- Generate alert notifications for subscribers
SELECT
  'WEATHER ALERT: ' || event ||
  ' issued by ' || sender_name ||
  '. Active from ' || TO_CHAR(TO_TIMESTAMP(start), 'Mon DD HH24:MI') ||
  ' to ' || TO_CHAR(TO_TIMESTAMP("end"), 'Mon DD HH24:MI') ||
  '. ' || LEFT(description, 200) as notification_message,
  CASE
    WHEN event ILIKE '%warning%' THEN 'high'
    WHEN event ILIKE '%watch%' THEN 'medium'
    ELSE 'low'
  END as priority
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405
  AND start <= EXTRACT(EPOCH FROM NOW() + INTERVAL '24 hours')::bigint;
```

### Alert History Tracking

```sql
-- Log alerts to a permanent table
CREATE TABLE alert_history (
  logged_at TIMESTAMP DEFAULT NOW(),
  lat NUMERIC,
  lon NUMERIC,
  event TEXT,
  sender_name TEXT,
  start_time TIMESTAMP,
  end_time TIMESTAMP,
  description TEXT,
  tags TEXT
);

-- Insert current alerts
INSERT INTO alert_history (lat, lon, event, sender_name, start_time, end_time, description, tags)
SELECT
  lat,
  lon,
  event,
  sender_name,
  TO_TIMESTAMP(start),
  TO_TIMESTAMP("end"),
  description,
  tags
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
```

### Combine Alerts with Forecast

```sql
-- Show alerts alongside daily forecast
SELECT
  TO_TIMESTAMP(d.dt)::DATE as date,
  d.weather_description as forecast,
  ROUND(d.temp_max, 1) as max_temp_c,
  COALESCE(a.event, 'No alerts') as alert,
  COALESCE(LEFT(a.description, 100), '-') as alert_details
FROM fdw_open_weather.daily_forecast d
LEFT JOIN fdw_open_weather.weather_alerts a
  ON d.lat = a.lat
  AND d.lon = a.lon
  AND d.dt BETWEEN a.start AND a."end"
WHERE d.lat = 52.52 AND d.lon = 13.405
ORDER BY d.dt;
```

## Caching Strategy

Due to real-time nature, cache for short duration only:

```sql
-- Create materialized view (refreshed every 15 minutes)
CREATE MATERIALIZED VIEW mv_weather_alerts_cache AS
SELECT *
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh every 15 minutes
SELECT cron.schedule('refresh-weather-alerts', '*/15 * * * *',
  'REFRESH MATERIALIZED VIEW mv_weather_alerts_cache');
```

**Note:** For critical alerts, query directly (don't use cache).

## Troubleshooting

### Always Returns 0 Rows

**This is normal** - most locations most of the time have no active alerts.

To verify alerts are working:
1. Test during known severe weather events
2. Check a location with frequent alerts (e.g., US Gulf Coast)
3. Compare with hourly/daily forecasts for severe weather indicators

### Missing Expected Alert

**Possible causes:**
1. Alert not yet issued by government agency
2. Alert outside geographic scope of provided lat/lon
3. API hasn't updated yet (usually updates within minutes)

### Description Too Long

Truncate for display:

```sql
SELECT
  event,
  CASE
    WHEN LENGTH(description) > 200
      THEN LEFT(description, 200) || '... [truncated]'
    ELSE description
  END as description_preview
FROM fdw_open_weather.weather_alerts
WHERE lat = 52.52 AND lon = 13.405;
```

## See Also

- **[Endpoints Overview](README.md)** - All endpoints comparison
- **[Current Weather](current-weather.md)** - Current conditions
- **[Hourly Forecast](hourly-forecast.md)** - 48-hour forecast
- **[Daily Forecast](daily-forecast.md)** - 8-day forecast
- **[API Overview](../reference/API_OVERVIEW.md)** - OpenWeather API details
- **[QUICKSTART](../../QUICKSTART.md)** - Setup guide
