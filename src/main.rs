mod auth;
mod trade;
mod ws;


use tokio::spawn;
use tokio_tungstenite::connect_async;
use tungstenite::{connect,Message};
use url::Url;
use futures_util::StreamExt;
use futures_util::SinkExt;
use futures_util::sink::Send;
use futures_util::TryFutureExt;




#[tokio::main]
async fn main ()  {
    let tok=auth::aquire_ws_token().await.unwrap();

    let mut url_string="".to_owned();
    url_string.push_str(tok.endpoint.as_str());
    url_string.push_str("?token=");
    url_string.push_str(tok.token.as_str());

    let uri:Url=Url::parse(url_string.as_str()).unwrap();

    let (ws_stream,_)=connect_async(uri)
        .await
        .expect("Failed to connect");


    let (mut ws_write,mut ws_read)=ws_stream.split();

    ws_write.send(Message::Text(r#"
        {
            "id": 1545910660739,
            "type": "subscribe",
            "topic": "/market/ticker:BTC-USDT",
            "response": true
        }
    "#.into())).await.unwrap();

    ws_write.send(Message::Text(r#"
        {
            "id": 1545910660739,
            "type": "subscribe",
            "topic": "/market/ticker:ETH-ECS",
            "response": true
        }
    "#.into())).await.unwrap();


    loop {
        let data=ws_read.next().await.expect("Stream is empty").unwrap();
        println!("{:?}",data);
    }



    // let (mut socket, response)=connect_async(
    //     Url::parse(url_string.as_str()).unwrap()
    // ).await.expect("Cannot connect to socket");
    //
    // socket.write_message(Message::Text(r#"{
    //         "id": 1545910660739,
    //         "type": "subscribe",
    //         "topic": "/market/ticker:BTC-USDT",
    //         "response": true
    //     }"#.into()));
    //
    //
    //
    // loop {
    //     let msg = socket.read_message().expect("Error reading message");
    //     println!("Received: {}", msg);
    // }
    //
    // socket.close();

}
