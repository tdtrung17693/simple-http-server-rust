use crate::router::{Request, Response};

pub trait StatefulHandler<T, S>: Clone + Send + Sized + 'static {
    fn call(&self, request: Request, state: S) -> Response;
}

struct StatefulHandlerShell<H, T, S> {
    handler: H,
    state: S,
    _marker: std::marker::PhantomData<fn() -> T>,
}

pub trait StatelessHandler: Send   {
    fn call(self: Box<Self>, request: Request) -> Response;
    fn clone_box(&self) -> Box<dyn StatelessHandler + Sync>;
}

pub struct StatelessHandlerImpl(Box<dyn StatelessHandler + Sync>);

impl StatelessHandlerImpl {
    pub fn call(self, request: Request) -> Response {
        self.0.call(request)
    }
}

impl Clone for StatelessHandlerImpl {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

impl<H, T, S> StatelessHandler for StatefulHandlerShell<H, T, S>
where
    H: StatefulHandler<T, S> + Sync,
    T: Send + 'static,
    S: Send + Sync + Clone + 'static,
{
    fn call(self: Box<Self>, request: Request) -> Response {
        self.handler.call(request, self.state.clone())
    }

    fn clone_box(&self) -> Box<dyn StatelessHandler  + Sync> {
        Box::new(StatefulHandlerShell {
            handler: self.handler.clone(),
            state: self.state.clone(),
            _marker: std::marker::PhantomData,
        })
    }
}

trait IntoStatelessHandler<S>: Send {
    fn clone_box(&self) -> Box<dyn IntoStatelessHandler<S> + Sync>;
    fn into_stateless_handler(self: Box<Self>, state: S) -> StatelessHandlerImpl;
}

struct MakeIntoStatelessHandler<H, S> {
    handler: H,
    into_stateless_handler: fn(H, S) -> StatelessHandlerImpl,
}

impl<H, S> IntoStatelessHandler<S> for MakeIntoStatelessHandler<H, S>
where
    H: Send + Sync + Clone + 'static,
    S: Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn IntoStatelessHandler<S> + Sync> {
        Box::new(MakeIntoStatelessHandler {
            handler: self.handler.clone(),
            into_stateless_handler: self.into_stateless_handler,
        })
    }

    fn into_stateless_handler(self: Box<Self>, state: S) -> StatelessHandlerImpl {
        (self.into_stateless_handler)(self.handler, state)
    }
}

    pub struct BoxedHandler<S>(Box<dyn IntoStatelessHandler<S> + Sync>);

impl<S> BoxedHandler<S> {
    pub fn from_handler<H, T>(handler: H) -> Self
    where
        H: StatefulHandler<T, S> + Send + Sync + Clone + 'static,
        T: Send + Clone + 'static,
        S: Send + Sync + Clone + 'static,
    {
        // Self(Box::new())
        Self(Box::new(MakeIntoStatelessHandler {
            handler,
            into_stateless_handler: |h, s| {
                StatelessHandlerImpl(Box::new(StatefulHandlerShell {
                    handler: h,
                    state: s,
                    _marker: std::marker::PhantomData,
                }))
            },
        }))
    }

    pub fn into_stateless_handler(self, state: S) -> StatelessHandlerImpl {
        self.0.into_stateless_handler(state)
    }
}

impl<S> Clone for BoxedHandler<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}
