use std::collections::{HashSet, VecDeque};

use futures::prelude::*;
use irc::{client::prelude::*, error};

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    // We can also load the Config at runtime via Config::load("path/to/config.toml")
    let config = Config {
        nickname: Some("hound".to_owned()),
        alt_nicks: vec![
            "hound_from_hell".to_owned(),
            "hound_from_gehenna".to_owned(),
            "hound_from_netherworld".to_owned(),
        ],
        server: Some("irc.afternet.org".to_owned()),
        channels: vec!["#shyroom".to_owned()],
        ..Config::default()
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    let mut social = Social::new(&client);

    while let Some(message) = stream.next().await.transpose()? {
        println!("{:?}", message);
        social.handle_privmsg(message)?
    }

    Ok(())
}

struct Social<'a> {
    client: &'a Client,
    previous_messages: VecDeque<(String, String)>,
}

impl<'a> Social<'a> {
    fn new(client: &'a Client) -> Self {
        Social {
            client,
            previous_messages: VecDeque::new(),
        }
    }
    fn handle_privmsg(&mut self, message: Message) -> error::Result<()> {
        if let Some(Prefix::Nickname(nickname, _, _)) = message.prefix {
            if let Command::PRIVMSG(channel, text) = message.command {
                if self.previous_messages.len() == 3 {
                    self.previous_messages.pop_front();
                }
                self.previous_messages.push_back((nickname, text.clone()));
                assert!(self.previous_messages.len() <= 3);
                if self.previous_messages.len() == 3 {
                    let nicknames: HashSet<_> = self
                        .previous_messages
                        .iter()
                        .map(|(nickname, _)| nickname)
                        .collect();
                    let texts: HashSet<_> = self
                        .previous_messages
                        .iter()
                        .map(|(_, text)| text.trim().to_lowercase())
                        .collect();
                    if nicknames.len() == 3 && texts.len() == 1 {
                        self.client.send_privmsg(channel, text)?;
                        self.previous_messages.clear();
                    }
                }
            }
        }

        Ok(())
    }
}
