use std::{error::Error, convert::Infallible};

use bytes::Bytes;
use config::Config;
use http::{Request, Response};
use hyper::Body;
use tonic::{transport::{Server, server::{Router, Routes}, NamedService}, body::BoxBody};
use tower_layer::{Identity, Stack, Layer};
use tower_service::Service;
use tracing::info;

pub struct ConfigurableServer<'a, L = Identity> {
    tonic_server: Server<L>,
    config: &'a Config,
}

impl<'a> ConfigurableServer<'a> {
    pub fn builder(config: &'a Config) -> Self {
        Self { tonic_server: Server::builder(), config }
    }
}

impl<'a, L> ConfigurableServer<'a, L> {
    pub fn layer<NewLayer>(self, new_layer: NewLayer) -> ConfigurableServer<'a, Stack<NewLayer, L>> {
        ConfigurableServer {
            tonic_server: self.tonic_server.layer(new_layer),
            config: self.config,
        }
    }

    pub fn add_service<S>(&mut self, svc: S) -> ConfigurableRouter<'a, L>
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        L: Clone,
    {
        ConfigurableRouter {
            tonic_router: self.tonic_server.add_service(svc),
            config: self.config,
        }
    }
}

pub struct ConfigurableRouter<'a, L = Identity> {
    tonic_router: Router<L>,
    config: &'a Config,
}

impl<'a, L> ConfigurableRouter<'a, L> {
    pub async fn serve<ResBody>(self) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        L: Layer<Routes>,
        L::Service: Service<Request<Body>, Response = Response<ResBody>> + Clone + Send + 'static,
        <<L as Layer<Routes>>::Service as Service<Request<Body>>>::Future: Send + 'static,
        <<L as Layer<Routes>>::Service as Service<Request<Body>>>::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
        ResBody: http_body::Body<Data = Bytes> + Send + 'static,
        ResBody::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let address = self.config.get_string("http.address")?.parse()?;
        info!("Starting server at {}", address);
        self.tonic_router.serve(address).await?;
        Ok(())
    }
}