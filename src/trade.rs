use std::io::Read;
use std::ops::Sub;
use serde::{Deserialize, Serialize};
use url::Url;
use tokio_tungstenite;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio::spawn;
use tungstenite::{connect,Message};
use futures_util::StreamExt;
use futures_util::SinkExt;
use futures_util::sink::Send;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::TryFutureExt;
use tokio::net::TcpStream;
use Vec;
use reqwest;
use json::parse;
use std::path::Path;
use crate::config;
use chrono;
use ring::{hmac, rand};
use ring::rand::SecureRandom;
use ring::error::Unspecified;
use ring::hmac::HMAC_SHA256;
use tungstenite::http::header::InvalidHeaderValue;


#[derive(Deserialize, Serialize,Debug)]
struct instanceServers{
    endpoint: String,
    encrypt:bool,
    protocol:String,
    pingInterval:u32,
    pingTimeout:u32
}


#[derive(Deserialize, Serialize,Debug)]
struct Token{
    token: String,
    instanceServers:Vec<instanceServers>
}


#[warn(non_snake_case)]
#[derive(Deserialize, Serialize,Debug)]
struct TokenResponse{
    code: String,
    data: Token,
}

#[derive(Debug)]
pub struct FinalCredentials {
    pub token:String,
    pub endpoint:String
}


pub async fn aquire_ws_token() -> Result<FinalCredentials,serde_json::Error>{
    let client=reqwest::Client::new();
    let resp=client.post("https://api.kucoin.com/api/v1/bullet-public")
        .send()
        .await.unwrap();

    let ip=resp
        .text()
        .await.unwrap();

    let t:TokenResponse=serde_json::from_str(ip.as_str()).unwrap();
    let cred:FinalCredentials=FinalCredentials{token: t.data.token.to_owned(), endpoint:t.data.instanceServers[0].endpoint.to_owned()};

    Ok(cred)
}

// struct of splitted websocket stream
pub struct WSStream{
    pub ws_write:SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>,Message>,
    pub ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>
}


pub async fn connect_to_websocket() -> WSStream {
    let tok=aquire_ws_token().await.unwrap();

    // creating endpoint url
    let mut url_string="".to_owned();
    url_string.push_str(tok.endpoint.as_str());
    url_string.push_str("?token=");
    url_string.push_str(tok.token.as_str());

    // parsing the url
    let uri:Url=Url::parse(url_string.as_str()).unwrap();

    // open websocket
    let (ws_stream,_)=connect_async(uri)
        .await
        .expect("Failed to connect");

    // split io streams
    let (mut ws_write,mut ws_read)=ws_stream.split();
    WSStream{ws_write:ws_write, ws_read:ws_read}
}

// struct to track which trades are active
pub struct SubscribeStream{
    id:u8,
    coin_pair:String
}

pub async fn read_websocket_loop(websocket:&mut WSStream, sub_vec:&mut Vec<SubscribeStream>) {
    loop {
        let data=websocket.ws_read.next().await.expect("Stream is empty").unwrap();
        println!("{:?}",data);
    }
}




pub async fn subscribe_to_stream(write_stream:&mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>,Message>,token_pair:&str,id:u8,sub_vec:&mut Vec<SubscribeStream>){
    // subscribing to a trading stream
    write_stream.send(Message::Text(r#"
        {
            "id": 1000000000000,
            "type": "subscribe",
            "topic": "/market/ticker:ETH-USDT",
            "response": true
        }
    "#.into())).await.unwrap();
}

// pub async fn buy_market_order() -> Result<(),()>{
//
// }

#[derive(Deserialize, Serialize,Debug)]
pub struct Listing{
    pub listing_date:String,
    pub token:String
}

#[derive(Deserialize, Serialize,Debug)]
pub struct Listings{
    pub listings:Vec<Listing>

}

pub async fn reload_lisings(config: &config::Config) -> Result<Vec<Listing>,std::io::Error>{
    // refreshing the listing file
    let path = if config.test{
        "./listings_test.json"
    } else {
        "./listings.json"
    };
    let s=tokio::fs::read_to_string(path).await.unwrap();
    let parsed_json:Vec<Listing>=serde_json::from_str(s.as_str()).unwrap();
    Ok(parsed_json)

}



#[derive(Debug,Clone)]
pub struct Kucoin{
    api_key: String,
    api_secret: String,
    passphrase: String,
    client: reqwest::Client,
    base_url: String

}

// Account balance serde scheme
#[derive(Debug,Deserialize,Serialize)]
pub struct AccountBalanceResponse{
    pub code:String,
    pub data:Vec<Asset>
}

#[derive(Debug,Deserialize,Serialize)]
pub struct Asset{
    pub id:String,
    pub currency:String,
    #[serde(rename="type")]
    pub typ:String,
    pub balance:String,
    pub available:String,
    pub holds:String
}
// till here

impl Kucoin{
    pub fn new(api_key: String, api_secret: String, passphrase: String, base_url: String) -> Kucoin {
        Kucoin{api_key, api_secret, passphrase, client: reqwest::Client::new(), base_url}
    }

    fn create_headers(&self,endpoint: &str, method: &str) -> Result<reqwest::header::HeaderMap,InvalidHeaderValue> {
        // Creating the headers for requests.
        let utc=chrono::Utc::now();
        let str_to_sign=utc.timestamp_millis().to_string()+method+endpoint;

        let key=hmac::Key::new(hmac::HMAC_SHA256,self.api_secret.as_bytes());
        let mut signature=hmac::sign(&key,str_to_sign.as_bytes());
        let encoded_signature=base64::encode(signature.as_ref());

        let key2=hmac::Key::new(hmac::HMAC_SHA256,self.api_secret.as_bytes());
        let mut signature_passphrase=hmac::sign(&key2,self.passphrase.as_bytes());
        let encoded_passphrase=base64::encode(signature_passphrase.as_ref());

        let mut heads=reqwest::header::HeaderMap::new();
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-sign"),reqwest::header::HeaderValue::from_bytes(encoded_signature.as_bytes()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-timestamp"),reqwest::header::HeaderValue::from(utc.timestamp_millis()));
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key"),reqwest::header::HeaderValue::from_str(self.api_key.as_str()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-passphrase"),reqwest::header::HeaderValue::from_bytes(encoded_passphrase.as_bytes()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key-version"),reqwest::header::HeaderValue::from_str("2").unwrap());

        Ok(heads)
    }

    pub async fn account_usdt_balance(&self) -> Result<AccountBalanceResponse,reqwest::Error>{
        // Get the spot wallet's usdt balance
        let cpy=self.clone();
        let headers=cpy.create_headers("/api/v1/accounts","GET",).unwrap();
        let endpoint=cpy.base_url.as_str().clone();

        let resp=self.client
            .get(endpoint.to_owned()+"/api/v1/accounts")
            .headers(headers)
            .send()
            .await?
            .text().await?;

        let resp_json:AccountBalanceResponse=serde_json::from_str(resp.as_str()).unwrap();
        Ok(resp_json)

    }

    pub async fn create_market_order_buy() -> Result<> {

    }

}
