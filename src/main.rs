use irc::client::prelude::*;
use futures::prelude::*;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // We can also load the Config at runtime via Config::load("path/to/config.toml")
    let config = Config {
        nickname: Some("hound".to_owned()),
        server: Some("irc.afternet.org".to_owned()),
        channels: vec!["#shyroom".to_owned()],
        ..Config::default()
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    while let Some(message) = stream.next().await.transpose()? {
        print!("{}", message);
    }

    Ok(())
}