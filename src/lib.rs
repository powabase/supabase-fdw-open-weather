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
enum EndpointData {
    #[default]
    None,

    // /onecall → current_weather (1 row)
    CurrentWeather {
        lat: f64,
        lon: f64,
        timezone: String,
        dt: i64,
        temp: f64,
        feels_like: f64,
        pressure: i64,
        humidity: i64,
        dew_point: f64,
        uvi: f64,
        clouds: i64,
        visibility: i64,
        wind_speed: f64,
        wind_deg: i64,
        wind_gust: Option<f64>,
        weather_main: String,
        weather_description: String,
        weather_icon: String,
    },

    // /onecall → minutely (60 rows)
    MinutelyForecast {
        lat: f64,
        lon: f64,
        timestamps: Vec<i64>,
        precipitation: Vec<f64>,
    },

    // /onecall → hourly (48 rows)
    HourlyForecast {
        lat: f64,
        lon: f64,
        timestamps: Vec<i64>,
        temps: Vec<f64>,
        feels_like: Vec<f64>,
        pressure: Vec<i64>,
        humidity: Vec<i64>,
        dew_point: Vec<f64>,
        uvi: Vec<f64>,
        clouds: Vec<i64>,
        visibility: Vec<i64>,
        wind_speed: Vec<f64>,
        wind_deg: Vec<i64>,
        wind_gust: Vec<Option<f64>>,
        pop: Vec<f64>,             // Probability of precipitation
        rain_1h: Vec<Option<f64>>, // Rain volume (optional)
        snow_1h: Vec<Option<f64>>, // Snow volume (optional)
        weather_main: Vec<String>,
        weather_description: Vec<String>,
        weather_icon: Vec<String>,
    },

    // /onecall → daily (8 rows)
    DailyForecast {
        lat: f64,
        lon: f64,
        timestamps: Vec<i64>,
        sunrise: Vec<i64>,
        sunset: Vec<i64>,
        moonrise: Vec<i64>,
        moonset: Vec<i64>,
        moon_phase: Vec<f64>,
        temp_day: Vec<f64>,
        temp_min: Vec<f64>,
        temp_max: Vec<f64>,
        temp_night: Vec<f64>,
        temp_eve: Vec<f64>,
        temp_morn: Vec<f64>,
        feels_like_day: Vec<f64>,
        feels_like_night: Vec<f64>,
        feels_like_eve: Vec<f64>,
        feels_like_morn: Vec<f64>,
        pressure: Vec<i64>,
        humidity: Vec<i64>,
        dew_point: Vec<f64>,
        wind_speed: Vec<f64>,
        wind_deg: Vec<i64>,
        wind_gust: Vec<Option<f64>>,
        clouds: Vec<i64>,
        pop: Vec<f64>,
        rain: Vec<Option<f64>>,
        snow: Vec<Option<f64>>,
        uvi: Vec<f64>,
        weather_main: Vec<String>,
        weather_description: Vec<String>,
        weather_icon: Vec<String>,
    },

    // /onecall → alerts (0-N rows)
    WeatherAlerts {
        lat: f64,
        lon: f64,
        alerts: Vec<AlertRow>,
    },

    // /onecall/timemachine (1 row)
    HistoricalWeather {
        lat: f64,
        lon: f64,
        dt: i64,
        temp: f64,
        feels_like: f64,
        pressure: i64,
        humidity: i64,
        dew_point: f64,
        clouds: i64,
        visibility: i64,
        wind_speed: f64,
        wind_deg: i64,
        weather_main: String,
        weather_description: String,
        weather_icon: String,
    },

    // /onecall/day_summary (1 row)
    DailySummary {
        lat: f64,
        lon: f64,
        tz: String,
        date: String,
        units: String,
        temp_min: f64,
        temp_max: f64,
        temp_morning: f64,
        temp_afternoon: f64,
        temp_evening: f64,
        temp_night: f64,
        cloud_cover_afternoon: f64,
        humidity_afternoon: f64,
        pressure_afternoon: f64,
        precipitation_total: f64,
        wind_max_speed: f64,
        wind_max_direction: f64,
    },

