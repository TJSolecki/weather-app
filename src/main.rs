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

    // build our application with a route
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
struct LongLatStrings {
    lon: String,
    lat: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct LongLat {
    lon: f64,
    lat: f64,
}

#[derive(Deserialize)]
struct WeatherParams {
    zipcode: String,
}

#[axum_macros::debug_handler]
async fn get_weather(
    Query(params): Query<WeatherParams>,
    State(api_keys): State<ApiKeys>,
) -> Result<Json<LongLat>, StatusCode> {
    match get_long_lat(&params.zipcode, &api_keys.geocoding_api_key).await {
        Ok(long_lat) => Ok(Json(long_lat)),
        Err(err) => {
            println!("{:?}", err);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_long_lat(zipcode: &str, api_key: &str) -> Result<LongLat, Box<dyn std::error::Error>> {
    let res = reqwest::get(format!(
        "https://geocode.maps.co/search?q={}&api_key={}",
        zipcode, api_key
    ))
    .await?
    .json::<Vec<LongLatStrings>>()
    .await?;

    let parsed_res: Vec<LongLat> = res
        .iter()
        .map(|long_lat_strs| LongLat {
            lon: long_lat_strs.lon.parse().unwrap_or(0.0),
            lat: long_lat_strs.lat.parse().unwrap_or(0.0),
        })
        .collect();

    parsed_res
        .get(0)
        .cloned()
        .ok_or("No results found for given zipcode".into())
}
