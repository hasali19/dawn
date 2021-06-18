use std::collections::HashMap;

use async_trait::async_trait;
use hyper::Method;
use route_recognizer::{Match, Params};

use crate::{Handler, Request};

#[derive(Default)]
pub struct Router {
    method_map: HashMap<Method, route_recognizer::Router<Box<dyn Handler>>>,
}

#[async_trait]
impl Handler for Router {
    async fn run(&self, req: crate::Request, next: &dyn crate::Next) -> crate::Result {
        let m = self
            .method_map
            .get(req.method())
            .and_then(|r| r.recognize(req.path()).ok())
            .map(|Match { handler, params }| (handler, params));

        let (handler, params) = match m {
            Some(val) => val,
            None => return next.run(req).await,
        };

        handler.run(req.with_ext(params), next).await
    }
}

macro_rules! method_fn {
    ($name:ident, $method:ident) => {
        pub fn $name(&mut self, path: &str, handler: impl Handler) {
            self.route(Method::$method, path, handler);
        }
    };
}

impl Router {
    pub fn new() -> Self {
        Router {
            method_map: Default::default(),
        }
    }

    pub fn build(mut self, builder: impl Fn(&mut Router)) -> Self {
        builder(&mut self);
        self
    }

    pub fn route(&mut self, method: Method, path: &str, handler: impl Handler) {
        self.method_map
            .entry(method)
            .or_insert_with(Default::default)
            .add(path, Box::new(handler));
    }

    method_fn!(connect, CONNECT);
    method_fn!(delete, DELETE);
    method_fn!(get, GET);
    method_fn!(head, HEAD);
    method_fn!(options, OPTIONS);
    method_fn!(patch, PATCH);
    method_fn!(post, POST);
    method_fn!(put, PUT);
    method_fn!(trace, TRACE);
}

pub trait RequestExt {
    fn param(&self, name: &str) -> Option<&str>;
}

impl RequestExt for Request {
    fn param(&self, name: &str) -> Option<&str> {
        self.ext::<Params>().and_then(|params| params.find(name))
    }
}
