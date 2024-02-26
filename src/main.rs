use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct ApiKeys {
    geocoding_api_key: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let geocoding_api_key = std::env::var("GEOCODING_API_KEY")
        .unwrap_or_else(|_| panic!("GEOCODING_API_KEY not found in .env"));
    let api_keys = ApiKeys { geocoding_api_key };

    let app = Router::new()
        .route("/weather", get(get_weather))
        .with_state(api_keys);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}", "127.0.0.1:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LongLat {
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

#[axum_macros::debug_handler]
async fn get_weather(
    Query(params): Query<WeatherParams>,
    State(api_keys): State<ApiKeys>,
) -> Result<Json<WeatherResponse>, StatusCode> {
    let long_lat = get_long_lat(&params.zipcode, &api_keys.geocoding_api_key)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    fetch_weather(&long_lat.lon, &long_lat.lat)
        .await
        .map_err(|err| {
            println!("{:?}", err);
            return StatusCode::INTERNAL_SERVER_ERROR;
        })
}

async fn fetch_weather(
    lon: &str,
    lat: &str,
) -> Result<Json<WeatherResponse>, Box<dyn std::error::Error>> {
    let endpoint = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,weather_code",
        lat, lon
    );

    let response = reqwest::get(&endpoint)
        .await?
        .json::<WeatherResponse>()
        .await?;
    Ok(Json(response))
}

async fn get_long_lat(zipcode: &str, api_key: &str) -> Result<LongLat, Box<dyn std::error::Error>> {
    let res = reqwest::get(format!(
        "https://geocode.maps.co/search?q={}&api_key={}",
        zipcode, api_key
    ))
    .await?
    .json::<Vec<LongLat>>()
    .await?;

    res.get(0)
        .cloned()
        .ok_or("No results found for given zipcode".into())
}
