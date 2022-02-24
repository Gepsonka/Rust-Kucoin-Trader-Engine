use serde_json;
use serde::{Serialize,Deserialize};
use chrono;
use chrono::{Datelike, Timelike, Utc};
use tokio::fs;
use std::string::String;

#[derive(Debug,Clone)]
pub struct Watcher{
    pub listing_json: Vec<Listing>,
    pub test: bool

}

#[derive(Debug,Clone)]
pub struct OngoingOrder{
    token:  String,


}


#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Listing{
    pub listing_date: String,
    pub token: String
}

impl Watcher {
    pub async fn new(test:bool) -> Result<Watcher,serde_json::Error> {
        let json_file:Result<String,tokio::io::Error>=if test {
            fs::read_to_string("listings_test.json");
        } else {
            fs::read_to_string("listings.json");

        };


        let json_str=match json_file {
            Ok(s) => {
                s
            }
            Err(e) => {
                if test{
                    fs::File::create("listings_test.json")
                } else {
                    fs::File::create("listings.json")
                }

                String::from("")
            }
        };

        let watcher=Watcher{
            listing_json: serde_json::from_str(json_str.as_str()).unwrap(),
            test: test
        };

        Ok(watcher)
    }

    pub async fn reload_json(&mut self) -> Result<(),serde_json::Error>{
        let filename=if self.test {
            "listings_test.json"
        } else {
            "listings.json"
        };

        let json_str=match fs::read_to_string(filename){
            Ok(s) => {
                s
            }
            Err(e) => {
                panic!("Could not refresh listings. (Could not open file)")
            }
        };

        let json_obj: Vec<Listing>=serde_json::from_str(json_str).unwrap();
        self.listing_json=json_obj;
        Ok(())
    }

    pub async fn check_listing_and_execute_order(&mut self,client:&mut crate::Kucoin::kucoin_client::Kucoin, ongoing_orders:&mut Vec<String>){
        let utc=Utc::now();
        for list in &self.listing_json{
            let naive_listing_date=chrono::NaiveDateTime::parse_from_str(list.listing_date.as_str(), "%Y-%m-%d %H:%M").unwrap();
            if utc.year()==naive_listing_date.year() && utc.month()==naive_listing_date.month()
                && utc.day()==naive_listing_date.day() && utc.minute()==naive_listing_date.minute()
                && !ongoing_orders.iter().any(|&s| s==list.token.as_str()){
                println!("Buy: {}...",list.token);
            }
        }
    }

}
