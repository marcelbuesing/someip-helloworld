use someip_helloworld::{E01HelloWorldClient, SayHelloRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut demo_client = E01HelloWorldClient::connect("1.2.3.4:30509").await?;
    println!("Sending SayHello Request");

    let result = demo_client
        .say_hello(SayHelloRequest("Hi".to_string()))
        .await;

    println!("Result: {:?}", result);
    Ok(())
}
