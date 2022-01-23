

use serde::{Deserialize, Serialize};
use url::Url;
use tokio_tungstenite;
use tokio_tungstenite::connect_async;


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



