// OpenWeather WASM FDW - Multi-Endpoint Wrapper
//
// This wrapper enables querying OpenWeather One Call API 3.0 endpoints
// as PostgreSQL foreign tables using WASM FDW.
//
// Supported endpoints (v0.2.0 - 8 tables from 4 API endpoints):
// - current_weather: Current weather conditions (1 row)
//   API: /onecall → parses 'current' section
//   Parameters: lat, lon, units (optional), lang (optional)
//
// - minutely_forecast: Minute-by-minute precipitation forecast (60 rows)
//   API: /onecall → parses 'minutely' array
//   Parameters: lat, lon, units (optional), lang (optional)
//
// - hourly_forecast: Hourly forecast for 48 hours (48 rows)
//   API: /onecall → parses 'hourly' array
//   Parameters: lat, lon, units (optional), lang (optional)
//
// - daily_forecast: Daily forecast for 8 days (8 rows)
//   API: /onecall → parses 'daily' array
//   Parameters: lat, lon, units (optional), lang (optional)
//
// - weather_alerts: Government weather alerts (0-N rows)
//   API: /onecall → parses 'alerts' array
//   Parameters: lat, lon, units (optional), lang (optional)
//
// - historical_weather: Historical weather data (1 row)
//   API: /onecall/timemachine → parses 'data[0]'
//   Parameters: lat, lon, dt (unix timestamp), units (optional), lang (optional)
//
// - daily_summary: Daily aggregated weather statistics (1 row)
//   API: /onecall/day_summary → parses nested aggregations
//   Parameters: lat, lon, date (YYYY-MM-DD), tz (optional), units (optional), lang (optional)
//
// - weather_overview: AI-generated weather summary (1 row)
//   API: /onecall/overview → parses AI summary text
//   Parameters: lat, lon, date (optional), units (optional), lang (optional)
//
// API Documentation: https://openweathermap.org/api/one-call-3
// Implementation Plan: docs/IMPLEMENTATION_PLAN.md

#[allow(warnings)]
mod bindings;

use serde_json::Value as JsonValue;

use bindings::{
    exports::supabase::wrappers::routines::Guest,
    supabase::wrappers::{
        http, stats,
        types::{
            Cell, Column, Context, FdwError, FdwResult, ImportForeignSchemaStmt, OptionsType, Row,
            Value,
        },
        utils,
    },
};

/// Supported OpenWeather One Call API 3.0 endpoints
/// v0.2.0: 8 foreign tables mapping to 4 API endpoints
#[derive(Debug, Clone, Copy, PartialEq)]
enum EndpointType {
    CurrentWeather,    // /onecall → current
    MinutelyForecast,  // /onecall → minutely[]
    HourlyForecast,    // /onecall → hourly[]
    DailyForecast,     // /onecall → daily[]
    WeatherAlerts,     // /onecall → alerts[]
    HistoricalWeather, // /onecall/timemachine → data[0]
    DailySummary,      // /onecall/day_summary → daily aggregations
    WeatherOverview,   // /onecall/overview → AI weather summary
}

impl EndpointType {
    /// Parse endpoint type from OPTIONS object parameter
    fn from_object_name(name: &str) -> Result<Self, FdwError> {
        match name {
            "current_weather" => Ok(EndpointType::CurrentWeather),
            "minutely_forecast" => Ok(EndpointType::MinutelyForecast),
            "hourly_forecast" => Ok(EndpointType::HourlyForecast),
            "daily_forecast" => Ok(EndpointType::DailyForecast),
            "weather_alerts" => Ok(EndpointType::WeatherAlerts),
            "historical_weather" => Ok(EndpointType::HistoricalWeather),
            "daily_summary" => Ok(EndpointType::DailySummary),
            "weather_overview" => Ok(EndpointType::WeatherOverview),
            _ => Err(format!("unsupported endpoint object '{}'. Supported: current_weather, minutely_forecast, hourly_forecast, daily_forecast, weather_alerts, historical_weather, daily_summary, weather_overview", name)),
        }
    }

    /// Get API endpoint path
    fn api_path(&self) -> &'static str {
        match self {
            EndpointType::CurrentWeather
            | EndpointType::MinutelyForecast
            | EndpointType::HourlyForecast
            | EndpointType::DailyForecast
            | EndpointType::WeatherAlerts => "/onecall",
            EndpointType::HistoricalWeather => "/onecall/timemachine",
            EndpointType::DailySummary => "/onecall/day_summary",
            EndpointType::WeatherOverview => "/onecall/overview",
        }
    }

    /// Check if endpoint calls /onecall (shared response parsing)
    #[allow(dead_code)]
    fn calls_onecall(&self) -> bool {
        matches!(
            self,
            EndpointType::CurrentWeather
                | EndpointType::MinutelyForecast
                | EndpointType::HourlyForecast
                | EndpointType::DailyForecast
                | EndpointType::WeatherAlerts
        )
    }
}

/// Endpoint-specific data storage
/// These schemas represent the flattened PostgreSQL output
#[derive(Debug, Default)]
#[allow(clippy::large_enum_variant)]
enum EndpointData {
    #[default]
    None,

    // /onecall → current_weather (1 row)
    CurrentWeather {
        latitude: f64,
        longitude: f64,
        timezone_name: String,
        observation_time: i64, // Unix seconds (convert to TIMESTAMPTZ in output)
        temperature_temp: f64,
        apparent_temperature_temp: f64,
        pressure_hpa: i64,
        humidity_pct: i64,
        dew_point_temp: f64,
        uv_index: f64,
        cloud_cover_pct: i64,
        visibility_m: i64,
        wind_speed_m_s: f64,
        wind_direction_deg: i64,
        wind_gust_speed_m_s: Option<f64>,
        weather_condition: String,
        weather_description: String,
        weather_icon_code: String,
    },

    // /onecall → minutely (60 rows)
    MinutelyForecast {
        latitude: f64,
        longitude: f64,
        forecast_time: Vec<i64>, // Unix seconds (convert to TIMESTAMPTZ in output)
        precipitation_mm: Vec<f64>,
    },

    // /onecall → hourly (48 rows)
    HourlyForecast {
        latitude: f64,
        longitude: f64,
        forecast_time: Vec<i64>, // Unix seconds (convert to TIMESTAMPTZ in output)
        temperature_temp: Vec<f64>,
        apparent_temperature_temp: Vec<f64>,
        pressure_hpa: Vec<i64>,
        humidity_pct: Vec<i64>,
        dew_point_temp: Vec<f64>,
        uv_index: Vec<f64>,
        cloud_cover_pct: Vec<i64>,
        visibility_m: Vec<i64>,
        wind_speed_m_s: Vec<f64>,
        wind_direction_deg: Vec<i64>,
        wind_gust_speed_m_s: Vec<Option<f64>>,
        precipitation_probability: Vec<f64>,
        rain_volume_1h_mm: Vec<Option<f64>>,
        snow_volume_1h_mm: Vec<Option<f64>>,
        weather_condition: Vec<String>,
        weather_description: Vec<String>,
        weather_icon_code: Vec<String>,
    },

    // /onecall → daily (8 rows)
    DailyForecast {
        latitude: f64,
        longitude: f64,
        forecast_date: Vec<i64>, // Unix seconds (convert to TIMESTAMPTZ in output)
        sunrise_time: Vec<i64>,  // Unix seconds (convert to TIMESTAMPTZ in output)
        sunset_time: Vec<i64>,   // Unix seconds (convert to TIMESTAMPTZ in output)
        moonrise_time: Vec<i64>, // Unix seconds (convert to TIMESTAMPTZ in output)
        moonset_time: Vec<i64>,  // Unix seconds (convert to TIMESTAMPTZ in output)
        moon_phase_fraction: Vec<f64>,
        temperature_day_temp: Vec<f64>,
        temperature_min_temp: Vec<f64>,
        temperature_max_temp: Vec<f64>,
        temperature_night_temp: Vec<f64>,
        temperature_evening_temp: Vec<f64>,
        temperature_morning_temp: Vec<f64>,
        apparent_temperature_day_temp: Vec<f64>,
        apparent_temperature_night_temp: Vec<f64>,
        apparent_temperature_evening_temp: Vec<f64>,
        apparent_temperature_morning_temp: Vec<f64>,
        pressure_hpa: Vec<i64>,
        humidity_pct: Vec<i64>,
        dew_point_temp: Vec<f64>,
        wind_speed_m_s: Vec<f64>,
        wind_direction_deg: Vec<i64>,
        wind_gust_speed_m_s: Vec<Option<f64>>,
        cloud_cover_pct: Vec<i64>,
        precipitation_probability: Vec<f64>,
        rain_volume_mm: Vec<Option<f64>>,
        snow_volume_mm: Vec<Option<f64>>,
        uv_index: Vec<f64>,
        weather_condition: Vec<String>,
        weather_description: Vec<String>,
        weather_icon_code: Vec<String>,
    },

