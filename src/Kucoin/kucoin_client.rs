use std::collections::HashMap;
use std::io::Read;
use std::ops::Sub;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
use uuid::Uuid;


#[derive(Debug,Clone)]
pub struct Kucoin{
    api_key: String,
    api_secret: String,
    passphrase: String,
    client: reqwest::Client,
    base_url: String,
    passphrase_signature:String,
    pub wallet: Option<AccountBalanceResponse>

}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct ErrorResponse{
    code:String,
    msg: String
}

// Account balance serde scheme
#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct AccountBalanceResponse{
    pub code:String,
    pub data:Vec<Asset>
}

#[derive(Clone,Debug,Deserialize,Serialize)]
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


#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct MarketOrderParams{
    #[serde(rename="clientOid")]
    pub client_oid: String,
    pub side: String,
    pub symbol: String,
    #[serde(rename="type")]
    pub typ: String,
    #[serde(rename="tradeType")]
    pub trade_type: String,
    pub funds: String,
    pub size: String
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct MarketOrderResponseSuccess{
    code: String,
    data: HashMap<String,String>
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct LimitOrderParams{
    #[serde(rename="clientOid")]
    pub client_oid: String,
    pub side: String,
    pub symbol: String,
    #[serde(rename="type")]
    pub typ: String,
    #[serde(rename="tradeType")]
    pub trade_type: String,
    pub price: f32,
    pub size: f32,
    #[serde(rename="timeInForce")]
    pub time_in_force: Option<String>,
    #[serde(rename="cancelAfter")]
    pub cancel_after: Option<u32>, // cancel the order after this amount of seconds
    #[serde(rename="postOnly")]
    post_only: Option<bool>,


}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct LimitOrderResponseSuccess{
    a:String

}



pub enum TradeType{
    Trade(OrderType),
    Balance
}

pub enum OrderType{
    Buy, // not yet implemented
    Sell, // not yet implemented

}

impl Kucoin{
    pub async fn new(config_obj: &crate::Kucoin::config::Config) -> Kucoin {
        let encoded_passphrase=Kucoin::generate_encrypted_passphrase(&config_obj.kucoin_secret, &config_obj.kucoin_passphrase);
        let mut client=Kucoin{
            api_key:config_obj.kucoin_key.to_owned(),
            api_secret:config_obj.kucoin_secret.to_string(),
            passphrase:config_obj.kucoin_passphrase.to_owned(),
            client: reqwest::Client::new(),
            base_url:config_obj.base_url.to_owned(),
            passphrase_signature: encoded_passphrase.to_owned(),
            wallet:None
        };

        client.wallet=client.fetch_account_balance().await;

        client

    }

    fn generate_encrypted_passphrase(api_secret: &String, passphrase: &String) -> String {
        /*Same purpose as the generate_endpoint_signatures fn but with the passphrase*/

        let passphrase_key=hmac::Key::new(hmac::HMAC_SHA256,api_secret.as_bytes());
        let mut signature_passphrase=hmac::sign(&passphrase_key,passphrase.as_bytes());
        let encoded_passphrase=base64::encode(signature_passphrase.to_owned());

        encoded_passphrase

    }


    fn create_headers(&self, trade_type: TradeType, request_body:&str) -> Result<reqwest::header::HeaderMap,InvalidHeaderValue> {
        /*Since every kucoin api request which requires which requires authentication needs credentials,
        and need those credentials
        must be sent as header key-value pairs, we need to create HeaderMap instance and fill it up
        with the required data, then attach this ti the request*/

        // Creating the headers for requests.
        let utc=chrono::Utc::now();
        let key=hmac::Key::new(hmac::HMAC_SHA256,self.api_secret.as_bytes());
        let str_to_string=if let TradeType::Balance=trade_type{
            let str=utc.timestamp_millis().to_string()+"GET"+"/api/v1/accounts"+request_body;
            str

        } else if let TradeType::Trade(OrderType)=trade_type{
            let str=utc.timestamp_millis().to_string()+"POST"+"/api/v1/orders"+request_body;
            str
        } else {
            // temporary
            "".to_string()
        };

        let signature=hmac::sign(&key,str_to_string.as_bytes());
        let encoded_signature=base64::encode(signature.to_owned());

        let mut heads=reqwest::header::HeaderMap::new();


        heads.insert(reqwest::header::HeaderName::from_static("kc-api-sign"),reqwest::header::HeaderValue::from_bytes(encoded_signature.as_bytes()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-timestamp"),reqwest::header::HeaderValue::from(utc.timestamp_millis()));
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key"),reqwest::header::HeaderValue::from_str(self.api_key.as_str()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-passphrase"),reqwest::header::HeaderValue::from_bytes(self.passphrase_signature.as_bytes()).unwrap());
        heads.insert(reqwest::header::HeaderName::from_static("kc-api-key-version"),reqwest::header::HeaderValue::from_str("2").unwrap());

        Ok(heads)
    }

    pub async fn fetch_account_balance(&mut self) -> Option<AccountBalanceResponse>{
        // Get the spot wallet's usdt balance (kucoin trade vallet)

        let cpy=self.clone();
        let mut headers=cpy.create_headers(TradeType::Balance,"").expect("Invalid header value!");


        let endpoint=cpy.base_url.as_str().clone();

        let resp=self.client
            .get(endpoint.to_owned()+"/api/v1/accounts")
            .headers(headers)
            .send()
            .await.unwrap()
            .text()
            .await.unwrap();


        // Parse response into json
        let resp_json:AccountBalanceResponse=serde_json::from_str(resp.as_str()).unwrap();
        Some(resp_json)

    }

    pub async fn refresh_account_balance(&mut self){
        self.wallet=self.fetch_account_balance().await;
    }

    pub async fn create_market_order(&self, token:String, side: OrderType, size: &str, funds: &str) -> Result<MarketOrderResponseSuccess,ErrorResponse>{
        if (size=="" && funds=="") || (size!="" && funds!=""){
            panic!("It is required that you use one of the two parameters, size or funds.");
        }

        let url=self.base_url.to_string()+"/api/v1/orders";
        let uid=Uuid::new_v4();

        // creating strings for request body
        let order_side=match side {
            OrderType::Sell => {
                "sell"
            },
            OrderType::Buy => {
                "buy"
            }
        };

        // creating the json body
        let market_order_json=MarketOrderParams{
            client_oid:uid.to_string(),
            side:order_side.to_string(),
            symbol:token+"-USDT",
            typ: "market".to_string(),
            trade_type: "TRADE".to_string(),
            funds: funds.to_string(),
            size: size.to_string()
        };

        let headers=self.create_headers(TradeType::Trade(OrderType::Buy), serde_json::to_string(&market_order_json).unwrap().as_str()).expect("Could not extract the headers.");

        let json_body=serde_json::to_string(&market_order_json).unwrap();

        let resp=self.client.post(url.as_str())
            .json(&market_order_json)
            .headers(headers)
            .send()
            .await
            .unwrap();

        let resp_text=resp.text().await.unwrap();
        let resp_json:Result<MarketOrderResponseSuccess,serde_json::Error>=serde_json::from_str(resp_text.as_str());

        // if the request or the operation failed on teh server we should handle it wout panicking
        let resp_json: Result<MarketOrderResponseSuccess, ErrorResponse>=match resp_json{
          Ok(succ)=>{
              Ok(succ)
          }
          Err(e) => {
              let err_resp: ErrorResponse=serde_json::from_str(resp_text.as_str()).unwrap();
              Err(err_resp)
          }
        };

        resp_json
    }

    pub async fn create_limit_order(&self, token:String, side:OrderType, price:f32, size:f32, cancel_after:Option<u32>, post_only:Option<bool>, hidden:Option<bool>, iceberg:Option<bool>, visible_size:Option<String> ) -> Result<LimitOrderResponseSuccess,ErrorResponse>{
        let url=self.base_url.to_string()+"/api/v1/orders";
        let uid=Uuid::new_v4();

        let order_side=match side {
            OrderType::Buy => "buy",
            OrderType::Sell => "sell"
        };

        let limit_order_json=LimitOrderParams{
            client_oid: uid.to_string(),
            side: order_side.to_string(),
            symbol: token+"-USDT",
            typ: "limit".to_string(),
            trade_type: "TRADE".to_string(),
            price: price,
            size: size,
            time_in_force:Some("GTT".to_string()),
            cancel_after: cancel_after,
            post_only: post_only
        };

        // finish function
    }

}