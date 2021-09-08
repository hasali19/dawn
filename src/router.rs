use std::collections::HashMap;

use async_trait::async_trait;
use hyper::Method;
use routefinder::Captures;

use crate::{Handler, Request};

#[derive(Default)]
pub struct Router {
    method_map: HashMap<Method, routefinder::Router<Box<dyn Handler>>>,
}

struct MatchedPath(usize);

#[async_trait]
impl Handler for Router {
    async fn run(&self, mut req: crate::Request, next: &dyn crate::Next) -> Request {
        // If this is a nested router, we should skip the part of the path that has
        // already been matched.
        let offset = match req.take_ext::<MatchedPath>() {
            Some(MatchedPath(n)) => n,
            None => 0,
        };

        let path = req.uri().path();
        let m = self
            .method_map
            .get(req.method())
            .and_then(|r| r.best_match(&path[offset..]));

        let (handler, params) = match m {
            Some(val) => (val.handler(), val.captures()),
            None => return next.run(req).await,
        };

        // Calculate how much of the path has been matched.
        // If this is a wildcard route, calculate the length of the matched part using
        // some simple pointer arithmetic.
        // Otherwise it's just the length of the path, since the whole thing was matched.
        let start = match params.wildcard() {
            Some(wildcard) => offset + wildcard.as_ptr() as usize - path.as_ptr() as usize,
            None => path.len(),
        };

        let params = params.into_owned();

        req.set_ext(MatchedPath(start));
        req.set_ext(params);

        handler.run(req, next).await
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
            .add(path, Box::new(handler))
            .expect("invalid path");
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

pub trait RouterRequestExt {
    fn param(&self, name: &str) -> Option<&str>;
}

impl RouterRequestExt for Request {
    fn param(&self, name: &str) -> Option<&str> {
        self.ext::<Captures>().and_then(|params| params.get(name))
    }
}
