# Migration Guide: v0.3.0 → v0.3.1

## Overview

Version 0.3.1 introduces **parameter transformation** for three endpoints. This is a **minor breaking change** affecting only endpoints that require API-specific parameters.

**Affected endpoints:** `historical_weather`, `daily_summary`, `weather_overview`

**Unaffected endpoints:** `current_weather`, `minutely_forecast`, `hourly_forecast`, `daily_forecast`, `weather_alerts`

## Breaking Changes

### Historical Weather

```sql
-- ❌ Before (v0.3.0) - Non-existent column
WHERE dt = 1704067200

-- ✅ After (v0.3.1) - Semantic TIMESTAMPTZ column
WHERE observation_time = '2024-01-01 00:00:00+00'
```

### Daily Summary

```sql
-- ❌ Before (v0.3.0)
WHERE date = '2024-01-15'

-- ✅ After (v0.3.1)
WHERE summary_date = '2024-01-15'
```

### Weather Overview

```sql
-- ❌ Before (v0.3.0)
WHERE date = '2025-10-29'

-- ✅ After (v0.3.1)
WHERE overview_date = '2025-10-29'
```

## Benefits

1. **Native Temporal Operations** - Use PostgreSQL's interval arithmetic
2. **Standards Compliance** - Semantic column names match database standards
3. **API Abstraction** - No need to know about Unix timestamps

## Migration Steps

**Step 1:** Update your queries with new column names (see examples above)

**Step 2:** Update WASM binary to v0.3.1:
```sql
ALTER SERVER openweather_server OPTIONS (
  SET fdw_package_version 'v0.3.1',
  SET fdw_package_checksum '0abb03a28bce499c1fdeedd0b64c461b226a907c3bcfc6542eb6d36e951f9eee'
);
```

**Step 3:** Test affected queries

## Rollback

To rollback to v0.3.0:
```sql
ALTER SERVER openweather_server OPTIONS (
  SET fdw_package_version 'v0.3.0',
  SET fdw_package_checksum '55e0226ad0880b25f2aac2e3028f3ce6987f33f91575afd6222537af3b5c8a31'
);
```
