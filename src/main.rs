use std::borrow::Cow;

use chrono::{DateTime, NaiveDateTime, Utc};
use futures::prelude::*;
use irc::client::prelude::*;
use serde::{Deserialize, Deserializer};

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    env_logger::init();

    // We can also load the Config at runtime via Config::load("path/to/config.toml")
    let config = Config {
        nickname: Some("weve".to_owned()),
        alt_nicks: vec![
            "wevebo".to_owned(),
            "weveva".to_owned(),
            "weveth".to_owned(),
        ],
        server: Some("irc.afternet.org".to_owned()),
        channels: vec!["#wevebo".to_owned(), "#shyroom".to_owned()],
        ..Config::default()
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        println!("{:?}", message);

        if let Command::PRIVMSG(channel, text) = message.command {
            println!("{:?} {:?}", channel, text);
            let prefix = format!("{}: ", client.current_nickname());
            if let Some(location_name) = text.strip_prefix(&prefix) {
                let reply = weather_reply(location_name).await.unwrap_or_else(|e| {
                    log::error!("weather_reply error: {:?}", e);
                    "hold up your finger".into()
                });
                client.send_privmsg(channel, reply)?;
            }
        }
    }

    Ok(())
}

async fn weather_reply(location_name: &str) -> Result<Cow<str>, failure::Error> {
    Ok(if let Some(location) = get_location(location_name).await? {
        let weather = get_weather(location.location).await?;

        weather_to_text(&weather, &location).into()
    } else {
        "don't know where it is".into()
    })
}

#[derive(Deserialize, Debug)]
struct WeatherResponse {
    current: WeatherResponseCurrent,
}

#[derive(Deserialize, Debug)]
struct WeatherResponseCurrent {
    #[serde(deserialize_with = "deserialize_openweather_datetime")]
    time: DateTime<Utc>,
    interval: u32,
    temperature_2m: f32,
    relative_humidity_2m: u8,
    weather_code: u8,
    wind_speed_10m: f32,
    wind_direction_10m: u16,
    wind_gusts_10m: f32,
}
fn deserialize_openweather_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let dt =
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M").map_err(serde::de::Error::custom)?;

    Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
}

async fn get_weather(location: Location) -> Result<WeatherResponseCurrent, failure::Error> {
    let location = [
        ("latitude", location.latitude.to_string()),
        ("longitude", location.longitude.to_string()),
    ];
    let client = reqwest::ClientBuilder::new()
        .connection_verbose(true)
        .build()?;
    let weather = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&location)
        .query(&[
            (
            "current",
            "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m",
        ), ("wind_speed_unit", "ms")])
        .send()
        .await?
        .json::<WeatherResponse>()
        .await?;

    println!("{weather:?}");

    Ok(weather.current)
}

fn weather_to_text(weather: &WeatherResponseCurrent, location: &SearchResponseEntry) -> String {
    let mut place_parts: Vec<&str> = vec![&location.name];

    if let Some(country) = &location.country {
        place_parts.push(country);
    }

    format!(
        "weather at {}: {}, {:.1}C, {}%, {} {}-{}m/s (reported {}m ago)",
        place_parts.join(", "),
        weather_code_to_text(weather.weather_code),
        weather.temperature_2m,
        weather.relative_humidity_2m,
        direction_to_text(weather.wind_direction_10m),
        weather.wind_speed_10m,
        weather.wind_gusts_10m,
        (Utc::now() - weather.time).num_minutes(),
    )
}

fn weather_code_to_text(code: u8) -> &'static str {
    match code {
        0 => "clear",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 => "fog",
        48 => "depositing rime fog",
        51 => "light drizzle",
        53 => "moderate drizzle",
        55 => "dense drizzle",
        56 => "light freezing drizzle",
        57 => "dense freezing drizzle",
        61 => "slight rain",
        63 => "moderate raing",
        65 => "heavy raing",
        66 => "light freezing raing",
        67 => "heavy freezing raing",
        71 => "slight snow",
        73 => "moderate snow",
        75 => "heavy snow",
        77 => "snow grains",
        80 => "slight rain shower",
        81 => "moderate rain shower",
        82 => "violent rain shower",
        85 => "slight snow shower",
        86 => "heavy snow shower",
        95 => "Thunderstorm",
        96 => "thunderstorm with slight hail",
        99 => "thunderstorm with heavy hail",
        _ => panic!("unknown weather code: {}", code),
    }
}

fn direction_to_text(direction: u16) -> &'static str {
    match direction {
        0..=22 | 338..=359 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        360.. => panic!("direction >= 360"),
    }
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    results: Option<Vec<SearchResponseEntry>>,
}

#[derive(Deserialize, Debug)]
struct SearchResponseEntry {
    name: String,
    #[serde(flatten)]
    location: Location,
    country: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct Location {
    latitude: f32,
    longitude: f32,
}

async fn get_location(name: &str) -> Result<Option<SearchResponseEntry>, failure::Error> {
    let query = [("name", name), ("count", "1")];
    let client = reqwest::ClientBuilder::new()
        .connection_verbose(true)
        .build()?;
    let response = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&query)
        .send()
        .await?
        .json::<SearchResponse>()
        .await?;

    Ok(response.results.and_then(|mut results| {
        assert!(results.len() <= 1);
        results.pop()
    }))
}
