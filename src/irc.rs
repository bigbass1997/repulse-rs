use std::fmt::Formatter;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender, unbounded};
use websocket::ClientBuilder;
use websocket::OwnedMessage::*;
use crate::Twitch;

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Privmsg {
    pub user: String,
    pub chan: String,
    pub text: String,
}
impl std::fmt::Display for Privmsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[#{}] {}: {}", self.chan, self.user, self.text)
    }
}

pub struct IrcClient {
    closed: Arc<AtomicBool>,
    
    pub privmsg_queue: Receiver<Privmsg>,
    pub read_queue: Receiver<String>,
    pub write_queue: Sender<String>,
}
impl IrcClient {
    pub fn new(twitch: &Twitch, channels: &Vec<&str>) -> Self {
        let socket = ClientBuilder::new("ws://irc-ws.chat.twitch.tv:80").unwrap()
            .connect_insecure()
            .unwrap();
        socket.set_nonblocking(true).unwrap();
        let (mut socket_read, mut socket_write) = socket.split().unwrap();
        
        let (privmsg_send, privmsg_recv) = unbounded();
        let (outbound_send, outbound_recv) = unbounded();
        let (inbound_send, inbound_recv) = unbounded();
        
        let client = Self {
            closed: Arc::new(AtomicBool::new(false)),
            
            privmsg_queue: privmsg_recv,
            read_queue: outbound_recv,
            write_queue: inbound_send,
        };
        
        let closed = client.closed.clone();
        let inbound_send = client.write_queue.clone();
        std::thread::spawn(move || {
            loop {
                if closed.load(Ordering::SeqCst) {
                    break;
                }
                
                match socket_read.recv_message() { // recieve message from IRC server
                    Ok(msg) => match msg {
                        Text(text) => {
                            println!("> {}", text.trim());
                            
                            if text.starts_with("PING") {
                                inbound_send.send("PONG :tmi.twitch.tv".to_owned()).unwrap()
                            } else {
                                if let Some(parsed) = parse_privmsg(&text) {
                                    privmsg_send.send(parsed).unwrap();
                                }
                                outbound_send.send(text).unwrap();
                            }
                        },
                        Binary(bytes) => {
                            println!(">>> Bytes received! Idk what to do with this. UTF_lossy: {}", String::from_utf8_lossy(&bytes).to_string());
                        },
                        Ping(ping) => {
                            let text = String::from_utf8_lossy(&ping);
                            inbound_send.send(format!("PONG {}", text)).unwrap();
                            println!(">>> PING {}", text);
                        },
                        Close(_) => {
                            println!("Websocket closed");
                            closed.swap(true, Ordering::SeqCst);
                        }
                        _ => {
                            println!("Unknown message: {:?}", msg);
                        }
                    },
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        });
        
        let closed = client.closed.clone();
        std::thread::spawn(move || {
            loop {
                if closed.load(Ordering::SeqCst) {
                    break;
                }
                
                if let Ok(msg) = inbound_recv.recv() {
                    println!("< {}", msg);
                    match socket_write.send_message(&Text(msg)) {
                        Ok(_) => (),
                        Err(err) => println!("Socket write failed! {}", err)
                    }
                } else {
                    break;
                }
            }
        });
        
        let inbound_send = client.write_queue.clone();
        inbound_send.send(format!("PASS oauth:{}", twitch.auth.access_token)).unwrap();
        inbound_send.send(format!("NICK {}", twitch.auth.username.to_lowercase())).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        
        client.join(channels);
        
        client
    }
    
    pub fn join(&self, channels: &Vec<&str>) {
        channels.iter().for_each(|chan| {
            self.write_queue.send(format!("JOIN #{}", chan.to_lowercase())).unwrap();
            std::thread::sleep(Duration::from_millis(500));
            
            self.write_queue.send("CAP REQ :twitch.tv/membership".to_owned()).unwrap();
            self.write_queue.send("CAP REQ :twitch.tv/tags".to_owned()).unwrap();
            self.write_queue.send("CAP REQ :twitch.tv/commands".to_owned()).unwrap();
        });
    }
    
    pub fn part(&self, channels: &Vec<&str>) {
        channels.iter().for_each(|chan| self.write_queue.send(format!("PART #{}", chan.to_lowercase())).unwrap());
    }
}

pub fn parse_privmsg(text: &str) -> Option<Privmsg> {
    if let Some(index) = text.find("PRIVMSG") {
        let msg = text.split_at(index + 8).1;
        if !msg.starts_with('#') { return None; }
        
        let chanmsg = msg.split_at(1).1.split_once(" :").map(|s| (s.0.to_string(), s.1.to_string()));
        
        if let Some((chan, msg)) = chanmsg {
            let text = text.split_at(index).0;
            let start = text.rfind('@');
            let end = text.rfind(".tmi.twitch.tv");
            let mut user = "".to_owned();
            if let Some(start) = start { if let Some(end) = end {
                user = text[(start + 1)..end].to_owned();
            }}
            
            return Some(Privmsg {
                user,
                chan,
                text: msg.trim().to_owned(),
            });
        }
    }
    
    None
}