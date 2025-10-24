# Weather Overview Endpoint

**Endpoint:** `weather_overview`
**Status:** ✅ Production Ready (v0.2.0)
**API:** https://api.openweathermap.org/data/3.0/onecall/overview
**Section:** AI-generated weather summary

Returns human-readable, AI-generated weather summaries for today or tomorrow, perfect for non-technical audiences and public communication.

## Schema

```sql
CREATE FOREIGN TABLE fdw_open_weather.weather_overview (
  lat numeric,
  lon numeric,
  tz text,
  date text,
  units text,
  weather_overview text
)
SERVER openweather_server
OPTIONS (object 'weather_overview');
```

## Columns

| Column | Type | Description | Example | Nullable |
|--------|------|-------------|---------|----------|
| `lat` | numeric | Latitude of location | 52.52 | No |
| `lon` | numeric | Longitude of location | 13.405 | No |
| `tz` | text | Timezone offset | "+02:00" | No |
| `date` | text | Date of summary | "2025-10-24" | No |
| `units` | text | Unit system used | "metric" | No |
| `weather_overview` | text | AI-generated weather summary | "The current weather is..." | No |

## WHERE Clause Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `lat` | numeric | **Yes** | - | Latitude (-90 to 90) |
| `lon` | numeric | **Yes** | - | Longitude (-180 to 180) |
| `date` | text | No | Today | Date in YYYY-MM-DD format (today or tomorrow only) |

### Optional Parameters (Server OPTIONS)
- `units`: "metric" (default), "imperial", or "standard"
- `lang`: Language code for summary (default: "en")

## Basic Usage

```sql
-- Get today's weather overview for Berlin
SELECT weather_overview
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

-- Get tomorrow's weather overview
SELECT
  date,
  weather_overview
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52
  AND lon = 13.405
  AND date = '2025-10-25';

-- Format output for display
SELECT
  date,
  LEFT(weather_overview, 100) || '...' as preview,
  LENGTH(weather_overview) as summary_length
FROM fdw_open_weather.weather_overview
WHERE lat = 40.7128 AND lon = -74.0060;
```

## Advanced Queries

### Multi-Location Summaries

```sql
-- Get weather overviews for multiple cities
CREATE TABLE cities (
  city_name TEXT PRIMARY KEY,
  lat NUMERIC,
  lon NUMERIC
);

INSERT INTO cities VALUES
  ('Berlin', 52.52, 13.405),
  ('London', 51.5074, -0.1278),
  ('Paris', 48.8566, 2.3522),
  ('New York', 40.7128, -74.0060);

-- Query all cities
SELECT
  c.city_name,
  w.date,
  w.weather_overview
FROM cities c
CROSS JOIN LATERAL (
  SELECT date, weather_overview
  FROM fdw_open_weather.weather_overview
  WHERE lat = c.lat AND lon = c.lon
) w
ORDER BY c.city_name;
```

### Public Announcement System

```sql
-- Generate today's weather announcement
SELECT
  'Good morning! ' || weather_overview as announcement
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

-- Example output:
-- "Good morning! The current weather is partly cloudy with a temperature
--  of 11°C and a wind speed of 8 m/s coming from the southwest..."
```

### Email Newsletter Content

```sql
-- Create weather section for newsletter
WITH today AS (
  SELECT weather_overview as today_weather
  FROM fdw_open_weather.weather_overview
  WHERE lat = 52.52 AND lon = 13.405
),
tomorrow AS (
  SELECT weather_overview as tomorrow_weather
  FROM fdw_open_weather.weather_overview
  WHERE lat = 52.52 AND lon = 13.405 AND date = CURRENT_DATE + 1
)
SELECT
  'TODAY: ' || today_weather || E'\n\n' ||
  'TOMORROW: ' || tomorrow_weather
  as weather_section
FROM today, tomorrow;
```

### Mobile App Push Notifications

```sql
-- Short weather summary for push notification (first 120 chars)
SELECT
  LEFT(weather_overview, 117) || '...' as push_notification
FROM fdw_open_weather.weather_overview
WHERE lat = 37.7749 AND lon = -122.4194;
```

## Sample Outputs

### Today's Weather (Partly Cloudy)

```
The current weather is partly cloudy with a temperature of 11°C and
a wind speed of 8 m/s coming from the southwest. The air pressure is
at 1000 hPa, and the humidity is at 66%. The visibility is at 10,000
meters, and the UV index is 0. With broken clouds in the sky, it
feels like 10°C due to the wind chill. Overall, it's a cool and
breezy evening with some clouds in the sky. So, if you're heading
out, you might want to grab a light jacket to stay comfortable in
the cool breeze.
```

### Tomorrow's Weather (Rainy)

```
Tomorrow's weather will bring a mix of partly cloudy skies and rain,
with temperatures ranging from 8 to 11 degrees Celsius. The day will
start off at 8 degrees in the morning and reach a high of 11 degrees
during the day. The wind speed will be around 7 meters per second,
with gusts up to 15 meters per second coming from the southwest. The
humidity will be at 62%, and there is a high chance of light rain,
with a 87% probability of precipitation. Overall, it will be a cool
and damp day, so be sure to dress accordingly and carry an umbrella
if you plan to be out and about.
```

## Use Cases

### 1. Website Weather Widgets

```sql
-- Simple weather widget
SELECT
  'Weather Today' as title,
  weather_overview as description
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;
```

### 2. Chatbot Responses

```sql
-- Weather chatbot query
SELECT
  'Here''s the weather outlook: ' || weather_overview as bot_response
FROM fdw_open_weather.weather_overview
WHERE lat = 51.5074 AND lon = -0.1278;
```