    // /onecall/overview (1 row)
    WeatherOverview {
        lat: f64,
        lon: f64,
        tz: String,
        date: String,
        units: String,
        weather_overview: String,
    },
}

/// Helper struct for weather alerts
#[derive(Debug, Clone)]
struct AlertRow {
    sender_name: String,
    event: String,
    start: i64,
    end: i64,
    description: String,
    tags: Vec<String>,
}

impl EndpointData {
    /// Get the number of rows in this dataset
    fn row_count(&self) -> usize {
        match self {
            EndpointData::None => 0,
            EndpointData::CurrentWeather { .. } => 1,
            EndpointData::MinutelyForecast { timestamps, .. } => timestamps.len(),
            EndpointData::HourlyForecast { timestamps, .. } => timestamps.len(),
            EndpointData::DailyForecast { timestamps, .. } => timestamps.len(),
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
    lat: f64,
    lon: f64,
    units: String,        // "metric", "imperial", or "standard"
    lang: String,         // "en", "de", "es", etc.
    dt: Option<i64>,      // Unix timestamp (historical_weather)
    date: Option<String>, // YYYY-MM-DD date (daily_summary, weather_overview)
    tz: Option<String>,   // Timezone offset +/-HHMM (daily_summary)
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

    /// Extract and validate location from WHERE clause
    fn extract_and_validate_location(
        quals: &[bindings::supabase::wrappers::types::Qual],
    ) -> Result<(f64, f64), FdwError> {
        let lat = Self::extract_qual_numeric(quals, "lat").ok_or(
            "WHERE clause must include 'lat' (latitude) between -90 and 90. \
             Example: WHERE lat = 52.52 AND lon = 13.405",
        )?;

        let lon = Self::extract_qual_numeric(quals, "lon").ok_or(
            "WHERE clause must include 'lon' (longitude) between -180 and 180. \
             Example: WHERE lat = 52.52 AND lon = 13.405",
        )?;

        // Validate ranges
        if !(-90.0..=90.0).contains(&lat) {
            return Err(format!(
                "lat must be between -90 and 90, got {}. Example: WHERE lat = 52.52",
                lat
            ));
        }

        if !(-180.0..=180.0).contains(&lon) {
            return Err(format!(
                "lon must be between -180 and 180, got {}. Example: WHERE lon = 13.405",
                lon
            ));
        }

        Ok((lat, lon))
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
                    self.lat,
                    self.lon,
                    self.api_key,
                    self.units,
                    self.lang
                )
            }
            EndpointType::HistoricalWeather => {
                let dt = self.dt.ok_or("dt parameter required for historical_weather. Example: WHERE lat = 52.52 AND lon = 13.405 AND dt = 1609459200")?;
                format!(
                    "{}{}?lat={}&lon={}&dt={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.lat,
                    self.lon,
                    dt,
                    self.api_key,
                    self.units,
                    self.lang
                )
            }
            EndpointType::DailySummary => {
                let date = self.date.as_ref().ok_or("date parameter required for daily_summary (YYYY-MM-DD format). Example: WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-15'")?;
                let mut url = format!(
                    "{}{}?lat={}&lon={}&date={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.lat,
                    self.lon,
                    date,
                    self.api_key,
                    self.units,
                    self.lang
                );
                // Add optional tz parameter
                if let Some(ref tz) = self.tz {
                    url.push_str(&format!("&tz={}", tz));
                }
                url
            }
            EndpointType::WeatherOverview => {
                let mut url = format!(
                    "{}{}?lat={}&lon={}&appid={}&units={}&lang={}",
                    self.base_url,
                    api_path,
                    self.lat,
                    self.lon,
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
            lat: self.lat,
            lon: self.lon,
            timezone: resp_json
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("UTC")
                .to_string(),
            dt,
            temp,
            feels_like,
            pressure,
            humidity,
            dew_point,
            uvi,
            clouds,
            visibility,
            wind_speed,
            wind_deg,
            wind_gust,
            weather_main,
            weather_description,
            weather_icon,
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
            lat: self.lat,
            lon: self.lon,
            timestamps,
            precipitation,
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
            lat: self.lat,
            lon: self.lon,
            timestamps,
            temps,
            feels_like,
            pressure,
            humidity,
            dew_point,
            uvi,
            clouds,
            visibility,
            wind_speed,
            wind_deg,
            wind_gust,
            pop,
            rain_1h,
            snow_1h,
            weather_main,
            weather_description,
            weather_icon,
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
            lat: self.lat,
            lon: self.lon,
            timestamps,
            sunrise,
            sunset,
            moonrise,
            moonset,
            moon_phase,
            temp_day,
            temp_min,
            temp_max,
            temp_night,
            temp_eve,
            temp_morn,
            feels_like_day,
            feels_like_night,
            feels_like_eve,
            feels_like_morn,
            pressure,
            humidity,
            dew_point,
            wind_speed,
            wind_deg,
            wind_gust,
            clouds,
            pop,
            rain,
            snow,
            uvi,
            weather_main,
            weather_description,
            weather_icon,
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
                    lat: self.lat,
                    lon: self.lon,
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
                sender_name,
                event,
                start,
                end,
                description,
                tags,
            });
        }