    // /onecall → alerts (0-N rows)
    WeatherAlerts {
        latitude: f64,
        longitude: f64,
        alerts: Vec<AlertRow>,
    },

    // /onecall/timemachine (1 row)
    HistoricalWeather {
        latitude: f64,
        longitude: f64,
        observation_time: i64, // Unix seconds (convert to TIMESTAMPTZ in output)
        temperature_temp: f64,
        apparent_temperature_temp: f64,
        pressure_hpa: i64,
        humidity_pct: i64,
        dew_point_temp: f64,
        cloud_cover_pct: i64,
        visibility_m: i64,
        wind_speed_m_s: f64,
        wind_direction_deg: i64,
        weather_condition: String,
        weather_description: String,
        weather_icon_code: String,
    },

    // /onecall/day_summary (1 row)
    DailySummary {
        latitude: f64,
        longitude: f64,
        timezone_offset: String,
        summary_date: String,
        unit_system: String,
        temperature_min_temp: f64,
        temperature_max_temp: f64,
        temperature_morning_temp: f64,
        temperature_afternoon_temp: f64,
        temperature_evening_temp: f64,
        temperature_night_temp: f64,
        cloud_cover_afternoon_pct: f64,
        humidity_afternoon_pct: f64,
        pressure_afternoon_hpa: f64,
        precipitation_total_mm: f64,
        wind_max_speed_m_s: f64,
        wind_max_direction_deg: f64,
    },

    // /onecall/overview (1 row)
    WeatherOverview {
        latitude: f64,
        longitude: f64,
        timezone_offset: String,
        overview_date: String,
        unit_system: String,
        weather_overview: String,
    },
}

/// Helper struct for weather alerts
#[derive(Debug, Clone)]
struct AlertRow {
    alert_sender_name: String,
    alert_event_type: String,
    alert_start_time: i64, // Unix seconds (convert to TIMESTAMPTZ in output)
    alert_end_time: i64,   // Unix seconds (convert to TIMESTAMPTZ in output)
    alert_description: String,
    alert_tags: Vec<String>,
}

impl EndpointData {
    /// Get the number of rows in this dataset
    fn row_count(&self) -> usize {
        match self {
            EndpointData::None => 0,
            EndpointData::CurrentWeather { .. } => 1,
            EndpointData::MinutelyForecast { forecast_time, .. } => forecast_time.len(),
            EndpointData::HourlyForecast { forecast_time, .. } => forecast_time.len(),
            EndpointData::DailyForecast { forecast_date, .. } => forecast_date.len(),
            EndpointData::WeatherAlerts { alerts, .. } => alerts.len(),
            EndpointData::HistoricalWeather { .. } => 1,
            EndpointData::DailySummary { .. } => 1,
            EndpointData::WeatherOverview { .. } => 1,
        }
    }

    /// Clear all data
    #[allow(dead_code)]
    fn clear(&mut self) {
        *self = EndpointData::None;
    }
}

/// FDW instance state
#[derive(Debug, Default)]
struct OpenWeatherFdw {
    /// API base URL
    base_url: String,
    /// API key
    api_key: String,
    /// HTTP headers for requests
    headers: Vec<(String, String)>,
    /// Current endpoint type
    endpoint_type: Option<EndpointType>,
    /// Endpoint-specific cached data
    data: EndpointData,
    /// Query parameters from WHERE clause
    latitude: f64,
    longitude: f64,
    units: String,                   // "metric", "imperial", or "standard"
    lang: String,                    // "en", "de", "es", etc.
    dt: Option<i64>,                 // Unix timestamp (historical_weather)
    date: Option<String>,            // YYYY-MM-DD date (daily_summary, weather_overview)
    timezone_offset: Option<String>, // Timezone offset +/-HHMM (daily_summary)
    /// Current row index for iteration
    current_row: usize,
}

// Global state (required by WASM FDW interface)
static mut INSTANCE: *mut OpenWeatherFdw = std::ptr::null_mut();
static FDW_NAME: &str = "OpenWeatherFdw";

impl OpenWeatherFdw {
    fn init() {
        let instance = Self::default();
        unsafe {
            INSTANCE = Box::leak(Box::new(instance));
        }
    }