### 3. Voice Assistant Integration

```sql
-- Generate voice-friendly weather response
SELECT
  'The weather forecast says, ' || weather_overview as voice_response
FROM fdw_open_weather.weather_overview
WHERE lat = 40.7128 AND lon = -74.0060;
```

### 4. Event Planning Communications

```sql
-- Event weather update email
SELECT
  'Weather Update for Your Event on ' || date || E':\n\n' ||
  weather_overview ||
  E'\n\nPlease plan accordingly.'
  as event_email
FROM fdw_open_weather.weather_overview
WHERE lat = 48.8566
  AND lon = 2.3522
  AND date = '2024-08-15';
```

### 5. Tourism Applications

```sql
-- Tourist destination weather
SELECT
  l.destination_name,
  'Weather in ' || l.destination_name || ': ' || w.weather_overview as description
FROM tourist_destinations l
CROSS JOIN LATERAL (
  SELECT weather_overview
  FROM fdw_open_weather.weather_overview
  WHERE lat = l.lat AND lon = l.lon
) w
WHERE l.destination_name = 'Paris';
```

## Multilingual Support

The endpoint supports 50+ languages via the `lang` parameter in server OPTIONS:

```sql
-- Create server with German language
CREATE SERVER openweather_de
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://host.docker.internal:8000/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version '0.2.0',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key',
    lang 'de'  -- German summaries
  );

-- Create table using German server
CREATE FOREIGN TABLE fdw_open_weather.wetter_uebersicht (
  lat numeric,
  lon numeric,
  tz text,
  date text,
  units text,
  weather_overview text
)
SERVER openweather_de
OPTIONS (object 'weather_overview');

-- Query in German
SELECT weather_overview
FROM fdw_open_weather.wetter_uebersicht
WHERE lat = 52.52 AND lon = 13.405;
```

## Performance Tips

### 1. Cache with Materialized Views

```sql
-- Cache today's overview (refresh every hour)
CREATE MATERIALIZED VIEW mv_today_weather AS
SELECT * FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

-- Refresh with pg_cron
SELECT cron.schedule(
  'refresh-weather-overview',
  '0 * * * *',  -- Every hour
  'REFRESH MATERIALIZED VIEW mv_today_weather'
);

-- Query cached data (no API call)
SELECT * FROM mv_today_weather;
```

### 2. Store for Display

```sql
-- Store summaries in application table
CREATE TABLE weather_summaries (
  location_id INTEGER,
  summary_date DATE,
  weather_text TEXT,
  fetched_at TIMESTAMPTZ DEFAULT NOW()
);

-- Populate from FDW
INSERT INTO weather_summaries (location_id, summary_date, weather_text)
SELECT
  1 as location_id,
  CURRENT_DATE,
  weather_overview
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

-- Query from stored data
SELECT weather_text FROM weather_summaries
WHERE location_id = 1 AND summary_date = CURRENT_DATE;
```

## Data Characteristics

- **Coverage:** Today and tomorrow only
- **Update Frequency:** Updated with latest forecast data
- **Summary Length:** Typically 200-500 characters
- **Language:** Natural, conversational tone
- **Content:** Temperature, wind, humidity, pressure, visibility, UV, conditions, recommendations

## Troubleshooting

### Empty or Generic Response

**Possible causes:**
- API temporarily unavailable
- Invalid coordinates
- Language not supported

**Solution:**
```sql
-- Verify coordinates
SELECT * FROM fdw_open_weather.weather_overview
WHERE lat BETWEEN -90 AND 90
  AND lon BETWEEN -180 AND 180;
```

### Date Out of Range

**Error:**
```
data not found
```

**Solution:**
Only today and tomorrow are supported:
```sql
-- Today (no date parameter)
WHERE lat = 52.52 AND lon = 13.405

-- Tomorrow
WHERE lat = 52.52 AND lon = 13.405 AND date = CURRENT_DATE + 1
```

### Summary Too Long for Display

**Solution:**
Truncate to desired length:
```sql
-- First 150 characters
SELECT LEFT(weather_overview, 147) || '...' as short_summary
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;

-- Extract first sentence
SELECT
  SPLIT_PART(weather_overview, '.', 1) || '.' as first_sentence
FROM fdw_open_weather.weather_overview
WHERE lat = 52.52 AND lon = 13.405;
```

## Comparison with Other Endpoints

| Feature | weather_overview | current_weather | daily_forecast |
|---------|-----------------|-----------------|----------------|
| **Format** | Natural language | Structured data | Structured data |
| **Audience** | General public | Technical users | Technical users |
| **Detail Level** | Narrative summary | Precise values | Daily aggregates |
| **Time Range** | Today/tomorrow | Current moment | 8 days ahead |
| **Use Case** | Communication | Analysis | Planning |

## Related Endpoints

- **[current_weather](current-weather.md)** - Structured current conditions
- **[daily_forecast](daily-forecast.md)** - 8-day structured forecast
- **[hourly_forecast](hourly-forecast.md)** - 48-hour detailed data

## API Rate Limits

- **Free Plan:** 1,000 calls/day
- **Note:** Each query = 1 API call
- **Recommendation:** Cache overviews for reuse

## See Also

- [OpenWeather One Call API 3.0 Documentation](https://openweathermap.org/api/one-call-3#weather_overview)
- [API Overview](../reference/API_OVERVIEW.md)
- [Troubleshooting Guide](../guides/TROUBLESHOOTING.md)