        self.data = EndpointData::WeatherAlerts {
            lat: self.lat,
            lon: self.lon,
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
            lat: self.lat,
            lon: self.lon,
            dt,
            temp,
            feels_like,
            pressure,
            humidity,
            dew_point,
            clouds,
            visibility,
            wind_speed,
            wind_deg,
            weather_main,
            weather_description,
            weather_icon,
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
            lat,
            lon,
            tz,
            date,
            units,
            temp_min,
            temp_max,
            temp_morning,
            temp_afternoon,
            temp_evening,
            temp_night,
            cloud_cover_afternoon,
            humidity_afternoon,
            pressure_afternoon,
            precipitation_total,
            wind_max_speed,
            wind_max_direction,
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
            lat,
            lon,
            tz,
            date,
            units,
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
                lat,
                lon,
                timezone,
                dt,
                temp,
                feels_like,
                pressure,
                humidity,
                dew_point,
                uvi,
                clouds,
                visibility,
                wind_speed,
                wind_deg,
                wind_gust,
                weather_main,
                weather_description,
                weather_icon,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "timezone" => Some(Cell::String(timezone.clone())),
                "dt" => Some(Cell::I64(*dt)),
                "temp" => Some(Cell::Numeric(*temp)),
                "feels_like" => Some(Cell::Numeric(*feels_like)),
                "pressure" => Some(Cell::I64(*pressure)),
                "humidity" => Some(Cell::I64(*humidity)),
                "dew_point" => Some(Cell::Numeric(*dew_point)),
                "uvi" => Some(Cell::Numeric(*uvi)),
                "clouds" => Some(Cell::I64(*clouds)),
                "visibility" => Some(Cell::I64(*visibility)),
                "wind_speed" => Some(Cell::Numeric(*wind_speed)),
                "wind_deg" => Some(Cell::I64(*wind_deg)),
                "wind_gust" => wind_gust.map(Cell::Numeric),
                "weather_main" => Some(Cell::String(weather_main.clone())),
                "weather_description" => Some(Cell::String(weather_description.clone())),
                "weather_icon" => Some(Cell::String(weather_icon.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for current_weather endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::MinutelyForecast {
                lat,
                lon,
                timestamps,
                precipitation,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "dt" => timestamps.get(row_idx).map(|&v| Cell::I64(v)),
                "precipitation" => precipitation.get(row_idx).map(|&v| Cell::Numeric(v)),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for minutely_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::HourlyForecast {
                lat,
                lon,
                timestamps,
                temps,
                feels_like,
                pressure,
                humidity,
                dew_point,
                uvi,
                clouds,
                visibility,
                wind_speed,
                wind_deg,
                wind_gust,
                pop,
                rain_1h,
                snow_1h,
                weather_main,
                weather_description,
                weather_icon,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "dt" => timestamps.get(row_idx).map(|&v| Cell::I64(v)),
                "temp" => temps.get(row_idx).map(|&v| Cell::Numeric(v)),
                "feels_like" => feels_like.get(row_idx).map(|&v| Cell::Numeric(v)),
                "pressure" => pressure.get(row_idx).map(|&v| Cell::I64(v)),
                "humidity" => humidity.get(row_idx).map(|&v| Cell::I64(v)),
                "dew_point" => dew_point.get(row_idx).map(|&v| Cell::Numeric(v)),
                "uvi" => uvi.get(row_idx).map(|&v| Cell::Numeric(v)),
                "clouds" => clouds.get(row_idx).map(|&v| Cell::I64(v)),
                "visibility" => visibility.get(row_idx).map(|&v| Cell::I64(v)),
                "wind_speed" => wind_speed.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_deg" => wind_deg.get(row_idx).map(|&v| Cell::I64(v)),
                "wind_gust" => wind_gust.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "pop" => pop.get(row_idx).map(|&v| Cell::Numeric(v)),
                "rain_1h" => rain_1h.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "snow_1h" => snow_1h.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "weather_main" => weather_main.get(row_idx).map(|v| Cell::String(v.clone())),
                "weather_description" => weather_description
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_icon" => weather_icon.get(row_idx).map(|v| Cell::String(v.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for hourly_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::DailyForecast {
                lat,
                lon,
                timestamps,
                sunrise,
                sunset,
                moonrise,
                moonset,
                moon_phase,
                temp_day,
                temp_min,
                temp_max,
                temp_night,
                temp_eve,
                temp_morn,
                feels_like_day,
                feels_like_night,
                feels_like_eve,
                feels_like_morn,
                pressure,
                humidity,
                dew_point,
                wind_speed,
                wind_deg,
                wind_gust,
                clouds,
                pop,
                rain,
                snow,
                uvi,
                weather_main,
                weather_description,
                weather_icon,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "dt" => timestamps.get(row_idx).map(|&v| Cell::I64(v)),
                "sunrise" => sunrise.get(row_idx).map(|&v| Cell::I64(v)),
                "sunset" => sunset.get(row_idx).map(|&v| Cell::I64(v)),
                "moonrise" => moonrise.get(row_idx).map(|&v| Cell::I64(v)),
                "moonset" => moonset.get(row_idx).map(|&v| Cell::I64(v)),
                "moon_phase" => moon_phase.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_day" => temp_day.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_min" => temp_min.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_max" => temp_max.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_night" => temp_night.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_eve" => temp_eve.get(row_idx).map(|&v| Cell::Numeric(v)),
                "temp_morn" => temp_morn.get(row_idx).map(|&v| Cell::Numeric(v)),
                "feels_like_day" => feels_like_day.get(row_idx).map(|&v| Cell::Numeric(v)),
                "feels_like_night" => feels_like_night.get(row_idx).map(|&v| Cell::Numeric(v)),
                "feels_like_eve" => feels_like_eve.get(row_idx).map(|&v| Cell::Numeric(v)),
                "feels_like_morn" => feels_like_morn.get(row_idx).map(|&v| Cell::Numeric(v)),
                "pressure" => pressure.get(row_idx).map(|&v| Cell::I64(v)),
                "humidity" => humidity.get(row_idx).map(|&v| Cell::I64(v)),
                "dew_point" => dew_point.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_speed" => wind_speed.get(row_idx).map(|&v| Cell::Numeric(v)),
                "wind_deg" => wind_deg.get(row_idx).map(|&v| Cell::I64(v)),
                "wind_gust" => wind_gust.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "clouds" => clouds.get(row_idx).map(|&v| Cell::I64(v)),
                "pop" => pop.get(row_idx).map(|&v| Cell::Numeric(v)),
                "rain" => rain.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "snow" => snow.get(row_idx).and_then(|&v| v.map(Cell::Numeric)),
                "uvi" => uvi.get(row_idx).map(|&v| Cell::Numeric(v)),
                "weather_main" => weather_main.get(row_idx).map(|v| Cell::String(v.clone())),
                "weather_description" => weather_description
                    .get(row_idx)
                    .map(|v| Cell::String(v.clone())),
                "weather_icon" => weather_icon.get(row_idx).map(|v| Cell::String(v.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for daily_forecast endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::WeatherAlerts { lat, lon, alerts } => {
                let alert = alerts.get(row_idx).ok_or("alert index out of bounds")?;
                match tgt_col_name.as_str() {
                    "lat" => Some(Cell::Numeric(*lat)),
                    "lon" => Some(Cell::Numeric(*lon)),
                    "sender_name" => Some(Cell::String(alert.sender_name.clone())),
                    "event" => Some(Cell::String(alert.event.clone())),
                    "start" => Some(Cell::I64(alert.start)),
                    "end" => Some(Cell::I64(alert.end)),
                    "description" => Some(Cell::String(alert.description.clone())),
                    "tags" => {
                        // Convert Vec<String> to comma-separated string
                        Some(Cell::String(alert.tags.join(",")))
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
                lat,
                lon,
                dt,
                temp,
                feels_like,
                pressure,
                humidity,
                dew_point,
                clouds,
                visibility,
                wind_speed,
                wind_deg,
                weather_main,
                weather_description,
                weather_icon,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "dt" => Some(Cell::I64(*dt)),
                "temp" => Some(Cell::Numeric(*temp)),
                "feels_like" => Some(Cell::Numeric(*feels_like)),
                "pressure" => Some(Cell::I64(*pressure)),
                "humidity" => Some(Cell::I64(*humidity)),
                "dew_point" => Some(Cell::Numeric(*dew_point)),
                "clouds" => Some(Cell::I64(*clouds)),
                "visibility" => Some(Cell::I64(*visibility)),
                "wind_speed" => Some(Cell::Numeric(*wind_speed)),
                "wind_deg" => Some(Cell::I64(*wind_deg)),
                "weather_main" => Some(Cell::String(weather_main.clone())),
                "weather_description" => Some(Cell::String(weather_description.clone())),
                "weather_icon" => Some(Cell::String(weather_icon.clone())),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for historical_weather endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::DailySummary {
                lat,
                lon,
                tz,
                date,
                units,
                temp_min,
                temp_max,
                temp_morning,
                temp_afternoon,
                temp_evening,
                temp_night,
                cloud_cover_afternoon,
                humidity_afternoon,
                pressure_afternoon,
                precipitation_total,
                wind_max_speed,
                wind_max_direction,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "tz" => Some(Cell::String(tz.clone())),
                "date" => Some(Cell::String(date.clone())),
                "units" => Some(Cell::String(units.clone())),
                "temp_min" => Some(Cell::Numeric(*temp_min)),
                "temp_max" => Some(Cell::Numeric(*temp_max)),
                "temp_morning" => Some(Cell::Numeric(*temp_morning)),
                "temp_afternoon" => Some(Cell::Numeric(*temp_afternoon)),
                "temp_evening" => Some(Cell::Numeric(*temp_evening)),
                "temp_night" => Some(Cell::Numeric(*temp_night)),
                "cloud_cover_afternoon" => Some(Cell::Numeric(*cloud_cover_afternoon)),
                "humidity_afternoon" => Some(Cell::Numeric(*humidity_afternoon)),
                "pressure_afternoon" => Some(Cell::Numeric(*pressure_afternoon)),
                "precipitation_total" => Some(Cell::Numeric(*precipitation_total)),
                "wind_max_speed" => Some(Cell::Numeric(*wind_max_speed)),
                "wind_max_direction" => Some(Cell::Numeric(*wind_max_direction)),
                _ => {
                    return Err(format!(
                        "unknown column '{}' for daily_summary endpoint",
                        tgt_col_name
                    ))
                }
            },

            EndpointData::WeatherOverview {
                lat,
                lon,
                tz,
                date,
                units,
                weather_overview,
            } => match tgt_col_name.as_str() {
                "lat" => Some(Cell::Numeric(*lat)),
                "lon" => Some(Cell::Numeric(*lon)),
                "tz" => Some(Cell::String(tz.clone())),
                "date" => Some(Cell::String(date.clone())),
                "units" => Some(Cell::String(units.clone())),
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
            "Fetching OpenWeather data for {:?} at lat={}, lon={}",
            endpoint_type, self.lat, self.lon
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
        // Changed from ^0.2.0 to ^0.1.0 to match local Supabase (0.1.5)
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
        let (lat, lon) = OpenWeatherFdw::extract_and_validate_location(&quals)?;
        instance.lat = lat;
        instance.lon = lon;

        // Extract optional parameters with defaults
        instance.units = OpenWeatherFdw::extract_qual_string(&quals, "units")
            .unwrap_or_else(|| "metric".to_string());
        instance.lang =
            OpenWeatherFdw::extract_qual_string(&quals, "lang").unwrap_or_else(|| "en".to_string());

        // Extract endpoint-specific parameters
        match endpoint_type {
            EndpointType::HistoricalWeather => {
                // Try to extract dt as numeric first, then as i64 if that fails
                let dt_value = OpenWeatherFdw::extract_qual_numeric(&quals, "dt")
                    .ok_or("WHERE clause must include 'dt' (Unix timestamp) for historical_weather. Example: WHERE lat = 52.52 AND lon = 13.405 AND dt = 1609459200")?;
                instance.dt = Some(dt_value as i64);
            }
            EndpointType::DailySummary => {
                // Extract required date parameter (YYYY-MM-DD)
                instance.date = Some(OpenWeatherFdw::extract_qual_string(&quals, "date")
                    .ok_or("WHERE clause must include 'date' (YYYY-MM-DD format) for daily_summary. Example: WHERE lat = 52.52 AND lon = 13.405 AND date = '2024-01-15'")?);
                // Extract optional tz parameter (+/-HHMM)
                instance.tz = OpenWeatherFdw::extract_qual_string(&quals, "tz");
            }
            EndpointType::WeatherOverview => {
                // Extract optional date parameter (defaults to today if omitted)
                instance.date = OpenWeatherFdw::extract_qual_string(&quals, "date");
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
        // Generate schemas for all 6 supported endpoints
        let ret = vec![
            // current_weather table (1 row from /onecall → current)
            format!(
                r#"create foreign table if not exists current_weather (
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
            server {} options (
                object 'current_weather'
            )"#,
                stmt.server_name,
            ),
            // minutely_forecast table (60 rows from /onecall → minutely[])
            format!(
                r#"create foreign table if not exists minutely_forecast (
                lat numeric,
                lon numeric,
                dt bigint,
                precipitation numeric
            )
            server {} options (
                object 'minutely_forecast'
            )"#,
                stmt.server_name,
            ),
            // hourly_forecast table (48 rows from /onecall → hourly[])
            format!(
                r#"create foreign table if not exists hourly_forecast (
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
            server {} options (
                object 'hourly_forecast'
            )"#,
                stmt.server_name,
            ),
            // daily_forecast table (8 rows from /onecall → daily[])
            format!(
                r#"create foreign table if not exists daily_forecast (
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
            server {} options (
                object 'daily_forecast'
            )"#,
                stmt.server_name,
            ),
            // weather_alerts table (0-N rows from /onecall → alerts[])
            format!(
                r#"create foreign table if not exists weather_alerts (
                lat numeric,
                lon numeric,
                sender_name text,
                event text,
                start bigint,
                "end" bigint,
                description text,
                tags text
            )
            server {} options (
                object 'weather_alerts'
            )"#,
                stmt.server_name,
            ),
            // historical_weather table (1 row from /onecall/timemachine → data[0])
            format!(
                r#"create foreign table if not exists historical_weather (
                lat numeric,
                lon numeric,
                dt bigint,
                temp numeric,
                feels_like numeric,
                pressure bigint,
                humidity bigint,
                dew_point numeric,
                clouds bigint,
                visibility bigint,
                wind_speed numeric,
                wind_deg bigint,
                weather_main text,
                weather_description text,
                weather_icon text
            )
            server {} options (
                object 'historical_weather'
            )"#,
                stmt.server_name,
            ),
        ];
        Ok(ret)
    }
}

// Export the implementation
bindings::export!(OpenWeatherFdwImpl with_types_in bindings);
