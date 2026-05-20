use hello::HelloRequest;
use hello::greeter_client::GreeterClient;
use tracing::{error, info};

pub mod hello {
    tonic::include_proto!("hello");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut client = GreeterClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(HelloRequest {
        name: "Rosemary".into(),
    });

    info!("Sending request to GreeterServer...");

    match client.say_hello(request).await {
        Ok(response) => {
            info!("RESPONSE={:?}", response.into_inner());
        }
        Err(e) => {
            error!("Error calling SayHello: {:?}", e);
        }
    }

    Ok(())
}
