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
    base_url: String,
    passphrase_signature:String,
    trade_signature:String,
    balance_signature:String

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
    pub holds:String,

}
// till here


enum TradeType{
    Trade,
    Balance
}


impl Kucoin{
    pub fn new(api_key: String, api_secret: String, passphrase: String, base_url: String) -> Kucoin {
        let encoded_passphrase=Kucoin::generate_encrypted_passphrase(&api_secret, &passphrase);
        let (encoded_trade_sign,encoded_balance_sign)=Kucoin::generate_endpoint_signatures(&api_secret);
        Kucoin{
            api_key:api_key.to_owned(),
            api_secret:api_secret.to_string(),
            passphrase:passphrase.to_owned(),
            client: reqwest::Client::new(),
            base_url:base_url.to_owned(),
            passphrase_signature: encoded_passphrase.to_owned(),
            trade_signature: encoded_trade_sign.to_owned(),
            balance_signature: encoded_balance_sign.to_owned()
        }
    }

    fn generate_encrypted_passphrase(api_secret: &String, passphrase: &String) -> String {
        /*Same purpose as the generate_endpoint_signatures fn but with the passphrase*/

        let passphrase_key=hmac::Key::new(hmac::HMAC_SHA256,api_secret.as_bytes());
        let mut signature_passphrase=hmac::sign(&passphrase_key,passphrase.as_bytes());
        println!("{:?}", signature_passphrase.to_owned());
        let encoded_passphrase=base64::encode(signature_passphrase.to_owned());

        encoded_passphrase

    }

    fn generate_endpoint_signatures(api_secret:&String) -> (String,String) {
        /* For every request we make through the kucoin api we need to encrypt the credentials in relation
        to the url we want to sen it, to save time (we do not want to encrypt before every request)
        I preeencrypt the credentials which will be used in the future, this way we save time before every
        request. Everything will be calculated as soon as a new Kucoin instance creation (Kucoin::new) is reuested */
        /*See more at https://docs.kucoin.com/#authentication */

        // api_key used to generate both hmac signatures
        let api_key_hmac_key=hmac::Key::new(hmac::HMAC_SHA256,api_secret.as_bytes());

        // Need timestamp in millis for encryption
        let utc=chrono::Utc::now();

        // create signature for trade enpoints
        let trade_str_to_sign=utc.timestamp_millis().to_string()+"GET"+"/api/v1/orders";
        let trade_signature=hmac::sign(&api_key_hmac_key, trade_str_to_sign.as_bytes());
        let base64_trade_encoded_signature=base64::encode(trade_signature.to_owned());

        // create signature for balance endpoint
        let balance_str_to_sign=utc.timestamp_millis().to_string()+"GET"+"/api/v1/accounts";
        let balance_signature=hmac::sign(&api_key_hmac_key, balance_str_to_sign.as_bytes());
        let base64_balance_encoded_signature=base64::encode(balance_signature.to_owned());

        (base64_trade_encoded_signature,base64_balance_encoded_signature)

    }

    fn create_headers(&self, trade_type: TradeType) -> Result<reqwest::header::HeaderMap,InvalidHeaderValue> {
        /*Since every kucoin api request which requires which requires authentication needs credentials,
        and need those credentials
        must be sent as header key-value pairs, we need to create HeaderMap instance and fill it up
        with the required data, then attach this ti the request*/

        // Creating the headers for requests.
        let utc=chrono::Utc::now();

        let mut heads=reqwest::header::HeaderMap::new();

        // filling up the header fields
        if let TradeType::Trade = trade_type {
            heads.insert(reqwest::header::HeaderName::from_static("kc-api-sign"),reqwest::header::HeaderValue::from_bytes(self.trade_signature.as_bytes()).unwrap());
        } else if let TradeType::Balance = trade_type {
            heads.insert(reqwest::header::HeaderName::from_static("kc-api-sign"),reqwest::header::HeaderValue::from_bytes(self.balance_signature.as_bytes()).unwrap());
        }

        heads.insert(reqwest::header::HeaderName::from_static("kc-api-timestamp"),reqwest::header::HeaderValue::from(utc.timestamp_millis()));
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key"),reqwest::header::HeaderValue::from_str(self.api_key.as_str()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-passphrase"),reqwest::header::HeaderValue::from_bytes(self.passphrase_signature.as_bytes()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key-version"),reqwest::header::HeaderValue::from_str("2").unwrap());

        Ok(heads)
    }

    pub async fn account_usdt_balance(&self) -> Result<String,reqwest::Error>{
        // Get the spot wallet's usdt balance
        let cpy=self.clone();
        let headers=cpy.create_headers(TradeType::Balance).unwrap();
        let endpoint=cpy.base_url.as_str().clone();

        let resp=self.client
            .get(endpoint.to_owned()+"/api/v1/accounts")
            .headers(headers)
            .send()
            .await?
            .text().await?;

        //let resp_json:AccountBalanceResponse=serde_json::from_str(resp.as_str()).unwrap();
        Ok(resp)

    }

    // pub async fn create_market_order_buy() -> Result<> {
    //
    // }

}
