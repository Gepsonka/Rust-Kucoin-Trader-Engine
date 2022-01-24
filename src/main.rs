
mod trade;



use tokio::spawn;
use tokio_tungstenite::connect_async;
use tungstenite::{connect,Message};
use url::Url;
use futures_util::StreamExt;
use futures_util::SinkExt;
use futures_util::sink::Send;
use futures_util::TryFutureExt;
use crate::trade::subscribe_to_stream;


#[tokio::main]
async fn main ()  {

    let j=trade::reload_lisings().await.unwrap();
    println!("{:?} {:?}",j[0].token,j[0].listing_date);


    // let mut conn=trade::connect_to_websocket().await;
    //
    // let mut subscribes:Vec<trade::SubscribeStream>=Vec::new();
    //
    // subscribe_to_stream(&mut conn.ws_write,"ETH-USDT",1,&mut subscribes).await;




    // loop {
    //     let data=conn.ws_read.next().await.expect("Stream is empty").unwrap();
    //     println!("{:?}",data);
    // }



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
