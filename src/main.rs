use std::collections::{HashSet, VecDeque};

use futures::prelude::*;
use irc::{client::prelude::*, error};

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
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
