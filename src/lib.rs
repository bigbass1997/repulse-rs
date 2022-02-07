
extern crate websocket;
extern crate rouille;

use std::process::Command;
use rouille::cgi::CgiRun;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone};

pub mod helix;
pub mod irc;
mod constants;



#[derive(Debug, Clone)]
pub struct Auth {
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl Auth {
    pub fn has_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

impl From<JsonAuth> for Auth {
    fn from(json_auth: JsonAuth) -> Self {
        Auth {
            client_id: json_auth.client_id,
            client_secret: json_auth.client_secret,
            username: json_auth.username,
            access_token: json_auth.access_token,
            refresh_token: json_auth.refresh_token,
            expires_at: Utc.timestamp(json_auth.expires_at, 0),
        }
    }
}



#[derive(Deserialize, Serialize)]
struct JsonAuth {
    client_id: String,
    client_secret: String,
    username: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

impl From<&Auth> for JsonAuth {
    fn from(auth: &Auth) -> Self {
        JsonAuth {
            client_id: auth.client_id.clone(),
            client_secret: auth.client_secret.clone(),
            username: auth.username.clone(),
            access_token: auth.access_token.clone(),
            refresh_token: auth.refresh_token.clone(),
            expires_at: auth.expires_at.timestamp(),
        }
    }
}


pub struct Twitch {
    pub auth: Auth,
    pub req: reqwest::blocking::Client,
}
impl Twitch {
    pub fn new() -> Self {
        let json = std::fs::read_to_string("twitch.json").unwrap_or(String::new());
        let parsed: JsonAuth = serde_json::from_str(&json).unwrap_or_else(|_| {
            let json_auth = JsonAuth {
                client_id: String::new(),
                client_secret: String::new(),
                username: String::new(),
                access_token: String::new(),
                refresh_token: String::new(),
                expires_at: Utc::now().timestamp(),
            };
            let default = serde_json::to_string_pretty(&json_auth).unwrap();
            std::fs::write("twitch.json", default).unwrap();
            println!("Default twitch config file created. Panics may occur until you have filled in at least client_id, client_secret, and refresh_token.");
            
            json_auth
        });
        
        let mut twi = Twitch {
            auth: Auth::from(parsed),
            req: reqwest::blocking::Client::new(),
        };
        
        if twi.auth.has_expired() && !twi.auth.refresh_token.is_empty() {
            twi.refresh_auth();
        }
        
        twi
    }
    
    pub fn init_token(&mut self) {
        // https://id.twitch.tv/oauth2/authorize?client_id=<CLIENT_ID>&redirect_uri=http://localhost:8000&response_type=code&scope=chat:edit chat:read
        let cli_id = self.auth.client_id.clone();
        std::fs::write("/tmp/twitch.php", format!("<?php echo \"Get token <a href='https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri=http://localhost:8000&response_type=code&scope=chat:read chat:edit'>here</a>.<br>Code: \"; echo $_GET['code'];", cli_id)).unwrap();
        rouille::start_server("localhost:8000", move |request| {
            let mut cmd = Command::new("php-cgi");
            cmd.arg("-n");
            cmd.env("SCRIPT_FILENAME", "/tmp/twitch.php");
            cmd.env("REDIRECT_STATUS", "1");
            
            cmd.start_cgi(&request).unwrap()
        });
    }
    
    pub fn create_token(&mut self, code: &str) {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct Response {
            access_token: String,
            refresh_token: String,
            expires_in: u64,
            scope: Vec<String>,
            token_type: String,
        }
        
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("client_id", self.auth.client_id.as_str());
        params.insert("client_secret", self.auth.client_secret.as_str());
        params.insert("code", code);
        params.insert("redirect_uri", "http://localhost:8000");
        let res: Response = self.req.post(constants::OAUTH2_TOKEN)
            .query(&params)
            .send().unwrap().json().unwrap();
        
        self.auth.access_token = res.access_token;
        self.auth.refresh_token = res.refresh_token;
        self.auth.expires_at = Utc.timestamp(Utc::now().timestamp() + res.expires_in as i64, 0);
    }
    
    pub fn refresh_auth(&mut self) {
        #[derive(Deserialize, Clone)]
        struct Response {
            access_token: String,
            refresh_token: String,
            expires_in: u64,
            scope: Vec<String>,
            token_type: String,
        }
        
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", self.auth.refresh_token.as_str());
        params.insert("client_id", self.auth.client_id.as_str());
        params.insert("client_secret", self.auth.client_secret.as_str());
        let res: Response = self.req.post(constants::OAUTH2_TOKEN)
            .query(&params)
            .send().unwrap().json().unwrap();
        
        self.auth.access_token = res.access_token;
        self.auth.refresh_token = res.refresh_token;
        self.auth.expires_at = Utc.timestamp(Utc::now().timestamp() + res.expires_in as i64, 0);
        
        let json = serde_json::to_string_pretty(&JsonAuth::from(&self.auth)).unwrap();
        std::fs::write("twitch.json", json).unwrap();
    }
}



#[derive(Deserialize, Debug)]
struct Ip {
    origin: String,
}