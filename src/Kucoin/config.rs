use serde::{Serialize, Deserialize};
use serde_json;
use std::fs::File;
use std::io::Read;


#[derive(Debug,Clone,Deserialize,Serialize)]
pub struct Config{
    pub test: bool,
    pub kucoin_key: String,
    pub kucoin_secret: String,
    pub kucoin_passphrase: String,
    pub base_url: String
}

impl Config {
    pub fn new() -> Config {
        let mut config_file=File::open("config.json").expect("Config file cannot be found!");
        let mut config_string: String="".to_owned();
        config_file.read_to_string(&mut config_string);
        let config_json:Config=serde_json::from_str(config_string.as_str()).unwrap();
        config_json
    }
}