use json::{Error, JsonError, JsonValue};
use reqwest::Response;
use reqwest::Error as err;
use std::fs::File;


// pub fn creat_client(api_key:&str, api_secret:&str, passphrase:&str) -> Result<Kucoin,failure::Error> {
//     let cred=Credentials::new(api_key,api_secret,passphrase);
//     let client=Kucoin::new(KucoinEnv::Live,Some(cred));
//     client
// }

pub fn read_cred(json_string:&str) -> Result<JsonValue,JsonError> {
    let parsed=json::parse(json_string);
    parsed
}


