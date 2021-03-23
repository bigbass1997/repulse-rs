use crate::Twitch;
use reqwest::blocking::RequestBuilder;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct Channel {
    pub broadcaster_language: String,
    pub broadcaster_login: String,
    pub display_name: String,
    pub game_id: String,
    pub id: String,
    pub is_live: bool,
    pub tag_ids: Vec<String>,
    pub thumbnail_url: String,
    pub title: String,
    pub started_at: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    pub r#type: String,
    pub title: String,
    pub viewer_count: u32,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
    pub tag_ids: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
}

#[derive(Deserialize, Debug)]
struct Pagination {
    pub cursor: Option<String>,
}

pub fn search_category(twi: &mut Twitch, category_name: &str) -> Category {
    check_auth(twi);
    
    let mut params = HashMap::new();
    params.insert("query", category_name);
    params.insert("first", "1");
    
    let request = get_template(twi, &url("search/categories"))
        .query(&params).build().unwrap();
    let res = twi.req.execute(request).unwrap();
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Category>,
    }
    let json: Response = res.json().unwrap();
    
    json.data.get(0).unwrap().clone()
}

pub fn search_channel(twi: &mut Twitch, channel_name: &str) -> Channel {
    check_auth(twi);
    
    let mut params = HashMap::new();
    params.insert("query", channel_name);
    params.insert("first", "1");
    
    let request = get_template(twi, &url("search/channels"))
        .query(&params).build().unwrap();
    let res = twi.req.execute(request).unwrap();
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Channel>,
    }
    let json: Response = res.json().unwrap();
    
    json.data.get(0).unwrap().clone()
}

pub fn get_streams(twi: &mut Twitch, channel_names: Vec<&str>) -> Vec<Stream> {
    check_auth(twi);
    
    let mut params: Vec<(&str, &str)> = Vec::new();
    params.push(("first", "100"));
    channel_names.iter().for_each(|s| {
        params.push(("user_login", s));
    });
    let request = get_template(twi, &url("streams"))
        .query(&params).build().unwrap();
    let res = twi.req.execute(request).unwrap();
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Stream>,
        //pagination will never be necessary because Twitch limits query to 100 names
    }
    let json: Response = res.json().unwrap();
    
    json.data
}

pub fn get_streams_by_games(twi: &mut Twitch, game_ids: Vec<&str>) -> Vec<Stream> {
    check_auth(twi);
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Stream>,
        pagination: Pagination,
    }
    
    fn recursive(twi: &Twitch, game_ids: &Vec<&str>, cursor: Option<String>) -> Vec<Stream> {
        let mut params: Vec<(&str, &str)> = Vec::new();
        params.push(("first", "100"));
        game_ids.iter().for_each(|s| {
            params.push(("game_id", s));
        });
        
        if cursor.is_some() {
            params.push(("after", cursor.as_ref().unwrap()))
        }
        
        let request = get_template(twi, &url("streams")).query(&params).build().unwrap();
        let res = twi.req.execute(request).unwrap();
        
        let json: Response = res.json().unwrap_or(Response {
            data: vec![],
            pagination: Pagination { cursor: None }
        });
        
        let mut data = json.data;
        if json.pagination.cursor.is_some() {
            let mut additional = recursive(&twi, &game_ids, json.pagination.cursor);
            data.append(&mut additional);
        }
        
        data
    }
    
    recursive(&twi, &game_ids, Option::None)
}

fn get_template(twi: &Twitch, url: &str) -> RequestBuilder {
    twi.req.get(url).header("Client-Id", &twi.auth.client_id).header("Authorization", ["Bearer ", &twi.auth.access_token].concat())
}

fn url(s: &str) -> String {
    [crate::constants::HELIX_API, s].concat()
}

fn check_auth(twi: &mut Twitch) {
    if twi.auth.has_expired() {
        twi.refresh_auth();
    }
}