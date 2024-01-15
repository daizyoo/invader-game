use std::net::IpAddr;

use bevy::ecs::system::Resource;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};

// ユーザーの構造体
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
pub struct User {
    #[serde(rename = "_name")]
    pub name: String,
    #[serde(rename = "_ip")]
    pub ip: IpAddr,
    // delta_seconds()を平均した数
    // どれくらいの間隔で座標を送ればいいか
    #[serde(rename = "_delta_seconds")]
    pub delta_seconds: f32,
}

#[derive(Serialize, Deserialize, Debug, Resource, Clone)]
pub struct RoomRequest {
    pub room_id: u32,
    pub user: User,
}

impl RoomRequest {
    pub fn new(room_id: u32, user: User) -> RoomRequest {
        RoomRequest { room_id, user }
    }
}

impl User {
    pub fn new(name: &str, ip: IpAddr, delta_seconds: f32) -> User {
        User {
            name: name.to_string(),
            ip,
            delta_seconds,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ResultResponse {
    Ok { message: String, user: Option<User> },
    Err(String),
}

// 部屋の作成の申請
#[actix_web::main]
pub async fn room_create(room_request: RoomRequest) -> Result<ResultResponse, Error> {
    let res = Client::new()
        .post("http://192.168.11.6:9999/create")
        .json(&room_request)
        .send()
        .await?;
    let json = res.json::<ResultResponse>().await?;
    println!("{:?}", json);

    Ok(json)
}

// 部屋に入る
#[actix_web::main]
pub async fn room_enter(room_request: RoomRequest) -> Result<ResultResponse, Error> {
    let res = Client::new()
        .post("http://192.168.11.6:9999/enter")
        .json(&room_request)
        .send()
        .await?;
    let json = res.json::<ResultResponse>().await?;

    println!("{:?}", json);

    Ok(json)
}
