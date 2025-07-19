use std::collections::{HashSet, VecDeque};

use chrono::{Date, DateTime, NaiveDateTime, Utc};
use futures::prelude::*;
use irc::{client::prelude::*, error};
use serde::{Deserialize, Deserializer, Serializer};

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    env_logger::init();

    // We can also load the Config at runtime via Config::load("path/to/config.toml")
    let config = Config {
        nickname: Some("wevebo".to_owned()),
        alt_nicks: vec![
            "wevebobo".to_owned(),
            "wevevabo".to_owned(),
            "weveboba".to_owned(),
        ],
        server: Some("irc.afternet.org".to_owned()),
        channels: vec!["#wevebo".to_owned()],
        ..Config::default()
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        println!("{:?}", message);

        if let Command::PRIVMSG(channel, text) = message.command {
            println!("{:?} {:?}", channel, text);
        }
    }

    Ok(())
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

async fn get_weather() -> Result<(), failure::Error> {
    let query = [
        ("latitude", "52.03"),
        ("longitude", "5.08"),
        (
            "current",
            "temperature_2m,relative_humidity_2m,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m",
        ),
    ];
    let client = reqwest::ClientBuilder::new()
        .connection_verbose(true)
        .build()?;
    let weather = client
        .get("https://api.open-meteo.com/v1/forecast")
        .query(&query)
        .send()
        .await?
        .json::<WeatherResponse>()
        .await?;

    println!("{weather:?}");
    println!("{}", weather_to_text(&weather));

    Ok(())
}

fn weather_to_text(weather: &WeatherResponse) -> String {
    let current = &weather.current;
    format!(
        "{} {:.1}C {}% {}-{}m/s {}",
        weather_code_to_text(current.weather_code),
        current.temperature_2m,
        current.relative_humidity_2m,
        current.wind_speed_10m,
        current.wind_gusts_10m,
        direction_to_text(current.wind_direction_10m),
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
