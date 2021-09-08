use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server, StatusCode};

use crate::handler::NextFn;
use crate::{Handler, Request, Response};

pub use hyper::Error;

async fn service(
    req: Request,
    handler: Arc<impl Handler>,
) -> std::result::Result<hyper::Response<Body>, Infallible> {
    let mut req = handler.run(req, &NextFn(|req| async move { req })).await;

    let res = match req.take_res() {
        Some(res) => res,
        None => Response::new().with_status(StatusCode::NOT_FOUND),
    };

    Ok(res.into_inner())
}

pub async fn run(addr: impl Into<SocketAddr>, handler: impl Handler) -> Result<(), Error> {
    let addr = addr.into();
    let handler = Arc::new(handler);

    let make_svc = make_service_fn(|_| {
        let handler = handler.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                service(Request::new(req), handler.clone())
            }))
        }
    });

    log::info!("running server at http://{}", addr);

    Server::bind(&addr).serve(make_svc).await
}
