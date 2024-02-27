use askama::Template;
use axum::extract::{Query, State};
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
    timezone: String,
    hourly: Hourly,
}

#[derive(Serialize, Deserialize, Debug)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    weather_code: Vec<u16>,
}

#[derive(Template, Serialize, Deserialize, Debug)]
#[template(path = "weather.html")]
struct WeatherDisplay {
    display_name: String,
    forecasts: Vec<Forecast>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Forecast {
    date: String,
    temperature: f64,
    weather_code: String,
}

fn celsius_to_fahrenheit(celsius: &f64) -> f64 {
    // Convert Celsius to Fahrenheit using the formula: (C × 9/5) + 32
    (*celsius * 9.0 / 5.0) + 32.0
}
impl WeatherDisplay {
    fn new(weather_data: &WeatherResponse, display_name: &String) -> WeatherDisplay {
        WeatherDisplay {
            display_name: display_name.to_string(),
            forecasts: weather_data
                .hourly
                .time
                .iter()
                .zip(weather_data.hourly.temperature_2m.iter())
                .zip(weather_data.hourly.weather_code.iter())
                .map(|((date, temp_c), weather_code)| Forecast {
                    date: date.to_string(),
                    temperature: celsius_to_fahrenheit(temp_c),
                    weather_code: weather_code.to_string(),
                })
                .collect(),
        }
    }
}

#[axum_macros::debug_handler]
async fn get_weather(
    Query(params): Query<WeatherParams>,
    State(api_key): State<String>,
) -> Result<WeatherDisplay, StatusCode> {
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
    Ok(weather_display)
}

async fn fetch_weather(
    lon: &str,
    lat: &str,
) -> Result<WeatherResponse, Box<dyn std::error::Error>> {
    let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,weather_code",
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
