
mod trade;
mod config;



use tokio::spawn;
use tokio_tungstenite::connect_async;
use tungstenite::{connect,Message};
use url::Url;
use futures_util::StreamExt;
use futures_util::SinkExt;
use futures_util::sink::Send;
use futures_util::TryFutureExt;
use crate::trade::subscribe_to_stream;
use std::path::Path;
use std::str::FromStr;
use chrono::{DateTime, TimeZone, Utc};
use base64::{encode, decode};
use sha2::digest::crypto_common::KeyIvInit;
use sha2::{Digest, Sha256};
use ring::{hmac, rand};
use ring::rand::SecureRandom;
use ring::error::Unspecified;
use ring::hmac::HMAC_SHA256;
use serde_json::json;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize,Debug)]
struct Signature{
    signature:String
}

#[tokio::main]
async fn main ()  {
    let conf=config::parse_config().await.unwrap();

    let kucoin_client=trade::Kucoin::new(conf.kucoin_key,conf.kucoin_secret,conf.kucoin_passphrase,conf.base_endpoint);
    let r=kucoin_client.account_usdt_balance().await.unwrap();
    println!("{:?}",r);
    // let utc=chrono::Utc::now();
    // // tmiestamp in millis
    // println!("timestamp in mills: {:?}",utc.timestamp_millis());
    //
    // let str_to_sign=utc.timestamp_millis().to_string()+"GET"+"/api/v1/accounts";
    //
    // let url="https://api.kucoin.com/api/v1/accounts";
    //
    // // creating hmac signature
    // let key=hmac::Key::new(hmac::HMAC_SHA256,conf.kucoin_secret.as_bytes());
    // let mut signature=hmac::sign(&key,str_to_sign.as_bytes());
    // let encoded_signature=base64::encode(signature.as_ref());
    //
    // // encrypting passphrase
    // let key2=hmac::Key::new(hmac::HMAC_SHA256,conf.kucoin_secret.as_bytes());
    // let mut signature2=hmac::sign(&key2,conf.kucoin_passphrase.as_bytes());
    // let encoded_sign2=base64::encode(signature2.as_ref());
    // //let pure_sign:Signature=serde_json::from_slice(signature.as_ref()).unwrap();
    // println!("{:?}",encoded_signature);
    //
    //
    // let mut heads=reqwest::header::HeaderMap::new();
    // heads.insert(reqwest::header::HeaderName::from_static("kc-api-sign"),reqwest::header::HeaderValue::from_bytes(encoded_signature.as_bytes()).unwrap());
    // heads.insert(reqwest::header::HeaderName::from_static("kc-api-timestamp"),reqwest::header::HeaderValue::from(utc.timestamp_millis()));
    // heads.insert(reqwest::header::HeaderName::from_static("kc-api-key"),reqwest::header::HeaderValue::from_str(conf.kucoin_key.as_str()).unwrap());
    // heads.insert(reqwest::header::HeaderName::from_static("kc-api-passphrase"),reqwest::header::HeaderValue::from_bytes(encoded_sign2.as_bytes()).unwrap());
    // heads.insert(reqwest::header::HeaderName::from_static("kc-api-key-version"),reqwest::header::HeaderValue::from_str("2").unwrap());
    //
    // let client=reqwest::Client::new();
    // let resp=client.get(url)
    //     .headers(heads)
    //     .send()
    //     .await.unwrap()
    //     .text().await.unwrap();
    //
    //
    // println!("signature:{:?}",resp);

}
