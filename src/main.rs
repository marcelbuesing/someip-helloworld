use someip_helloworld::{E01HelloWorldClient, SayHelloRequest};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let mut demo_client = E01HelloWorldClient::connect().await?;
    println!("Sending SayHello Request");

    let result = demo_client
        .say_hello(&SayHelloRequest("John Doe".to_string()))
        .await;

    println!("Result: {:?}", result);
    Ok(())
}
