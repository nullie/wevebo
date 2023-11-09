use std::collections::HashMap;

use chrono::NaiveDateTime;
use irc::client::prelude::*;
use lazy_regex::{regex_captures, regex_is_match};

fn main() {
    let mut parser = LogParser::new();
    for line in std::io::stdin().lines() {
        match line {
            Ok(line) => {
                println!("{:?}", parser.parse_line(&line).unwrap());
            }
            _ => panic!("error"),
        }
    }
}

struct LogParser {
    state: Option<State>,
}

struct State {
    channel: String,
    my_nickname: String,
    attrs: HashMap<String, Attrs>,
}

impl State {
    fn new(channel: String, my_nickname: String) -> Self {
        Self {
            channel,
            my_nickname,
            attrs: HashMap::new(),
        }
    }
}

impl LogParser {
    fn new() -> Self {
        Self { state: None }
    }

    fn parse_line(&mut self, line: &str) -> Result<Option<ParsedMessage>, UnknownMessage> {
        let fields: Vec<_> = line.splitn(3, '\t').collect();
        if let &[timestamp, source, message] = &fields[..] {
            let timestamp = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").unwrap();
            let message = match source {
                "-->" => {
                    let (_, nickname, username, hostname, channel) =
                        regex_captures!(r#"^(.*?) \((.*?)@(.*?)\) has joined (.*)$"#, message)
                            .unwrap();
                    Ok(Some(self.join(nickname, username, hostname, channel)))
                }
                "<--" => {
                    if let Some((_, nickname, username, hostname, comment)) =
                        regex_captures!(r#"^(.*?) \((.*?)@(.*?)\) has quit (.*)$"#, message)
                    {
                        Ok(Some(self.quit(nickname, username, hostname, comment)))
                    } else if let Some((_, nickname, username, hostname, channel, comment)) = regex_captures!(
                        r"^(.*?) \((.*?)@(.*?)\) has left (.*?)( \(.*\))?$",
                        message
                    ) {
                        let comment = if comment == "" {
                            None
                        } else {
                            Some(regex_captures!(r" \((.*)\)", comment).unwrap().1)
                        };
                        Ok(Some(
                            self.part(nickname, username, hostname, channel, comment),
                        ))
                    } else if let Some((_, kicker, target, comment)) =
                        regex_captures!(r"^(.*?) has kicked (.*?) \((.*)\)$", message)
                    {
                        Ok(Some(self.kick(kicker, target, comment)))
                    } else {
                        Err(UnknownMessage(line.to_string()))
                    }
                }
                "--" => {
                    if regex_is_match!(r#"^Topic for .*? is .*"#, message) {
                        Ok(None)
                    } else if regex_is_match!(
                        r#"^(.*?) has changed topic for (.*?) from "(.*?)" to "(.*?)"$"#,
                        message
                    ) {
                        Ok(None)
                    } else if regex_is_match!(r#"^Topic set by "#, message) {
                        Ok(None)
                    } else if regex_is_match!(r#"^Channel "#, message) {
                        Ok(None)
                    } else if regex_is_match!(r#"^irc: "#, message) {
                        self.state = None;
                        Ok(None)
                    } else if regex_is_match!(r#"^Mode "#, message) {
                        Ok(None)
                    } else if regex_is_match!(r#"^Notice"#, message) {
                        Ok(None)
                    } else if regex_is_match!(r#"^PvNotice"#, message) {
                        Ok(None)
                    } else if let Some((_, my_new_nickname)) =
                        regex_captures!(r#"^You are now known as (.*?)$"#, message)
                    {
                        let my_nickname = self.state.as_ref().unwrap().my_nickname.clone();
                        self.state.as_mut().unwrap().my_nickname = my_new_nickname.to_string();
                        Ok(Some(self.nick(&my_nickname, my_new_nickname)))
                    } else if let Some((_, nickname, new_nickname)) =
                        regex_captures!(r#"^(.*) is now known as (.*?)$"#, message)
                    {
                        Ok(Some(self.nick(&nickname, new_nickname)))
                    } else {
                        Err(UnknownMessage(line.to_string()))
                    }
                }
                nickname => {
                    let (_, nickname) = regex_captures!(r"^@?(.*)$", nickname).unwrap();
                    Ok(Some(self.privmsg(nickname, message)))
                }
            };
            message.map(|message| message.map(|message| ParsedMessage::new(timestamp, message)))
        } else {
            panic!("invalid number of fields: {fields:?}")
        }
    }

    fn join(&mut self, nickname: &str, username: &str, hostname: &str, channel: &str) -> Message {
        if self.state.is_none() {
            self.state = Some(State::new(channel.to_string(), nickname.to_string()));
        }
        self.state.as_mut().unwrap().attrs.insert(
            String::from(nickname),
            Attrs {
                username: username.to_string(),
                hostname: hostname.to_string(),
            },
        );
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                nickname.to_string(),
                username.to_string(),
                hostname.to_string(),
            )),
            command: Command::JOIN(String::from(channel), None, None),
        }
    }

    fn quit(&mut self, nickname: &str, username: &str, hostname: &str, comment: &str) -> Message {
        let state = self.state.as_mut().unwrap();
        state.attrs.remove(nickname);
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                nickname.to_string(),
                username.to_string(),
                hostname.to_string(),
            )),
            command: Command::QUIT(Some(comment.to_string())),
        }
    }

    fn part(
        &mut self,
        nickname: &str,
        username: &str,
        hostname: &str,
        channel: &str,
        comment: Option<&str>,
    ) -> Message {
        let state = self.state.as_mut().unwrap();
        state.attrs.remove(nickname);
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                nickname.to_string(),
                username.to_string(),
                hostname.to_string(),
            )),
            command: Command::PART(channel.to_string(), comment.map(|c| c.to_string())),
        }
    }

    fn kick(&mut self, kicker: &str, target: &str, comment: &str) -> Message {
        let state = self.state.as_mut().unwrap();
        let attrs = state.attrs.get(kicker);
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                kicker.to_string(),
                attrs.map_or("".to_string(), |attrs| attrs.username.clone()),
                attrs.map_or("".to_string(), |attrs| attrs.hostname.clone()),
            )),
            command: Command::KICK(
                state.channel.clone(),
                target.to_string(),
                Some(comment.to_string()),
            ),
        }
    }

    fn nick(&mut self, nickname: &str, new_nickname: &str) -> Message {
        let state = self.state.as_mut().unwrap();
        let attrs = state.attrs.remove(nickname);
        let attrs_ref = attrs.as_ref();
        if let Some(attrs) = attrs_ref {
            state.attrs.insert(new_nickname.to_string(), attrs.clone());
        }
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                nickname.to_string(),
                attrs_ref.map_or("".to_string(), |attrs| attrs.username.clone()),
                attrs_ref.map_or("".to_string(), |attrs| attrs.username.clone()),
            )),
            command: Command::NICK(new_nickname.to_string()),
        }
    }

    fn privmsg(&mut self, nickname: &str, message: &str) -> Message {
        let state = self.state.as_mut().unwrap();
        let attrs = state.attrs.get(nickname);
        Message {
            tags: None,
            prefix: Some(Prefix::Nickname(
                nickname.to_string(),
                attrs.map_or("".to_string(), |attrs| attrs.username.clone()),
                attrs.map_or("".to_string(), |attrs| attrs.hostname.clone()),
            )),
            command: Command::PRIVMSG(state.channel.to_string(), message.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
struct Attrs {
    username: String,
    hostname: String,
}

#[derive(Debug)]
struct UnknownMessage(String);

#[derive(Debug)]
struct ParsedMessage {
    timestamp: NaiveDateTime,
    message: Message,
}

impl ParsedMessage {
    fn new(timestamp: NaiveDateTime, message: Message) -> Self {
        ParsedMessage { timestamp, message }
    }
}
