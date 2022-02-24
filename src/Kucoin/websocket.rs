use std::io::Read;
use std::ops::Sub;
use serde::{Deserialize,Serialize};
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
use crate::Kucoin::config;
use chrono;
use ring::{hmac, rand};
use ring::rand::SecureRandom;
use ring::error::Unspecified;
use ring::hmac::{HMAC_SHA256, sign};
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

// struct to track which trades are active
pub struct SubscribeStream {
    id: u8,
    coin_pair: String
}

// struct of splitted websocket stream
pub struct WSStream{
    pub ws_write:SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>,Message>,
    pub ws_read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>
}

impl WSStream {
    pub async fn new() -> WSStream {
        let tok = crate::Kucoin::websocket::WSStream::aquire_ws_token().await.unwrap();

        // creating endpoint url
        let mut url_string = "".to_owned();
        url_string.push_str(tok.endpoint.as_str());
        url_string.push_str("?token=");
        url_string.push_str(tok.token.as_str());

        // parsing the url
        let uri: Url = Url::parse(url_string.as_str()).unwrap();

        // open websocket
        let (ws_stream, _) = connect_async(uri)
            .await
            .expect("Failed to connect");

        // split io streams
        let (mut ws_write, mut ws_read) = ws_stream.split();
        WSStream { ws_write: ws_write, ws_read: ws_read }
    }

    async fn aquire_ws_token() -> Result<FinalCredentials,serde_json::Error>{
        /*Fetch the websocket token*/
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

    

    pub async fn read_websocket_loop(&mut self, sub_vec: &mut Vec<SubscribeStream>) {
        println!("Websocket reading loop is starting....");
        loop {
            let data = self.ws_read.next().await.expect("Stream is empty").unwrap();
            println!("{:?}", data);
        }
    }


    pub async fn subscribe_to_stream(&mut self, token_pair: &str, id: u8, sub_vec: &mut Vec<SubscribeStream>) {
        // subscribing to a trading stream
        let mut ws_request_txt=String::new();
        ws_request_txt.push_str("{\"id\":");
        ws_request_txt.push_str(id.to_string().as_str());
        ws_request_txt.push_str(", \"type\":\"subscribe\",\"topic\":\"/market/ticker:");
        ws_request_txt.push_str(token_pair.to_string().as_str());
        ws_request_txt.push_str("\", \"response\":true}");

        self.ws_write.send(Message::Text(ws_request_txt.as_str().into())).await.unwrap();
    }
}

    