    fn this_mut() -> &'static mut Self {
        unsafe { &mut (*INSTANCE) }
    }

    /// Extract numeric parameter from WHERE clause (for lat, lon, dt)
    fn extract_qual_numeric(
        quals: &[bindings::supabase::wrappers::types::Qual],
        field: &str,
    ) -> Option<f64> {
        for qual in quals {
            if qual.field() == field && qual.operator() == "=" {
                return match qual.value() {
                    Value::Cell(Cell::F64(n)) => Some(n),
                    Value::Cell(Cell::I64(n)) => Some(n as f64),
                    Value::Cell(Cell::I32(n)) => Some(n as f64),
                    Value::Cell(Cell::Numeric(n)) => Some(n),
                    _ => None,
                };
            }
        }
        None
    }

    /// Extract string parameter from WHERE clause (for units, lang, date)
    fn extract_qual_string(
        quals: &[bindings::supabase::wrappers::types::Qual],
        field: &str,
    ) -> Option<String> {
        quals
            .iter()
            .find(|q| q.field() == field && q.operator() == "=")
            .and_then(|q| match q.value() {
                Value::Cell(Cell::String(s)) => Some(s),
                _ => None,
            })
    }

    /// Extract TIMESTAMPTZ parameter from WHERE clause (returns microseconds)
    fn extract_qual_timestamptz(
        quals: &[bindings::supabase::wrappers::types::Qual],
        field: &str,
    ) -> Option<i64> {
        quals
            .iter()
            .find(|q| q.field() == field && q.operator() == "=")
            .and_then(|q| match q.value() {
                Value::Cell(Cell::Timestamptz(ts)) => Some(ts),
                _ => None,
            })
    }

    /// Extract and validate location from WHERE clause
    fn extract_and_validate_location(
        quals: &[bindings::supabase::wrappers::types::Qual],
    ) -> Result<(f64, f64), FdwError> {
        let latitude = Self::extract_qual_numeric(quals, "latitude").ok_or(
            "WHERE clause must include 'latitude' between -90 and 90. \
             Example: WHERE latitude = 52.52 AND longitude = 13.405",
        )?;

        let longitude = Self::extract_qual_numeric(quals, "longitude").ok_or(
            "WHERE clause must include 'longitude' between -180 and 180. \
             Example: WHERE latitude = 52.52 AND longitude = 13.405",
        )?;

        // Validate ranges
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(format!(
                "latitude must be between -90 and 90, got {}. Example: WHERE latitude = 52.52",
                latitude
            ));
        }

        if !(-180.0..=180.0).contains(&longitude) {
            return Err(format!(
                "longitude must be between -180 and 180, got {}. Example: WHERE longitude = 13.405",
                longitude
            ));
        }

        Ok((latitude, longitude))
    }

    /// Create HTTP request for OpenWeather API based on endpoint type
    fn create_request(&self) -> Result<http::Request, FdwError> {
        let endpoint_type = self
            .endpoint_type
            .ok_or("endpoint type not set - call begin_scan first")?;

        let api_path = endpoint_type.api_path();

        // Build URL with appropriate query parameters
        let url = match endpoint_type {
            EndpointType::CurrentWeather
            | EndpointType::MinutelyForecast
            | EndpointType::HourlyForecast
            | EndpointType::DailyForecast
            | EndpointType::WeatherAlerts => {
                format!(
                    "{}{}?lat={}&lon={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.latitude,
                    self.longitude,
                    self.api_key,
                    self.units,
                    self.lang
                )
            }
            EndpointType::HistoricalWeather => {
                let dt = self.dt.ok_or("observation_time parameter required for historical_weather. Example: WHERE latitude = 52.52 AND longitude = 13.405 AND observation_time = '2024-01-01 00:00:00+00'")?;
                format!(
                    "{}{}?lat={}&lon={}&dt={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.latitude,
                    self.longitude,
                    dt,
                    self.api_key,
                    self.units,
                    self.lang
                )
            }
            EndpointType::DailySummary => {
                let date = self.date.as_ref().ok_or("summary_date parameter required for daily_summary (YYYY-MM-DD format). Example: WHERE latitude = 52.52 AND longitude = 13.405 AND summary_date = '2024-01-15'")?;
                let mut url = format!(
                    "{}{}?lat={}&lon={}&date={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.latitude,
                    self.longitude,
                    date,
                    self.api_key,
                    self.units,
                    self.lang
                );
                // Add optional timezone_offset parameter
                if let Some(ref tz) = self.timezone_offset {
                    url.push_str(&format!("&tz={}", tz));
                }
                url
            }
            EndpointType::WeatherOverview => {
                let mut url = format!(
                    "{}{}?lat={}&lon={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.latitude,
                    self.longitude,
                    self.api_key,
                    self.units,
                    self.lang
                );
                // Add optional date parameter (defaults to today if omitted)
                if let Some(ref date) = self.date {
                    url.push_str(&format!("&date={}", date));
                }
                url
            }
        };

        Ok(http::Request {
            method: http::Method::Get,
            url,
            headers: self.headers.clone(),
            body: String::default(),
        })
    }

    /// Parse current weather from /onecall response
    fn parse_current_weather(&mut self, resp_json: &JsonValue) -> FdwResult {
        let current = resp_json
            .get("current")
            .ok_or("missing 'current' object in /onecall response")?;

        // Extract scalar values using .get() for safe access
        let dt = current
            .get("dt")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'dt' in current")?;

        let temp = current
            .get("temp")
            .and_then(|v| v.as_f64())
            .ok_or("missing 'temp' in current")?;

        let feels_like = current
            .get("feels_like")
            .and_then(|v| v.as_f64())
            .ok_or("missing 'feels_like' in current")?;

        let pressure = current
            .get("pressure")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'pressure' in current")?;

        let humidity = current
            .get("humidity")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'humidity' in current")?;

        let dew_point = current
            .get("dew_point")
            .and_then(|v| v.as_f64())
            .ok_or("missing 'dew_point' in current")?;

        let uvi = current
            .get("uvi")
            .and_then(|v| v.as_f64())
            .ok_or("missing 'uvi' in current")?;

        let clouds = current
            .get("clouds")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'clouds' in current")?;

        let visibility = current
            .get("visibility")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'visibility' in current")?;

        let wind_speed = current
            .get("wind_speed")
            .and_then(|v| v.as_f64())
            .ok_or("missing 'wind_speed' in current")?;

        let wind_deg = current
            .get("wind_deg")
            .and_then(|v| v.as_i64())
            .ok_or("missing 'wind_deg' in current")?;

        let wind_gust = current.get("wind_gust").and_then(|v| v.as_f64());

        // CRITICAL: Extract weather from array[0]
        let weather_arr = current
            .get("weather")
            .and_then(|v| v.as_array())
            .ok_or("missing 'weather' array in current")?;

        let weather = weather_arr.first().ok_or("weather array is empty")?;

        let weather_main = weather
            .get("main")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let weather_description = weather
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let weather_icon = weather
            .get("icon")
            .and_then(|v| v.as_str())
            .unwrap_or("01d")
            .to_string();

        // Store data
        self.data = EndpointData::CurrentWeather {
            latitude: self.latitude,
            longitude: self.longitude,
            timezone_name: resp_json
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("UTC")
                .to_string(),
            observation_time: dt,
            temperature_temp: temp,
            apparent_temperature_temp: feels_like,
            pressure_hpa: pressure,
            humidity_pct: humidity,
            dew_point_temp: dew_point,
            uv_index: uvi,
            cloud_cover_pct: clouds,
            visibility_m: visibility,
            wind_speed_m_s: wind_speed,
            wind_direction_deg: wind_deg,
            wind_gust_speed_m_s: wind_gust,
            weather_condition: weather_main,
            weather_description,
            weather_icon_code: weather_icon,
        };

        Ok(())
    }

    /// Parse minutely forecast from /onecall response
    fn parse_minutely_forecast(&mut self, resp_json: &JsonValue) -> FdwResult {
        let minutely_arr = resp_json
            .get("minutely")
            .and_then(|v| v.as_array())
            .ok_or("missing 'minutely' array in /onecall response")?;

        let mut timestamps = Vec::with_capacity(minutely_arr.len());
        let mut precipitation = Vec::with_capacity(minutely_arr.len());

        for item in minutely_arr {
            if let Some(dt) = item.get("dt").and_then(|v| v.as_i64()) {
                let precip = item
                    .get("precipitation")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                timestamps.push(dt);
                precipitation.push(precip);
            }
        }

        self.data = EndpointData::MinutelyForecast {
            latitude: self.latitude,
            longitude: self.longitude,
            forecast_time: timestamps,
            precipitation_mm: precipitation,
        };

        utils::report_info(&format!(
            "Parsed {} minutely forecast data points",
            self.data.row_count()
        ));

        Ok(())
    }

    /// Parse hourly forecast from /onecall response
    fn parse_hourly_forecast(&mut self, resp_json: &JsonValue) -> FdwResult {
        let hourly_arr = resp_json
            .get("hourly")
            .and_then(|v| v.as_array())
            .ok_or("missing 'hourly' array")?;

        let capacity = hourly_arr.len();
        let mut timestamps = Vec::with_capacity(capacity);
        let mut temps = Vec::with_capacity(capacity);
        let mut feels_like = Vec::with_capacity(capacity);
        let mut pressure = Vec::with_capacity(capacity);
        let mut humidity = Vec::with_capacity(capacity);
        let mut dew_point = Vec::with_capacity(capacity);
        let mut uvi = Vec::with_capacity(capacity);
        let mut clouds = Vec::with_capacity(capacity);
        let mut visibility = Vec::with_capacity(capacity);
        let mut wind_speed = Vec::with_capacity(capacity);
        let mut wind_deg = Vec::with_capacity(capacity);
        let mut wind_gust = Vec::with_capacity(capacity);
        let mut pop = Vec::with_capacity(capacity);
        let mut rain_1h = Vec::with_capacity(capacity);
        let mut snow_1h = Vec::with_capacity(capacity);
        let mut weather_main = Vec::with_capacity(capacity);
        let mut weather_description = Vec::with_capacity(capacity);
        let mut weather_icon = Vec::with_capacity(capacity);

        for item in hourly_arr {
            timestamps.push(
                item.get("dt")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing dt")?,
            );
            temps.push(
                item.get("temp")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp")?,
            );
            feels_like.push(
                item.get("feels_like")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing feels_like")?,
            );
            pressure.push(
                item.get("pressure")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing pressure")?,
            );
            humidity.push(
                item.get("humidity")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing humidity")?,
            );
            dew_point.push(
                item.get("dew_point")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing dew_point")?,
            );
            uvi.push(
                item.get("uvi")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing uvi")?,
            );
            clouds.push(
                item.get("clouds")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing clouds")?,
            );
            visibility.push(
                item.get("visibility")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing visibility")?,
            );
            wind_speed.push(
                item.get("wind_speed")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing wind_speed")?,
            );
            wind_deg.push(
                item.get("wind_deg")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing wind_deg")?,
            );
            wind_gust.push(item.get("wind_gust").and_then(|v| v.as_f64()));
            pop.push(
                item.get("pop")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing pop")?,
            );

            // CRITICAL: Rain/snow are conditional NESTED objects
            let rain = item
                .get("rain")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("1h"))
                .and_then(|v| v.as_f64());
            rain_1h.push(rain);

            let snow = item
                .get("snow")
                .and_then(|v| v.as_object())
                .and_then(|obj| obj.get("1h"))
                .and_then(|v| v.as_f64());
            snow_1h.push(snow);

            // Extract weather from weather[0]
            let weather_arr = item
                .get("weather")
                .and_then(|v| v.as_array())
                .ok_or("missing weather array")?;
            let weather = weather_arr.first().ok_or("weather array is empty")?;
            weather_main.push(
                weather
                    .get("main")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
            );
            weather_description.push(
                weather
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            );
            weather_icon.push(
                weather
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .unwrap_or("01d")
                    .to_string(),
            );
        }

        self.data = EndpointData::HourlyForecast {
            latitude: self.latitude,
            longitude: self.longitude,
            forecast_time: timestamps,
            temperature_temp: temps,
            apparent_temperature_temp: feels_like,
            pressure_hpa: pressure,
            humidity_pct: humidity,
            dew_point_temp: dew_point,
            uv_index: uvi,
            cloud_cover_pct: clouds,
            visibility_m: visibility,
            wind_speed_m_s: wind_speed,
            wind_direction_deg: wind_deg,
            wind_gust_speed_m_s: wind_gust,
            precipitation_probability: pop,
            rain_volume_1h_mm: rain_1h,
            snow_volume_1h_mm: snow_1h,
            weather_condition: weather_main,
            weather_description,
            weather_icon_code: weather_icon,
        };

        utils::report_info(&format!(
            "Parsed {} hourly forecast data points",
            self.data.row_count()
        ));

        Ok(())
    }

    /// Parse daily forecast from /onecall response
    fn parse_daily_forecast(&mut self, resp_json: &JsonValue) -> FdwResult {
        let daily_arr = resp_json
            .get("daily")
            .and_then(|v| v.as_array())
            .ok_or("missing 'daily' array")?;

        let capacity = daily_arr.len();
        let mut timestamps = Vec::with_capacity(capacity);
        let mut sunrise = Vec::with_capacity(capacity);
        let mut sunset = Vec::with_capacity(capacity);
        let mut moonrise = Vec::with_capacity(capacity);
        let mut moonset = Vec::with_capacity(capacity);
        let mut moon_phase = Vec::with_capacity(capacity);
        let mut temp_day = Vec::with_capacity(capacity);
        let mut temp_min = Vec::with_capacity(capacity);
        let mut temp_max = Vec::with_capacity(capacity);
        let mut temp_night = Vec::with_capacity(capacity);
        let mut temp_eve = Vec::with_capacity(capacity);
        let mut temp_morn = Vec::with_capacity(capacity);
        let mut feels_like_day = Vec::with_capacity(capacity);
        let mut feels_like_night = Vec::with_capacity(capacity);
        let mut feels_like_eve = Vec::with_capacity(capacity);
        let mut feels_like_morn = Vec::with_capacity(capacity);
        let mut pressure = Vec::with_capacity(capacity);
        let mut humidity = Vec::with_capacity(capacity);
        let mut dew_point = Vec::with_capacity(capacity);
        let mut wind_speed = Vec::with_capacity(capacity);
        let mut wind_deg = Vec::with_capacity(capacity);
        let mut wind_gust = Vec::with_capacity(capacity);
        let mut clouds = Vec::with_capacity(capacity);
        let mut pop = Vec::with_capacity(capacity);
        let mut rain = Vec::with_capacity(capacity);
        let mut snow = Vec::with_capacity(capacity);
        let mut uvi = Vec::with_capacity(capacity);
        let mut weather_main = Vec::with_capacity(capacity);
        let mut weather_description = Vec::with_capacity(capacity);
        let mut weather_icon = Vec::with_capacity(capacity);

        for item in daily_arr {
            timestamps.push(
                item.get("dt")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing dt")?,
            );
            sunrise.push(
                item.get("sunrise")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing sunrise")?,
            );
            sunset.push(
                item.get("sunset")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing sunset")?,
            );
            moonrise.push(
                item.get("moonrise")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing moonrise")?,
            );
            moonset.push(
                item.get("moonset")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing moonset")?,
            );
            moon_phase.push(
                item.get("moon_phase")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing moon_phase")?,
            );

            // CRITICAL: Extract from NESTED temp object
            let temp_obj = item
                .get("temp")
                .and_then(|v| v.as_object())
                .ok_or("missing temp object")?;

            temp_day.push(
                temp_obj
                    .get("day")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.day")?,
            );
            temp_min.push(
                temp_obj
                    .get("min")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.min")?,
            );
            temp_max.push(
                temp_obj
                    .get("max")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.max")?,
            );
            temp_night.push(
                temp_obj
                    .get("night")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.night")?,
            );
            temp_eve.push(
                temp_obj
                    .get("eve")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.eve")?,
            );
            temp_morn.push(
                temp_obj
                    .get("morn")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing temp.morn")?,
            );

            // CRITICAL: Extract from NESTED feels_like object
            let feels_like_obj = item
                .get("feels_like")
                .and_then(|v| v.as_object())
                .ok_or("missing feels_like object")?;

            feels_like_day.push(
                feels_like_obj
                    .get("day")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing feels_like.day")?,
            );
            feels_like_night.push(
                feels_like_obj
                    .get("night")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing feels_like.night")?,
            );
            feels_like_eve.push(
                feels_like_obj
                    .get("eve")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing feels_like.eve")?,
            );
            feels_like_morn.push(
                feels_like_obj
                    .get("morn")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing feels_like.morn")?,
            );

            pressure.push(
                item.get("pressure")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing pressure")?,
            );
            humidity.push(
                item.get("humidity")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing humidity")?,
            );
            dew_point.push(
                item.get("dew_point")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing dew_point")?,
            );
            wind_speed.push(
                item.get("wind_speed")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing wind_speed")?,
            );
            wind_deg.push(
                item.get("wind_deg")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing wind_deg")?,
            );
            wind_gust.push(item.get("wind_gust").and_then(|v| v.as_f64()));
            clouds.push(
                item.get("clouds")
                    .and_then(|v| v.as_i64())
                    .ok_or("missing clouds")?,
            );
            pop.push(
                item.get("pop")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing pop")?,
            );
            rain.push(item.get("rain").and_then(|v| v.as_f64()));
            snow.push(item.get("snow").and_then(|v| v.as_f64()));
            uvi.push(
                item.get("uvi")
                    .and_then(|v| v.as_f64())
                    .ok_or("missing uvi")?,
            );

            // Extract weather from weather[0]
            let weather_arr = item
                .get("weather")
                .and_then(|v| v.as_array())
                .ok_or("missing weather array")?;
            let weather = weather_arr.first().ok_or("weather array is empty")?;
            weather_main.push(
                weather
                    .get("main")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string(),
            );
            weather_description.push(
                weather
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            );
            weather_icon.push(
                weather
                    .get("icon")
                    .and_then(|v| v.as_str())
                    .unwrap_or("01d")
                    .to_string(),
            );
        }

        self.data = EndpointData::DailyForecast {
            latitude: self.latitude,
            longitude: self.longitude,
            forecast_date: timestamps,
            sunrise_time: sunrise,
            sunset_time: sunset,
            moonrise_time: moonrise,
            moonset_time: moonset,
            moon_phase_fraction: moon_phase,
            temperature_day_temp: temp_day,
            temperature_min_temp: temp_min,
            temperature_max_temp: temp_max,
            temperature_night_temp: temp_night,
            temperature_evening_temp: temp_eve,
            temperature_morning_temp: temp_morn,
            apparent_temperature_day_temp: feels_like_day,
            apparent_temperature_night_temp: feels_like_night,
            apparent_temperature_evening_temp: feels_like_eve,
            apparent_temperature_morning_temp: feels_like_morn,
            pressure_hpa: pressure,
            humidity_pct: humidity,
            dew_point_temp: dew_point,
            wind_speed_m_s: wind_speed,
            wind_direction_deg: wind_deg,
            wind_gust_speed_m_s: wind_gust,
            cloud_cover_pct: clouds,
            precipitation_probability: pop,
            rain_volume_mm: rain,
            snow_volume_mm: snow,
            uv_index: uvi,
            weather_condition: weather_main,
            weather_description,
            weather_icon_code: weather_icon,
        };

        utils::report_info(&format!(
            "Parsed {} daily forecast data points",
            self.data.row_count()
        ));

        Ok(())
    }

    /// Parse weather alerts from /onecall response
    fn parse_weather_alerts(&mut self, resp_json: &JsonValue) -> FdwResult {
        // Alerts are optional - may not exist
        let alerts_arr = match resp_json.get("alerts").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                // No alerts - return empty dataset
                self.data = EndpointData::WeatherAlerts {
                    latitude: self.latitude,
                    longitude: self.longitude,
                    alerts: Vec::new(),
                };
                utils::report_info("No weather alerts for this location");
                return Ok(());
            }
        };

        let mut alerts = Vec::with_capacity(alerts_arr.len());

        for alert in alerts_arr {
            let sender_name = alert
                .get("sender_name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let event = alert
                .get("event")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let start = alert.get("start").and_then(|v| v.as_i64()).unwrap_or(0);
            let end = alert.get("end").and_then(|v| v.as_i64()).unwrap_or(0);

            let description = alert
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // Tags array
            let tags = alert
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default();

            alerts.push(AlertRow {
                alert_sender_name: sender_name,
                alert_event_type: event,
                alert_start_time: start,
                alert_end_time: end,
                alert_description: description,
                alert_tags: tags,
            });
        }

        self.data = EndpointData::WeatherAlerts {
            latitude: self.latitude,
            longitude: self.longitude,
            alerts,
        };

        utils::report_info(&format!("Parsed {} weather alerts", self.data.row_count()));

        Ok(())
    }

    /// Parse historical weather from /onecall/timemachine response
    fn parse_historical_weather(&mut self, resp_json: &JsonValue) -> FdwResult {
        // CRITICAL: Extract from data[0] NOT flat response
        let data_arr = resp_json
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or("missing 'data' array in timemachine response")?;

        let historical = data_arr.first().ok_or("data array is empty")?;

        let dt = historical
            .get("dt")
            .and_then(|v| v.as_i64())
            .ok_or("missing dt")?;
        let temp = historical
            .get("temp")
            .and_then(|v| v.as_f64())
            .ok_or("missing temp")?;
        let feels_like = historical
            .get("feels_like")
            .and_then(|v| v.as_f64())
            .ok_or("missing feels_like")?;
        let pressure = historical
            .get("pressure")
            .and_then(|v| v.as_i64())
            .ok_or("missing pressure")?;
        let humidity = historical
            .get("humidity")
            .and_then(|v| v.as_i64())
            .ok_or("missing humidity")?;
        let dew_point = historical
            .get("dew_point")
            .and_then(|v| v.as_f64())
            .ok_or("missing dew_point")?;
        let clouds = historical
            .get("clouds")
            .and_then(|v| v.as_i64())
            .ok_or("missing clouds")?;
        let visibility = historical
            .get("visibility")
            .and_then(|v| v.as_i64())
            .ok_or("missing visibility")?;
        let wind_speed = historical
            .get("wind_speed")
            .and_then(|v| v.as_f64())
            .ok_or("missing wind_speed")?;
        let wind_deg = historical
            .get("wind_deg")
            .and_then(|v| v.as_i64())
            .ok_or("missing wind_deg")?;

        // Extract weather from weather[0]
        let weather_arr = historical
            .get("weather")
            .and_then(|v| v.as_array())
            .ok_or("missing weather array")?;
        let weather = weather_arr.first().ok_or("weather array is empty")?;
        let weather_main = weather
            .get("main")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let weather_description = weather
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let weather_icon = weather
            .get("icon")
            .and_then(|v| v.as_str())
            .unwrap_or("01d")
            .to_string();

        self.data = EndpointData::HistoricalWeather {
            latitude: self.latitude,
            longitude: self.longitude,
            observation_time: dt,
            temperature_temp: temp,
            apparent_temperature_temp: feels_like,
            pressure_hpa: pressure,
            humidity_pct: humidity,
            dew_point_temp: dew_point,
            cloud_cover_pct: clouds,
            visibility_m: visibility,
            wind_speed_m_s: wind_speed,
            wind_direction_deg: wind_deg,
            weather_condition: weather_main,
            weather_description,
            weather_icon_code: weather_icon,
        };

        utils::report_info("Parsed historical weather data");

        Ok(())
    }

    fn parse_daily_summary(&mut self, resp_json: &JsonValue) -> FdwResult {
        // Extract top-level metadata
        let lat = resp_json
            .get("lat")
            .and_then(|v| v.as_f64())
            .ok_or("missing lat")?;
        let lon = resp_json
            .get("lon")
            .and_then(|v| v.as_f64())
            .ok_or("missing lon")?;
        let tz = resp_json
            .get("tz")
            .and_then(|v| v.as_str())
            .unwrap_or("+00:00")
            .to_string();
        let date = resp_json
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let units = resp_json
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("metric")
            .to_string();

        // Extract nested temperature object
        let temp_obj = resp_json
            .get("temperature")
            .and_then(|v| v.as_object())
            .ok_or("missing temperature object")?;
        let temp_min = temp_obj
            .get("min")
            .and_then(|v| v.as_f64())
            .ok_or("missing temperature.min")?;
        let temp_max = temp_obj
            .get("max")
            .and_then(|v| v.as_f64())
            .ok_or("missing temperature.max")?;
        let temp_morning = temp_obj
            .get("morning")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let temp_afternoon = temp_obj
            .get("afternoon")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let temp_evening = temp_obj
            .get("evening")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let temp_night = temp_obj
            .get("night")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Extract nested cloud_cover.afternoon
        let cloud_cover_afternoon = resp_json
            .get("cloud_cover")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("afternoon"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Extract nested humidity.afternoon
        let humidity_afternoon = resp_json
            .get("humidity")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("afternoon"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Extract nested pressure.afternoon
        let pressure_afternoon = resp_json
            .get("pressure")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("afternoon"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Extract nested precipitation.total
        let precipitation_total = resp_json
            .get("precipitation")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("total"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Extract nested wind.max.speed and wind.max.direction
        let wind_obj = resp_json
            .get("wind")
            .and_then(|v| v.as_object())
            .ok_or("missing wind object")?;
        let wind_max = wind_obj
            .get("max")
            .and_then(|v| v.as_object())
            .ok_or("missing wind.max")?;
        let wind_max_speed = wind_max
            .get("speed")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wind_max_direction = wind_max
            .get("direction")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        self.data = EndpointData::DailySummary {
            latitude: lat,
            longitude: lon,
            timezone_offset: tz,
            summary_date: date,
            unit_system: units,
            temperature_min_temp: temp_min,
            temperature_max_temp: temp_max,
            temperature_morning_temp: temp_morning,
            temperature_afternoon_temp: temp_afternoon,
            temperature_evening_temp: temp_evening,
            temperature_night_temp: temp_night,
            cloud_cover_afternoon_pct: cloud_cover_afternoon,
            humidity_afternoon_pct: humidity_afternoon,
            pressure_afternoon_hpa: pressure_afternoon,
            precipitation_total_mm: precipitation_total,
            wind_max_speed_m_s: wind_max_speed,
            wind_max_direction_deg: wind_max_direction,
        };

        utils::report_info("Parsed daily summary data");

        Ok(())
    }

    fn parse_weather_overview(&mut self, resp_json: &JsonValue) -> FdwResult {
        // Extract flat fields
        let lat = resp_json
            .get("lat")
            .and_then(|v| v.as_f64())
            .ok_or("missing lat")?;
        let lon = resp_json
            .get("lon")
            .and_then(|v| v.as_f64())
            .ok_or("missing lon")?;
        let tz = resp_json
            .get("tz")
            .and_then(|v| v.as_str())
            .unwrap_or("+00:00")
            .to_string();
        let date = resp_json
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let units = resp_json
            .get("units")
            .and_then(|v| v.as_str())
            .unwrap_or("metric")
            .to_string();
        let weather_overview = resp_json
            .get("weather_overview")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        self.data = EndpointData::WeatherOverview {
            latitude: lat,
            longitude: lon,
            timezone_offset: tz,
            overview_date: date,
            unit_system: units,
            weather_overview,
        };

        utils::report_info("Parsed weather overview data");

        Ok(())
    }

    /// Convert OpenWeather data at current row index to PostgreSQL cell
    fn get_cell_value(&self, tgt_col: &Column) -> Result<Option<Cell>, FdwError> {
        let tgt_col_name = tgt_col.name();
        let row_idx = self.current_row;

        // Check if we have data at current index
        if row_idx >= self.data.row_count() {
            return Err("row index out of bounds".to_owned());
        }

        // Map column name to data based on endpoint type
        let cell = match &self.data {
            EndpointData::CurrentWeather {
                latitude,
                longitude,
                timezone_name,
                observation_time,
                temperature_temp,
                apparent_temperature_temp,
                pressure_hpa,
                humidity_pct,
                dew_point_temp,
                uv_index,
                cloud_cover_pct,
                visibility_m,
                wind_speed_m_s,
                wind_direction_deg,
                wind_gust_speed_m_s,
                weather_condition,
                weather_description,
                weather_icon_code,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "timezone_name" => Some(Cell::String(timezone_name.clone())),
                "observation_time" => Some(Cell::Timestamptz(observation_time * 1_000_000)),
                "temperature_temp" => Some(Cell::Numeric(*temperature_temp)),
                "apparent_temperature_temp" => Some(Cell::Numeric(*apparent_temperature_temp)),
                "pressure_hpa" => Some(Cell::Numeric(*pressure_hpa as f64)),
                "humidity_pct" => Some(Cell::Numeric(*humidity_pct as f64)),
                "dew_point_temp" => Some(Cell::Numeric(*dew_point_temp)),
                "uv_index" => Some(Cell::Numeric(*uv_index)),
                "cloud_cover_pct" => Some(Cell::Numeric(*cloud_cover_pct as f64)),
                "visibility_m" => Some(Cell::Numeric(*visibility_m as f64)),
                "wind_speed_m_s" => Some(Cell::Numeric(*wind_speed_m_s)),
                "wind_direction_deg" => Some(Cell::Numeric(*wind_direction_deg as f64)),
                "wind_gust_speed_m_s" => wind_gust_speed_m_s.map(Cell::Numeric),
                "weather_condition" => Some(Cell::String(weather_condition.clone())),
                "weather_description" => Some(Cell::String(weather_description.clone())),
                "weather_icon_code" => Some(Cell::String(weather_icon_code.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for current_weather endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::MinutelyForecast {
                latitude,
                longitude,
                forecast_time,
                precipitation_mm,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "forecast_time" => forecast_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "precipitation_mm" => precipitation_mm.get(row_idx).map(|&v| Cell::Numeric(v)),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for minutely_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::HourlyForecast {
                latitude,
                longitude,
                forecast_time,
                temperature_temp,
                apparent_temperature_temp,
                pressure_hpa,
                humidity_pct,
                dew_point_temp,
                uv_index,
                cloud_cover_pct,
                visibility_m,
                wind_speed_m_s,
                wind_direction_deg,
                wind_gust_speed_m_s,
                precipitation_probability,
                rain_volume_1h_mm,
                snow_volume_1h_mm,
                weather_condition,
                weather_description,
                weather_icon_code,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "forecast_time" => forecast_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "temperature_temp" => temperature_temp.get(row_idx).map(|&v| Cell::Numeric(v)),
                "apparent_temperature_temp" => apparent_temperature_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "pressure_hpa" => pressure_hpa.get(row_idx).map(|&v| Cell::Numeric(v as f64)),
                "humidity_pct" => humidity_pct.get(row_idx).map(|&v| Cell::Numeric(v as f64)),
                "dew_point_temp" => dew_point_temp.get(row_idx).map(|&v| Cell::Numeric(v)),
                "uv_index" => uv_index.get(row_idx).map(|&v| Cell::Numeric(v)),
                "cloud_cover_pct" => cloud_cover_pct
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v as f64)),
                "visibility_m" => visibility_m.get(row_idx).map(|&v| Cell::Numeric(v as f64)),
                "wind_speed_m_s" => wind_speed_m_s.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_direction_deg" => wind_direction_deg
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v as f64)),
                "wind_gust_speed_m_s" => wind_gust_speed_m_s
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "precipitation_probability" => precipitation_probability
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "rain_volume_1h_mm" => rain_volume_1h_mm
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "snow_volume_1h_mm" => snow_volume_1h_mm
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "weather_condition" => weather_condition
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_description" => weather_description
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_icon_code" => weather_icon_code
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for hourly_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::DailyForecast {
                latitude,
                longitude,
                forecast_date,
                sunrise_time,
                sunset_time,
                moonrise_time,
                moonset_time,
                moon_phase_fraction,
                temperature_day_temp,
                temperature_min_temp,
                temperature_max_temp,
                temperature_night_temp,
                temperature_evening_temp,
                temperature_morning_temp,
                apparent_temperature_day_temp,
                apparent_temperature_night_temp,
                apparent_temperature_evening_temp,
                apparent_temperature_morning_temp,
                pressure_hpa,
                humidity_pct,
                dew_point_temp,
                wind_speed_m_s,
                wind_direction_deg,
                wind_gust_speed_m_s,
                cloud_cover_pct,
                precipitation_probability,
                rain_volume_mm,
                snow_volume_mm,
                uv_index,
                weather_condition,
                weather_description,
                weather_icon_code,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "forecast_date" => forecast_date
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "sunrise_time" => sunrise_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "sunset_time" => sunset_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "moonrise_time" => moonrise_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "moonset_time" => moonset_time
                    .get(row_idx)
                    .map(|&v| Cell::Timestamptz(v * 1_000_000)),
                "moon_phase_fraction" => {
                    moon_phase_fraction.get(row_idx).map(|&v| Cell::Numeric(v))
                }
                "temperature_day_temp" => {
                    temperature_day_temp.get(row_idx).map(|&v| Cell::Numeric(v))
                }
                "temperature_min_temp" => {
                    temperature_min_temp.get(row_idx).map(|&v| Cell::Numeric(v))
                }
                "temperature_max_temp" => {
                    temperature_max_temp.get(row_idx).map(|&v| Cell::Numeric(v))
                }
                "temperature_night_temp" => temperature_night_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "temperature_evening_temp" => temperature_evening_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "temperature_morning_temp" => temperature_morning_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "apparent_temperature_day_temp" => apparent_temperature_day_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "apparent_temperature_night_temp" => apparent_temperature_night_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "apparent_temperature_evening_temp" => apparent_temperature_evening_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "apparent_temperature_morning_temp" => apparent_temperature_morning_temp
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "pressure_hpa" => pressure_hpa.get(row_idx).map(|&v| Cell::Numeric(v as f64)),
                "humidity_pct" => humidity_pct.get(row_idx).map(|&v| Cell::Numeric(v as f64)),
                "dew_point_temp" => dew_point_temp.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_speed_m_s" => wind_speed_m_s.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_direction_deg" => wind_direction_deg
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v as f64)),
                "wind_gust_speed_m_s" => wind_gust_speed_m_s
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "cloud_cover_pct" => cloud_cover_pct
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v as f64)),
                "precipitation_probability" => precipitation_probability
                    .get(row_idx)
                    .map(|&v| Cell::Numeric(v)),
                "rain_volume_mm" => rain_volume_mm
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "snow_volume_mm" => snow_volume_mm
                    .get(row_idx)
                    .and_then(|&v| v.map(Cell::Numeric)),
                "uv_index" => uv_index.get(row_idx).map(|&v| Cell::Numeric(v)),
                "weather_condition" => weather_condition
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_description" => weather_description
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_icon_code" => weather_icon_code
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for daily_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::WeatherAlerts {
                latitude,
                longitude,
                alerts,
            } => {
                let alert = alerts.get(row_idx).ok_or("alert index out of bounds")?;
                match tgt_col_name.as_str() {
                    "latitude" => Some(Cell::Numeric(*latitude)),
                    "longitude" => Some(Cell::Numeric(*longitude)),
                    "alert_sender_name" => Some(Cell::String(alert.alert_sender_name.clone())),
                    "alert_event_type" => Some(Cell::String(alert.alert_event_type.clone())),
                    "alert_start_time" => {
                        Some(Cell::Timestamptz(alert.alert_start_time * 1_000_000))
                    }
                    "alert_end_time" => Some(Cell::Timestamptz(alert.alert_end_time * 1_000_000)),
                    "alert_description" => Some(Cell::String(alert.alert_description.clone())),
                    "alert_tags" => {
                        // Convert Vec<String> to comma-separated string
                        Some(Cell::String(alert.alert_tags.join(",")))
                    }
                    _ => {
                        return Err(format!(
                            "unknown column '{}' for weather_alerts endpoint",
                            tgt_col_name
                        ))
                    }
                }
            }

            EndpointData::HistoricalWeather {
                latitude,
                longitude,
                observation_time,
                temperature_temp,
                apparent_temperature_temp,
                pressure_hpa,
                humidity_pct,
                dew_point_temp,
                cloud_cover_pct,
                visibility_m,
                wind_speed_m_s,
                wind_direction_deg,
                weather_condition,
                weather_description,
                weather_icon_code,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "observation_time" => Some(Cell::Timestamptz(observation_time * 1_000_000)),
                "temperature_temp" => Some(Cell::Numeric(*temperature_temp)),
                "apparent_temperature_temp" => Some(Cell::Numeric(*apparent_temperature_temp)),
                "pressure_hpa" => Some(Cell::Numeric(*pressure_hpa as f64)),
                "humidity_pct" => Some(Cell::Numeric(*humidity_pct as f64)),
                "dew_point_temp" => Some(Cell::Numeric(*dew_point_temp)),
                "cloud_cover_pct" => Some(Cell::Numeric(*cloud_cover_pct as f64)),
                "visibility_m" => Some(Cell::Numeric(*visibility_m as f64)),
                "wind_speed_m_s" => Some(Cell::Numeric(*wind_speed_m_s)),
                "wind_direction_deg" => Some(Cell::Numeric(*wind_direction_deg as f64)),
                "weather_condition" => Some(Cell::String(weather_condition.clone())),
                "weather_description" => Some(Cell::String(weather_description.clone())),
                "weather_icon_code" => Some(Cell::String(weather_icon_code.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for historical_weather endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::DailySummary {
                latitude,
                longitude,
                timezone_offset,
                summary_date,
                unit_system,
                temperature_min_temp,
                temperature_max_temp,
                temperature_morning_temp,
                temperature_afternoon_temp,
                temperature_evening_temp,
                temperature_night_temp,
                cloud_cover_afternoon_pct,
                humidity_afternoon_pct,
                pressure_afternoon_hpa,
                precipitation_total_mm,
                wind_max_speed_m_s,
                wind_max_direction_deg,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "timezone_offset" => Some(Cell::String(timezone_offset.clone())),
                "summary_date" => Some(Cell::String(summary_date.clone())),
                "unit_system" => Some(Cell::String(unit_system.clone())),
                "temperature_min_temp" => Some(Cell::Numeric(*temperature_min_temp)),
                "temperature_max_temp" => Some(Cell::Numeric(*temperature_max_temp)),
                "temperature_morning_temp" => Some(Cell::Numeric(*temperature_morning_temp)),
                "temperature_afternoon_temp" => Some(Cell::Numeric(*temperature_afternoon_temp)),
                "temperature_evening_temp" => Some(Cell::Numeric(*temperature_evening_temp)),
                "temperature_night_temp" => Some(Cell::Numeric(*temperature_night_temp)),
                "cloud_cover_afternoon_pct" => Some(Cell::Numeric(*cloud_cover_afternoon_pct)),
                "humidity_afternoon_pct" => Some(Cell::Numeric(*humidity_afternoon_pct)),
                "pressure_afternoon_hpa" => Some(Cell::Numeric(*pressure_afternoon_hpa)),
                "precipitation_total_mm" => Some(Cell::Numeric(*precipitation_total_mm)),
                "wind_max_speed_m_s" => Some(Cell::Numeric(*wind_max_speed_m_s)),
                "wind_max_direction_deg" => Some(Cell::Numeric(*wind_max_direction_deg)),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for daily_summary endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::WeatherOverview {
                latitude,
                longitude,
                timezone_offset,
                overview_date,
                unit_system,
                weather_overview,
            } => match tgt_col_name.as_str() {
                "latitude" => Some(Cell::Numeric(*latitude)),
                "longitude" => Some(Cell::Numeric(*longitude)),
                "timezone_offset" => Some(Cell::String(timezone_offset.clone())),
                "overview_date" => Some(Cell::String(overview_date.clone())),
                "unit_system" => Some(Cell::String(unit_system.clone())),
                "weather_overview" => Some(Cell::String(weather_overview.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for weather_overview endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::None => {
                return Err("no data loaded - fetch_source_data not called".to_owned());
            }
        };

        Ok(cell)
    }

    /// Fetch data from OpenWeather API based on endpoint type
    fn fetch_source_data(&mut self) -> FdwResult {
        let endpoint_type = self
            .endpoint_type
            .ok_or("endpoint type not set - call begin_scan first")?;

        // Log request details
        utils::report_info(&format!(
            "Fetching OpenWeather data for {:?} at latitude={}, longitude={}",
            endpoint_type, self.latitude, self.longitude
        ));

        // Create and execute HTTP request
        let req = self.create_request()?;
        let resp = http::get(&req)?;

        // Check for HTTP errors
        http::error_for_status(&resp).map_err(|err| format!("{}: {}", err, resp.body))?;

        utils::report_info(&format!(
            "API Response: {} bytes, status {}",
            resp.body.len(),
            resp.status_code
        ));

        // Parse JSON response
        let resp_json: JsonValue =
            serde_json::from_str(&resp.body).map_err(|e| format!("JSON parse error: {}", e))?;

        // Parse response based on endpoint type
        match endpoint_type {
            EndpointType::CurrentWeather => self.parse_current_weather(&resp_json)?,
            EndpointType::MinutelyForecast => self.parse_minutely_forecast(&resp_json)?,
            EndpointType::HourlyForecast => self.parse_hourly_forecast(&resp_json)?,
            EndpointType::DailyForecast => self.parse_daily_forecast(&resp_json)?,
            EndpointType::WeatherAlerts => self.parse_weather_alerts(&resp_json)?,
            EndpointType::HistoricalWeather => self.parse_historical_weather(&resp_json)?,
            EndpointType::DailySummary => self.parse_daily_summary(&resp_json)?,
            EndpointType::WeatherOverview => self.parse_weather_overview(&resp_json)?,
        }

        // Track stats
        let row_count = self.data.row_count();
        stats::inc_stats(FDW_NAME, stats::Metric::BytesIn, resp.body.len() as i64);
        stats::inc_stats(FDW_NAME, stats::Metric::RowsIn, row_count as i64);

        utils::report_info(&format!("Parsed {} rows", row_count));

        // Reset row iterator
        self.current_row = 0;

        Ok(())
    }
}

struct OpenWeatherFdwImpl;

impl Guest for OpenWeatherFdwImpl {
    fn host_version_requirement() -> String {
        // Supabase Wrappers version requirement
        // Compatible with both local (0.1.5) and production (0.2.x+)
        // This must match WIT declarations in wit/world.wit
        "^0.1.0".to_string()
    }

    fn init(ctx: &Context) -> FdwResult {
        OpenWeatherFdw::init();

        // Extract server options
        let opts = ctx.get_options(&OptionsType::Server);
        let instance = OpenWeatherFdw::this_mut();

        // Get base URL (default to OpenWeather API v3.0)
        instance.base_url = match opts.get("api_url") {
            Some(url) => url.clone(),
            None => "https://api.openweathermap.org/data/3.0".to_string(),
        };

        // Get API key (required) - framework handles api_key_id vault resolution automatically
        instance.api_key = opts
            .get("api_key")
            .ok_or("api_key is required in server options")?
            .clone();

        // Set up HTTP headers
        instance.headers.push((
            "user-agent".to_owned(),
            "Supabase Wrappers OpenWeather FDW".to_string(),
        ));
        instance
            .headers
            .push(("accept".to_owned(), "application/json".to_string()));

        utils::report_info(&format!(
            "OpenWeather FDW initialized with URL: {}",
            instance.base_url
        ));
        stats::inc_stats(FDW_NAME, stats::Metric::CreateTimes, 1);
        Ok(())
    }

    fn begin_scan(ctx: &Context) -> FdwResult {
        let instance = OpenWeatherFdw::this_mut();

        // Get table options
        let opts = ctx.get_options(&OptionsType::Table);

        // Parse endpoint type from 'object' option
        let object_name = opts.get("object").ok_or("'object' option is required")?;

        let endpoint_type = EndpointType::from_object_name(&object_name)?;
        instance.endpoint_type = Some(endpoint_type);

        // Extract WHERE clause parameters
        let quals = ctx.get_quals();

        // Extract and validate location (required for all endpoints)
        let (latitude, longitude) = OpenWeatherFdw::extract_and_validate_location(&quals)?;
        instance.latitude = latitude;
        instance.longitude = longitude;

        // Extract optional parameters with defaults
        instance.units = OpenWeatherFdw::extract_qual_string(&quals, "units")
            .unwrap_or_else(|| "metric".to_string());
        instance.lang =
            OpenWeatherFdw::extract_qual_string(&quals, "lang").unwrap_or_else(|| "en".to_string());

        // Extract endpoint-specific parameters
        match endpoint_type {
            EndpointType::HistoricalWeather => {
                // Extract observation_time and convert to Unix seconds for API
                let observation_time = OpenWeatherFdw::extract_qual_timestamptz(&quals, "observation_time")
                    .ok_or("WHERE clause must include 'observation_time' for historical_weather. Example: WHERE latitude = 52.52 AND longitude = 13.405 AND observation_time = '2024-01-01 00:00:00+00'")?;
                instance.dt = Some(observation_time / 1_000_000); // Convert microseconds → seconds for API
            }
            EndpointType::DailySummary => {
                // Extract required summary_date parameter (YYYY-MM-DD)
                instance.date = Some(OpenWeatherFdw::extract_qual_string(&quals, "summary_date")
                    .ok_or("WHERE clause must include 'summary_date' (YYYY-MM-DD format) for daily_summary. Example: WHERE latitude = 52.52 AND longitude = 13.405 AND summary_date = '2024-01-15'")?);
                // Extract optional timezone_offset parameter (+/-HHMM)
                instance.timezone_offset =
                    OpenWeatherFdw::extract_qual_string(&quals, "timezone_offset");
            }
            EndpointType::WeatherOverview => {
                // Extract optional overview_date parameter (defaults to today if omitted)
                instance.date = OpenWeatherFdw::extract_qual_string(&quals, "overview_date");
            }
            _ => {} // No additional parameters needed for other endpoints
        }

        // Fetch data from API
        instance.fetch_source_data()
    }

    fn iter_scan(ctx: &Context, row: &Row) -> Result<Option<u32>, FdwError> {
        let instance = OpenWeatherFdw::this_mut();

        // Check if we've exhausted all rows
        if instance.current_row >= instance.data.row_count() {
            stats::inc_stats(
                FDW_NAME,
                stats::Metric::RowsOut,
                instance.current_row as i64,
            );
            return Ok(None);
        }

        // Populate row with values from current index
        for tgt_col in ctx.get_columns() {
            let cell = instance.get_cell_value(&tgt_col)?;
            row.push(cell.as_ref());
        }

        // Move to next row
        instance.current_row += 1;
        Ok(Some(0))
    }

    fn end_scan(_ctx: &Context) -> FdwResult {
        let instance = OpenWeatherFdw::this_mut();

        // Reset instance state
        instance.endpoint_type = None;
        instance.data = EndpointData::None;
        instance.current_row = 0;

        Ok(())
    }

    fn begin_modify(_ctx: &Context) -> FdwResult {
        Err("OpenWeather FDW does not support data modification".to_string())
    }

    fn insert(_ctx: &Context, _row: &Row) -> FdwResult {
        Err("OpenWeather FDW does not support INSERT".to_string())
    }

    fn update(_ctx: &Context, _rowid: Cell, _row: &Row) -> FdwResult {
        Err("OpenWeather FDW does not support UPDATE".to_string())
    }

    fn delete(_ctx: &Context, _rowid: Cell) -> FdwResult {
        Err("OpenWeather FDW does not support DELETE".to_string())
    }

    fn end_modify(_ctx: &Context) -> FdwResult {
        Err("OpenWeather FDW does not support data modification".to_string())
    }

    fn re_scan(_ctx: &Context) -> FdwResult {
        // Re-scan not implemented (would reset scan to beginning)
        Ok(())
    }

    fn import_foreign_schema(
        _ctx: &Context,
        stmt: ImportForeignSchemaStmt,
    ) -> Result<Vec<String>, FdwError> {
        // Generate schemas for all 8 supported endpoints (v0.3.0 - standards compliant)
        let ret = vec![
            // current_weather table (1 row from /onecall → current)
            format!(
                r#"create foreign table if not exists current_weather (
                latitude numeric,
                longitude numeric,
                timezone_name text,
                observation_time timestamp with time zone,
                temperature_temp numeric,
                apparent_temperature_temp numeric,
                pressure_hpa numeric,
                humidity_pct numeric,
                dew_point_temp numeric,
                uv_index numeric,
                cloud_cover_pct numeric,
                visibility_m numeric,
                wind_speed_m_s numeric,
                wind_direction_deg numeric,
                wind_gust_speed_m_s numeric,
                weather_condition text,
                weather_description text,
                weather_icon_code text
            )
            server {} options (
                object 'current_weather'
            )"#,
                stmt.server_name,
            ),
            // minutely_forecast table (60 rows from /onecall → minutely[])
            format!(
                r#"create foreign table if not exists minutely_forecast (
                latitude numeric,
                longitude numeric,
                forecast_time timestamp with time zone,
                precipitation_mm numeric
            )
            server {} options (
                object 'minutely_forecast'
            )"#,
                stmt.server_name,
            ),
            // hourly_forecast table (48 rows from /onecall → hourly[])
            format!(
                r#"create foreign table if not exists hourly_forecast (
                latitude numeric,
                longitude numeric,
                forecast_time timestamp with time zone,
                temperature_temp numeric,
                apparent_temperature_temp numeric,
                pressure_hpa numeric,
                humidity_pct numeric,
                dew_point_temp numeric,
                uv_index numeric,
                cloud_cover_pct numeric,
                visibility_m numeric,
                wind_speed_m_s numeric,
                wind_direction_deg numeric,
                wind_gust_speed_m_s numeric,
                precipitation_probability numeric,
                rain_volume_1h_mm numeric,
                snow_volume_1h_mm numeric,
                weather_condition text,
                weather_description text,
                weather_icon_code text
            )
            server {} options (
                object 'hourly_forecast'
            )"#,
                stmt.server_name,
            ),
            // daily_forecast table (8 rows from /onecall → daily[])
            format!(
                r#"create foreign table if not exists daily_forecast (
                latitude numeric,
                longitude numeric,
                forecast_date timestamp with time zone,
                sunrise_time timestamp with time zone,
                sunset_time timestamp with time zone,
                moonrise_time timestamp with time zone,
                moonset_time timestamp with time zone,
                moon_phase_fraction numeric,
                temperature_day_temp numeric,
                temperature_min_temp numeric,
                temperature_max_temp numeric,
                temperature_night_temp numeric,
                temperature_evening_temp numeric,
                temperature_morning_temp numeric,
                apparent_temperature_day_temp numeric,
                apparent_temperature_night_temp numeric,
                apparent_temperature_evening_temp numeric,
                apparent_temperature_morning_temp numeric,
                pressure_hpa numeric,
                humidity_pct numeric,
                dew_point_temp numeric,
                wind_speed_m_s numeric,
                wind_direction_deg numeric,
                wind_gust_speed_m_s numeric,
                cloud_cover_pct numeric,
                precipitation_probability numeric,
                rain_volume_mm numeric,
                snow_volume_mm numeric,
                uv_index numeric,
                weather_condition text,
                weather_description text,
                weather_icon_code text
            )
            server {} options (
                object 'daily_forecast'
            )"#,
                stmt.server_name,
            ),
            // weather_alerts table (0-N rows from /onecall → alerts[])
            format!(
                r#"create foreign table if not exists weather_alerts (
                latitude numeric,
                longitude numeric,
                alert_sender_name text,
                alert_event_type text,
                alert_start_time timestamp with time zone,
                alert_end_time timestamp with time zone,
                alert_description text,
                alert_tags text
            )
            server {} options (
                object 'weather_alerts'
            )"#,
                stmt.server_name,
            ),
            // historical_weather table (1 row from /onecall/timemachine → data[0])
            format!(
                r#"create foreign table if not exists historical_weather (
                latitude numeric,
                longitude numeric,
                observation_time timestamp with time zone,
                temperature_temp numeric,
                apparent_temperature_temp numeric,
                pressure_hpa numeric,
                humidity_pct numeric,
                dew_point_temp numeric,
                cloud_cover_pct numeric,
                visibility_m numeric,
                wind_speed_m_s numeric,
                wind_direction_deg numeric,
                weather_condition text,
                weather_description text,
                weather_icon_code text
            )
            server {} options (
                object 'historical_weather'
            )"#,
                stmt.server_name,
            ),
            // daily_summary table (1 row from /onecall/day_summary → daily aggregations)
            format!(
                r#"create foreign table if not exists daily_summary (
                latitude numeric,
                longitude numeric,
                timezone_offset text,
                summary_date text,
                unit_system text,
                temperature_min_temp numeric,
                temperature_max_temp numeric,
                temperature_morning_temp numeric,
                temperature_afternoon_temp numeric,
                temperature_evening_temp numeric,
                temperature_night_temp numeric,
                cloud_cover_afternoon_pct numeric,
                humidity_afternoon_pct numeric,
                pressure_afternoon_hpa numeric,
                precipitation_total_mm numeric,
                wind_max_speed_m_s numeric,
                wind_max_direction_deg numeric
            )
            server {} options (
                object 'daily_summary'
            )"#,
                stmt.server_name,
            ),
            // weather_overview table (1 row from /onecall/overview → AI weather summary)
            format!(
                r#"create foreign table if not exists weather_overview (
                latitude numeric,
                longitude numeric,
                timezone_offset text,
                overview_date text,
                unit_system text,
                weather_overview text
            )
            server {} options (
                object 'weather_overview'
            )"#,
                stmt.server_name,
            ),
        ];
        Ok(ret)
    }
}

// Export the implementation
bindings::export!(OpenWeatherFdwImpl with_types_in bindings);
