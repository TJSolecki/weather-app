use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use core::panic;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let geocoding_api_key = std::env::var("GEOCODING_API_KEY")
        .unwrap_or_else(|_| panic!("GEOCODING_API_KEY not found in .env"));

    let file = File::open("./weather-codes.json").expect("Failed to open file");

    let reader = BufReader::new(file);

    let weather_code_to_href: HashMap<String, String> =
        serde_json::from_reader(reader).expect("Failed to parse JSON");

    let app = Router::new()
        .route("/weather", get(get_weather))
        .nest_service("", ServeDir::new("static"))
        .with_state(LocalState {
            api_key: geocoding_api_key,
            weather_code_to_href,
        });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on http://{}", "0.0.0.0:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

fn get_weather_icon(
    weather_map: &HashMap<String, String>,
    weather_code: &str,
    is_day: &u8,
) -> String {
    if *is_day == 0 && vec!["0", "1", "2", "3"].contains(&weather_code) {
        return weather_map
            .get(&format!("{}night", weather_code))
            .unwrap_or_else(|| panic!("could not unwrap night icon"))
            .clone();
    }
    weather_map
        .get(weather_code)
        .unwrap_or_else(|| panic!("could not unwrap icon"))
        .clone()
}

#[axum_macros::debug_handler]
async fn get_weather(
    Query(params): Query<WeatherParams>,
    State(local_state): State<LocalState>,
) -> Result<WeatherDisplay, StatusCode> {
    let location_data = get_location_data(&params.zipcode, &local_state.api_key)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let weather_data = fetch_weather(&location_data.lon, &location_data.lat)
        .await
        .map_err(|err| {
            println!("{:?}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let weather_display = WeatherDisplay::new(
        &weather_data,
        &location_data.display_name,
        &local_state.weather_code_to_href,
    );
    Ok(weather_display)
}

async fn fetch_weather(
    lon: &str,
    lat: &str,
) -> Result<WeatherResponse, Box<dyn std::error::Error>> {
    let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,weather_code,is_day&temperature_unit=fahrenheit&forecast_days=5&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min,weather_code&past_days=1&timezone=auto&timeformat=unixtime",
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

#[derive(Clone)]
pub struct LocalState {
    pub api_key: String,
    pub weather_code_to_href: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LocationData {
    pub display_name: String,
    pub lon: String,
    pub lat: String,
}

#[derive(Deserialize)]
pub struct WeatherParams {
    pub zipcode: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub utc_offset_seconds: i64,
    pub timezone: String,
    pub timezone_abbreviation: String,
    pub current: Current,
    pub hourly: Hourly,
    pub daily: Daily,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Current {
    pub time: i64,
    pub temperature_2m: f64,
    pub weather_code: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hourly {
    pub time: Vec<i64>,
    pub temperature_2m: Vec<f64>,
    pub weather_code: Vec<u16>,
    pub is_day: Vec<u8>, // 1 for day 0 for night
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Daily {
    pub time: Vec<i64>,
    pub temperature_2m_max: Vec<f64>,
    pub temperature_2m_min: Vec<f64>,
    pub weather_code: Vec<u16>,
}

#[derive(Template, Serialize, Deserialize, Debug)]
#[template(path = "weather.html")]
pub struct WeatherDisplay {
    pub display_name: String,
    pub current: CurrentForecast,
    pub hourly: Vec<HourlyForecast>,
    pub daily: Vec<DailyForecast>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CurrentForecast {
    pub time: String,
    pub temp_max: i32,
    pub temp_min: i32,
    pub temp: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HourlyForecast {
    pub date: String,
    pub temperature: i32,
    pub weather_code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DailyForecast {
    pub date: String,
    pub temperature_min: i32,
    pub temperature_max: i32,
    pub weather_code: String,
}

pub struct DailyForecastWithDateTime {
    pub date: DateTime<Utc>,
    pub temperature_min: f64,
    pub temperature_max: f64,
    pub weather_code: u16,
}

pub struct HourlyForecastWithDateTime {
    pub date: DateTime<Utc>,
    pub temperature: f64,
    pub weather_code: String,
    pub is_day: u8,
}

impl WeatherDisplay {
    fn new(
        weather_data: &WeatherResponse,
        display_name: &String,
        weather_code_to_href: &HashMap<String, String>,
    ) -> WeatherDisplay {
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
        let todays_forecast: &DailyForecastWithDateTime = daily_forecasts
            .iter()
            .take(1)
            .collect::<Vec<&DailyForecastWithDateTime>>()
            .get(0)
            .unwrap_or_else(|| panic!("Daily forecast not found"));

        let current_hour_tuple = (now.year(), now.month(), now.day(), now.hour());
        let hourly_forcasts: Vec<HourlyForecastWithDateTime> = weather_data
            .hourly
            .time
            .iter()
            .zip(weather_data.hourly.temperature_2m.iter())
            .zip(weather_data.hourly.weather_code.iter())
            .zip(weather_data.hourly.is_day.iter())
            .map(
                |(((time, temp), weather_code), is_day)| HourlyForecastWithDateTime {
                    date: DateTime::from_timestamp(time + utc_time_offset, 0)
                        .unwrap_or_else(|| panic!("Could not parse unixtime")),
                    temperature: *temp,
                    weather_code: weather_code.to_string(),
                    is_day: *is_day,
                },
            )
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

        WeatherDisplay {
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
                .zip(weather_data.hourly.is_day.iter())
                .map(
                    |(((time, temp), weather_code), is_day)| HourlyForecastWithDateTime {
                        date: DateTime::from_timestamp(time + utc_time_offset, 0)
                            .unwrap_or_else(|| panic!("Could not parse unixtime")),
                        temperature: *temp,
                        weather_code: weather_code.to_string(),
                        is_day: *is_day,
                    },
                )
                .filter(|hour| hour.date >= this_hour)
                .map(|hour| HourlyForecast {
                    date: hour.date.format("%-l %p").to_string(),
                    temperature: hour.temperature as i32,
                    weather_code: get_weather_icon(
                        &weather_code_to_href,
                        &hour.weather_code,
                        &hour.is_day,
                    ),
                })
                .take(25)
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
                .skip(1)
                .take(5)
                .map(|day| DailyForecast {
                    date: day.date.format("%-m/%d").to_string(),
                    temperature_min: day.temperature_min as i32,
                    temperature_max: day.temperature_max as i32,
                    weather_code: weather_code_to_href
                        .get(&day.weather_code.to_string())
                        .unwrap_or_else(|| panic!("Weather Code unkonwn"))
                        .clone(),
                })
                .collect(),
        }
    }
}
