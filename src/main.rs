use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use core::panic;
// use askama::Template;
use axum::extract::{Json, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let geocoding_api_key = std::env::var("GEOCODING_API_KEY")
        .unwrap_or_else(|_| panic!("GEOCODING_API_KEY not found in .env"));

    let app = Router::new()
        .route("/weather", get(get_weather))
        .with_state(geocoding_api_key);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}", "127.0.0.1:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LocationData {
    display_name: String,
    lon: String,
    lat: String,
}

#[derive(Deserialize)]
struct WeatherParams {
    zipcode: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct WeatherResponse {
    latitude: f64,
    longitude: f64,
    utc_offset_seconds: i64,
    timezone: String,
    timezone_abbreviation: String,
    current: Current,
    hourly: Hourly,
    daily: Daily,
}

#[derive(Serialize, Deserialize, Debug)]
struct Current {
    time: i64,
    temperature_2m: f64,
    weather_code: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hourly {
    time: Vec<i64>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u16>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Daily {
    time: Vec<i64>,
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    weather_code: Vec<u16>,
}

// #[derive(Template, Serialize, Deserialize, Debug)]
// #[template(path = "weather.html")]
// struct WeatherDisplay {
//     display_name: String,
//     forecasts: Vec<Forecast>,
// }

#[derive(Serialize, Deserialize, Debug)]
struct WeatherDisplay {
    display_name: String,
    current: CurrentForecast,
    hourly: Vec<HourlyForecast>,
    daily: Vec<DailyForecast>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CurrentForecast {
    time: String,
    temp_max: i32,
    temp_min: i32,
    temp: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct HourlyForecast {
    date: String,
    temperature: i32,
    weather_code: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DailyForecast {
    date: String,
    temperature_min: i32,
    temperature_max: i32,
    weather_code: String,
}

struct DailyForecastWithDateTime {
    date: DateTime<Utc>,
    temperature_min: f64,
    temperature_max: f64,
    weather_code: u16,
}

struct HourlyForecastWithDateTime {
    date: DateTime<Utc>,
    temperature: f64,
    weather_code: String,
}

impl WeatherDisplay {
    fn new(weather_data: &WeatherResponse, display_name: &String) -> WeatherDisplay {
        let utc_time_offset = weather_data.utc_offset_seconds;
        let current_time = weather_data.current.time;

        let now = DateTime::from_timestamp(current_time + utc_time_offset, 0)
            .unwrap_or_else(|| panic!("Could not parse unixtime"));
        let now_utc = DateTime::from_timestamp(current_time, 0)
            .unwrap_or_else(|| panic!("Could not parse unixtime"));

        let this_hour = Utc
            .with_ymd_and_hms(
                now_utc.year(),
                now_utc.month(),
                now_utc.day(),
                now_utc.hour(),
                0,
                0,
            )
            .unwrap();
        let this_hour = DateTime::from_timestamp(this_hour.timestamp() + utc_time_offset, 0)
            .unwrap_or_else(|| panic!("Unwrap for this_hour failed"));

        let today = Utc
            .with_ymd_and_hms(now_utc.year(), now_utc.month(), now_utc.day() - 1, 0, 0, 0)
            .unwrap();
        let today = DateTime::from_timestamp(today.timestamp() + utc_time_offset, 0)
            .unwrap_or_else(|| panic!("unwrap for today failed"));

        let current_date_tuple = (now.year(), now.month(), now.day());
        let daily_forecasts: Vec<DailyForecastWithDateTime> = weather_data
            .daily
            .time
            .iter()
            .zip(weather_data.daily.temperature_2m_max.iter())
            .zip(weather_data.daily.temperature_2m_min.iter())
            .zip(weather_data.daily.weather_code.iter())
            .map(
                |(((time, temp_max), temp_min), weather_code)| DailyForecastWithDateTime {
                    date: DateTime::from_timestamp(time + utc_time_offset, 0)
                        .unwrap_or_else(|| panic!("Could not parse unixtime")),
                    temperature_min: *temp_min,
                    temperature_max: *temp_max,
                    weather_code: *weather_code,
                },
            )
            .collect();
        let todays_forcasts: Vec<&DailyForecastWithDateTime> = daily_forecasts
            .iter()
            .filter(|forecast| {
                (
                    forecast.date.year(),
                    forecast.date.month(),
                    forecast.date.day(),
                ) == current_date_tuple
            })
            .take(1)
            .collect();
        let todays_forecast = todays_forcasts
            .get(0)
            .unwrap_or_else(|| panic!("Daily forecast not found"));

        let current_hour_tuple = (now.year(), now.month(), now.day(), now.hour());
        let hourly_forcasts: Vec<HourlyForecastWithDateTime> = weather_data
            .hourly
            .time
            .iter()
            .zip(weather_data.hourly.temperature_2m.iter())
            .zip(weather_data.hourly.weather_code.iter())
            .map(|((time, temp), weather_code)| HourlyForecastWithDateTime {
                date: DateTime::from_timestamp(time + utc_time_offset, 0)
                    .unwrap_or_else(|| panic!("Could not parse unixtime")),
                temperature: *temp,
                weather_code: weather_code.to_string(),
            })
            .collect();

        let this_hours_forecasts: Vec<&HourlyForecastWithDateTime> = hourly_forcasts
            .iter()
            .filter(|forecast| {
                (
                    forecast.date.year(),
                    forecast.date.month(),
                    forecast.date.day(),
                    forecast.date.hour(),
                ) == current_hour_tuple
            })
            .take(1)
            .collect();
        let this_hours_forecast = this_hours_forecasts
            .get(0)
            .unwrap_or_else(|| panic!("Could not unwrap this hours forcast"));

        return WeatherDisplay {
            display_name: display_name.split(",").take(1).collect(),
            current: CurrentForecast {
                time: now.format("%-l:%M %p").to_string(),
                temp_max: todays_forecast.temperature_max as i32,
                temp_min: todays_forecast.temperature_min as i32,
                temp: this_hours_forecast.temperature as i32,
            },
            hourly: weather_data
                .hourly
                .time
                .iter()
                .zip(weather_data.hourly.temperature_2m.iter())
                .zip(weather_data.hourly.weather_code.iter())
                .map(|((time, temp), weather_code)| HourlyForecastWithDateTime {
                    date: DateTime::from_timestamp(time + utc_time_offset, 0)
                        .unwrap_or_else(|| panic!("Could not parse unixtime")),
                    temperature: *temp,
                    weather_code: weather_code.to_string(),
                })
                .filter(|hour| hour.date >= this_hour)
                .map(|hour| HourlyForecast {
                    date: hour.date.format("%-l %p").to_string(),
                    temperature: hour.temperature as i32,
                    weather_code: hour.weather_code,
                })
                .collect(),
            daily: weather_data
                .daily
                .time
                .iter()
                .zip(weather_data.daily.temperature_2m_min.iter())
                .zip(weather_data.daily.temperature_2m_max.iter())
                .zip(weather_data.daily.weather_code.iter())
                .map(
                    |(((time, temp_min), temp_max), weather_code)| DailyForecastWithDateTime {
                        date: DateTime::from_timestamp(time + utc_time_offset, 0)
                            .unwrap_or_else(|| panic!("Could not parse unixtime")),
                        temperature_max: *temp_max,
                        temperature_min: *temp_min,
                        weather_code: *weather_code,
                    },
                )
                .filter(|day| day.date >= today)
                .map(|day| DailyForecast {
                    date: day.date.format("%-m/%d").to_string(),
                    temperature_min: day.temperature_min as i32,
                    temperature_max: day.temperature_max as i32,
                    weather_code: day.weather_code.to_string(),
                })
                .collect(),
        };
    }
}

#[axum_macros::debug_handler]
async fn get_weather(
    Query(params): Query<WeatherParams>,
    State(api_key): State<String>,
) -> Result<Json<WeatherDisplay>, StatusCode> {
    let location_data = get_location_data(&params.zipcode, &api_key)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let weather_data = fetch_weather(&location_data.lon, &location_data.lat)
        .await
        .map_err(|err| {
            println!("{:?}", err);
            return StatusCode::INTERNAL_SERVER_ERROR;
        })?;
    let weather_display = WeatherDisplay::new(&weather_data, &location_data.display_name);
    Ok(Json(weather_display))
}

async fn fetch_weather(
    lon: &str,
    lat: &str,
) -> Result<WeatherResponse, Box<dyn std::error::Error>> {
    let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,weather_code&temperature_unit=fahrenheit&forecast_days=5&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min,weather_code&past_days=1&timezone=auto&timeformat=unixtime",
        lat, lon
    );

    let response = reqwest::get(&endpoint)
        .await?
        .json::<WeatherResponse>()
        .await?;
    Ok(response)
}

async fn get_location_data(
    zipcode: &str,
    api_key: &str,
) -> Result<LocationData, Box<dyn std::error::Error>> {
    let res = reqwest::get(format!(
        "https://geocode.maps.co/search?q={}&api_key={}",
        zipcode, api_key
    ))
    .await?
    .json::<Vec<LocationData>>()
    .await?;

    res.get(0)
        .cloned()
        .ok_or("No results found for given zipcode".into())
}
