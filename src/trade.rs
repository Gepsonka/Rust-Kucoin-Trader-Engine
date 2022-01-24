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
use json::parse;


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

// struct to note with trades are active
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
    write_stream.send(Message::Text(r#"
        {
            "id": 1000000000000,
            "type": "subscribe",
            "topic": "/market/ticker:ETH-USDT",
            "response": true
        }
    "#.into())).await.unwrap();


}

#[derive(Deserialize, Serialize,Debug)]
pub struct Listing{
    pub listing_date:String,
    pub token:String
}

#[derive(Deserialize, Serialize,Debug)]
pub struct Listings{
    pub listings:Vec<Listing>

}

pub async fn reload_lisings() -> Result<Vec<Listing>,std::io::Error>{
    let s=tokio::fs::read_to_string("C:\\Users\\expel\\Documents\\GitHub\\Rust-Kucoin-Trader-Engine").await.unwrap();
    let parsed_json:Vec<Listing>=serde_json::from_str(s.as_str()).unwrap();
    Ok(parsed_json)

}

