use std::{future::Future, pin::Pin};

use super::{Request, Response};

trait Handler<P, S> {
    type Future: Future<Output = Response>;

    fn call(self, request: Request, state: S) -> Self::Future;
}

impl<F, Fut, Res, S> Handler<((),), S> for F
where
    F: FnOnce() -> Fut + 'static,
    Fut: Future<Output = Res> + 'static,
    Res: Into<Response>,
{
    type Future = Pin<Box<dyn Future<Output = Response>>>;

    fn call(self, request: Request, state: S) -> Self::Future {
        let _ = state;
        let _ = request;
        Box::pin(async move {
            let res = self().await;

            res.into()
        })
    }
}

impl <F, Fut, Res, S, T1> Handler<(T1,), S> for F
where
    F: FnOnce(T1) -> Fut + 'static,
    Fut: Future<Output = Res> + 'static,
    Res: Into<Response>,
{
    type Future = Pin<Box<dyn Future<Output = Response>>>;

