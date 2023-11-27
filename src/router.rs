use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
};

use crate::handler::{BoxedHandler, StatefulHandler, StatelessHandlerImpl};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
}

pub struct Request {
    pub path: String,
    pub method: Method,
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub enum Error {
    MethodNotAllowed(String),
    NotFound,
}

impl Method {
    pub fn from_str(str: &str) -> Result<Self, Error> {
        match str {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            _ => Err(Error::MethodNotAllowed(str.to_string())),
        }
    }
}

impl From<&TcpStream> for Request {
    fn from(mut stream: &TcpStream) -> Self {
        let mut buf_reader = BufReader::new(&mut stream);

        let mut line = Vec::new();
        let mut request: Vec<String> = Vec::new();
        loop {
            match buf_reader.read_until(b'\n', &mut line) {
                Ok(_) => {
                    if line == b"\r\n" {
                        break;
                    }
                    request.push(
                        String::from_utf8_lossy(&line)
                            .strip_suffix("\r\n")
                            .unwrap()
                            .to_string(),
                    );
                }
                Err(_) => {
                    break;
                }
            }
            line.clear();
        }

        let first_line = request[0].split_whitespace().collect::<Vec<_>>();
        let method = first_line[0];
        let path = first_line[1];

        let headers = request
            .iter()
            .skip(1)
            .map(|line| {
                let mut parts = line.splitn(2, ": ");
                (
                    parts.next().unwrap().to_string().to_lowercase(),
                    parts.next().unwrap().to_string(),
                )
            })
            .collect::<HashMap<String, String>>();

        let content_length = headers
            .get("content-length")
            .map_or(0, |v| v.parse().unwrap());
        let mut body = vec![0; content_length];
        buf_reader.read_exact(&mut body).unwrap();

        Self {
            path: path.into(),
            method: Method::from_str(method).unwrap(),
            params: HashMap::new(),
            headers,
            body,
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub content_type: String,
}

impl Response {
    fn get_status_message(&self) -> &str {
        match self.status_code {
            200 => "OK",
            404 => "Not Found",
            _ => "Unknown",
        }
    }
}

impl From<Response> for Vec<u8> {
    fn from(value: Response) -> Self {
        let header = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
            value.status_code,
            value.get_status_message(),
            value.content_type,
            value.body.len(),
        )
        .into_bytes();

        [header, value.body].concat()
    }
}

enum RouteHandler<S> {
    Stateful(BoxedHandler<S>),
    Stateless(StatelessHandlerImpl),
}

pub struct Route<S> {
    matcher: regex::Regex,
    handler: RouteHandler<S>,
    params: Vec<String>,
}

pub type RouteBag<S> = Vec<Route<S>>;

pub struct Router<S> {
    routes: HashMap<Method, RouteBag<S>>,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::NotFound
    }
}

impl<S> Route<S>
where
    S: Send + Sync + Clone + 'static,
{
    fn new<H, T>(path: &str, handler: H) -> Self
    where
        H: StatefulHandler<T, S> + Send + Sync + Clone + 'static,
        T: Send + Clone + 'static,
    {
        let params_regex = regex::Regex::new(r":([a-zA-Z0-9]+)").unwrap();
        let params = params_regex
            .find_iter(path)
            .map(|m| m.as_str().replace(':', "").to_string())
            .collect();

        let segments = params_regex.replace_all(path, "(?<$1>\\S+)");

        Self {
            matcher: regex::Regex::new(&format!("^{}$", segments)).unwrap(),
            params,
            handler: RouteHandler::Stateful(BoxedHandler::from_handler(handler)),
        }
    }

    fn with_state<S2>(self, state: S) -> Route<S2> {
        Route {
            matcher: self.matcher,
            params: self.params,
            handler: match self.handler {
                RouteHandler::Stateful(handler) => {
                    RouteHandler::Stateless(handler.into_stateless_handler(state))
                }
                RouteHandler::Stateless(handler) => RouteHandler::Stateless(handler),
            },
        }
    }
}

impl<S> Router<S>
where
    S: Send + Sync + Clone + 'static,
{
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
    pub fn get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: StatefulHandler<T, S> + Send + Sync + Clone + 'static,
        T: Send + Clone + 'static,
    {
        let route_bag =
            if let std::collections::hash_map::Entry::Vacant(e) = self.routes.entry(Method::Get) {
                e.insert(Vec::new())
            } else {
                self.routes.get_mut(&Method::Get).unwrap()
            };

        let route = Route::new(path, handler);
        route_bag.push(route);
        self
    }

    pub fn post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: StatefulHandler<T, S> + Send + Sync + Clone + 'static,
        T: Send + Clone + 'static,
    {
        let route_bag =
            if let std::collections::hash_map::Entry::Vacant(e) = self.routes.entry(Method::Post) {
                e.insert(Vec::new())
            } else {
                self.routes.get_mut(&Method::Get).unwrap()
            };

        let route = Route::new(path, handler);
        route_bag.push(route);
        self
    }

    fn get_route(&self, request: &mut Request) -> Option<&Route<S>> {
        let route_bag = self.routes.get(&request.method)?;
        let path = &request.path;
        route_bag.iter().find(|route| {
            if !route.matcher.is_match(path) {
                return false;
            }
            let matches = route.matcher.captures_iter(path);

            for (param, (_, [value])) in matches
                .zip(route.params.iter())
                .map(|(c, param)| (param, c.extract()))
            {
                request.params.insert(param.clone(), value.into());
            }
            true
        })
    }

    pub fn with_state<S2>(self, state: S) -> Router<S2> {
        Router {
            routes: self
                .routes
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|r| r.with_state(state.clone())).collect()))
                .collect(),
        }
    }
}

impl Router<()> {
    pub fn execute(&self, mut connection: TcpStream) -> Result<(), Error> {
        let mut request = Request::from(&connection);
        let route = self.get_route(&mut request);

        let response = route
            .map(|route| {
                let handler = &route.handler;

                match handler {
                    RouteHandler::Stateful(handler) => {
                        let handler = handler.clone();
                        let stateless_handler = handler.into_stateless_handler(());
                        stateless_handler.call(request)
                    }
                    RouteHandler::Stateless(handler) => {
                        let handler = handler.clone();
                        handler.call(request)
                    }
                }
            })
            .or_else(|| {
                Some(Response {
                    status_code: 404,
                    body: "Not Found".into(),
                    content_type: "text/plain".into(),
                })
            })
            .unwrap();

        let response: Vec<u8> = response.into();
        connection.write_all(&response[..])?;
        Ok(())
    }
}
