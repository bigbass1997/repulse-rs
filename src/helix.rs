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
    pub game_name: String,
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
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub r#type: String,
    pub broadcaster_type: String,
    pub description: String,
    pub profile_image_url: String,
    pub offline_image_url: String,
    pub view_count: u32,
    pub created_at: String,
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

pub fn search_category(twitch: &mut Twitch, category_name: &str) -> Category {
    check_auth(twitch);
    
    let mut params = HashMap::new();
    params.insert("query", category_name);
    params.insert("first", "1");
    
    let request = get_template(twitch, &url("search/categories"))
        .query(&params).build().unwrap();
    let res = twitch.req.execute(request).unwrap();
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Category>,
    }
    let json: Response = res.json().unwrap();
    
    json.data.get(0).unwrap().clone()
}

pub fn search_channel(twitch: &mut Twitch, channel_name: &str) -> Channel {
    check_auth(twitch);
    
    let mut params = HashMap::new();
    params.insert("query", channel_name);
    params.insert("first", "1");
    
    let request = get_template(twitch, &url("search/channels"))
        .query(&params).build().unwrap();
    let res = twitch.req.execute(request).unwrap();
    
    #[derive(Deserialize, Default)]
    struct Response {
        data: Vec<Channel>,
    }
    let json: Response = res.json().unwrap_or(Response::default());
    
    json.data.get(0).unwrap().clone()
}

pub fn get_streams(twitch: &mut Twitch, channel_names: Vec<&str>) -> Vec<Stream> {
    check_auth(twitch);
    
    let mut params: Vec<(&str, &str)> = Vec::new();
    params.push(("first", "100"));
    channel_names.iter().for_each(|s| {
        params.push(("user_login", s));
    });
    let request = get_template(twitch, &url("streams"))
        .query(&params).build().unwrap();
    let res = twitch.req.execute(request).unwrap();
    
    #[derive(Deserialize, Default)]
    struct Response {
        data: Vec<Stream>,
        //pagination will never be necessary because Twitch limits query to 100 names
    }
    let json: Response = res.json().unwrap_or(Response::default());
    
    json.data
}

pub fn get_streams_by_games(twitch: &mut Twitch, game_ids: Vec<&str>) -> Vec<Stream> {
    check_auth(twitch);
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Stream>,
        pagination: Pagination,
    }
    
    fn recursive(twitch: &mut Twitch, game_ids: &Vec<&str>, cursor: Option<String>) -> Vec<Stream> {
        let mut params: Vec<(&str, &str)> = Vec::new();
        params.push(("first", "100"));
        game_ids.iter().for_each(|s| {
            params.push(("game_id", s));
        });
        
        if cursor.is_some() {
            params.push(("after", cursor.as_ref().unwrap()))
        }
        
        let request = get_template(twitch, &url("streams")).query(&params).build().unwrap();
        let res = twitch.req.execute(request).unwrap();
        
        let json: Response = res.json().unwrap_or(Response {
            data: vec![],
            pagination: Pagination { cursor: None }
        });
        
        let mut data = json.data;
        if json.pagination.cursor.is_some() {
            let mut additional = recursive(twitch, &game_ids, json.pagination.cursor);
            data.append(&mut additional);
        }
        
        data
    }
    
    recursive(twitch, &game_ids, Option::None)
}

pub fn get_users_by_ids(twitch: &mut Twitch, user_ids: Vec<&str>) -> Vec<User> {
    check_auth(twitch);
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<User>,
    }
    
    let mut params: Vec<(&str, &str)> = Vec::new();
    user_ids.iter().for_each(|s| {
        params.push(("id", s));
    });
    
    let request = get_template(twitch, &url("users")).query(&params).build().unwrap();
    let res = twitch.req.execute(request).unwrap();
    
    let json: Response = res.json().unwrap_or(Response {
        data: vec![]
    });
    
    json.data
}

pub fn get_users_by_names(twitch: &mut Twitch, usernames: Vec<&str>) -> Vec<User> {
    check_auth(twitch);
    
    #[derive(Deserialize)]
    struct Response {
        data: Vec<User>,
    }
    
    let mut params: Vec<(&str, &str)> = Vec::new();
    
    usernames.iter().for_each(|s| {
        params.push(("login", s));
    });
    
    let request = get_template(twitch, &url("users")).query(&params).build().unwrap();
    let res = twitch.req.execute(request).unwrap();
    
    let json: Response = res.json().unwrap_or(Response {
        data: vec![]
    });
    
    json.data
}

fn get_template(twitch: &Twitch, url: &str) -> RequestBuilder {
    let twitch = twitch;
    
    twitch.req.get(url).header("Client-Id", &twitch.auth.client_id).header("Authorization", ["Bearer ", &twitch.auth.access_token].concat())
}

fn url(s: &str) -> String {
    [crate::constants::HELIX_API, s].concat()
}

fn check_auth(twitch: &mut Twitch) {
    if twitch.auth.has_expired() {
        twitch.refresh_auth();
    }
}