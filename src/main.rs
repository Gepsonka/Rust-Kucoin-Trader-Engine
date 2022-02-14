mod Kucoin;


#[tokio::main]
async fn main ()  {
    let conf=Kucoin::config::Config::new();
    let mut kucoin_client=Kucoin::kucoin_client::Kucoin::new(&conf).await;
    kucoin_client.refresh_account_balance().await;
    println!("{:?}",kucoin_client.wallet);

    let buy_result=kucoin_client.create_limit_order("ADA".to_string(),Kucoin::kucoin_client::OrderType::Buy, 1.0, 12.0, Some(60)).await;
    match buy_result {
        Ok(ok) => {
            println!("Successful trade: {:?}", ok);
        }
        Err(ref e) => {
            println!("Error: {:?}",e);
        }
    }



    // let mut sub_vec:Vec<Kucoin::websocket::SubscribeStream>=Vec::new();
    //
    // let mut ws_stream=Kucoin::websocket::WSStream::new().await;
    // ws_stream.subscribe_to_stream("ETH-USDT",12, &mut sub_vec).await;
    // ws_stream.read_websocket_loop(&mut sub_vec).await;


}
