use crate::{handler::StatefulHandler, router::{Request, Response}};

impl<F,S> StatefulHandler<(), S> for F
where
    F: Fn(Request) -> Response + Clone + Send + Sync + 'static,
    S: Send + Sync + Clone + 'static,
{
    fn call(&self, request: Request, _: S) -> Response {
        (self)(request)
    }
}

impl <F,S> StatefulHandler<(S,), S> for F
where
    F: Fn(Request, S) -> Response + Clone + Send + Sync + 'static,
    S: Send + Sync + Clone + 'static,
{
    fn call(&self, request: Request, state: S) -> Response {
        (self)(request, state)
    }
}
