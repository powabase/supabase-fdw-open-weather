# Deployment Guide

Deploy OpenWeather WASM FDW to your Supabase project.

## Prerequisites

- Supabase project (local or hosted)
- OpenWeather API key ([free tier available](https://openweathermap.org/api/one-call-3))

## Local Testing

### Step 1: Start Supabase
```bash
supabase start
```

### Step 2: Serve WASM Binary
```bash
cd target/wasm32-unknown-unknown/release
python3 -m http.server 8000
```

### Step 3: Connect and Setup
```bash
psql postgresql://postgres:postgres@127.0.0.1:54322/postgres
```

```sql
-- Create server (use host.docker.internal for local testing)
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'http://host.docker.internal:8000/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.1',
    fdw_package_checksum 'your_checksum_here',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key_here'
  );

-- Import schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;
IMPORT FOREIGN SCHEMA public
  FROM SERVER openweather_server
  INTO fdw_open_weather;
```

### Step 4: Test
```sql
SELECT * FROM fdw_open_weather.current_weather
WHERE latitude = 52.52 AND longitude = 13.405;
```

## Production Deployment

### Step 1: Upload WASM Binary
Upload `open_weather_fdw.wasm` to a public URL (GitHub Releases recommended).

### Step 2: Get Checksum
```bash
shasum -a 256 open_weather_fdw.wasm
```

### Step 3: Deploy to Supabase
```sql
CREATE SERVER openweather_server
  FOREIGN DATA WRAPPER wasm_wrapper
  OPTIONS (
    fdw_package_url 'https://github.com/your-org/your-repo/releases/download/v0.3.1/open_weather_fdw.wasm',
    fdw_package_name 'powabase:supabase-fdw-open-weather',
    fdw_package_version 'v0.3.1',
    fdw_package_checksum 'your_sha256_checksum',
    api_url 'https://api.openweathermap.org/data/3.0',
    api_key 'your_api_key_here'
  );

-- Import schema
CREATE SCHEMA IF NOT EXISTS fdw_open_weather;
IMPORT FOREIGN SCHEMA public
  FROM SERVER openweather_server
  INTO fdw_open_weather;
```

## Important Notes

- **Local testing:** Use `http://host.docker.internal:8000/...` for Docker containers
- **Checksum:** Find current checksum in [README.md](../../README.md#release-information)
- **API Key:** Keep your API key secure, use environment variables in production

## Troubleshooting

If you encounter issues, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
