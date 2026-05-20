use hello::greeter_server::{Greeter, GreeterServer};
use hello::{HelloRequest, HelloResponse};
use tonic::{Request, Response, Status, transport::Server};
use tracing::{info, instrument};

pub mod hello {
    tonic::include_proto!("hello");
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    #[instrument(skip(self))]
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloResponse>, Status> {
        let name = request.into_inner().name;
        info!(?name, "Received SayHello request");

        let reply = HelloResponse {
            message: format!("Hello {}!", name),
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = "[::1]:50051".parse()?;
    let greeter = MyGreeter::default();

    info!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
