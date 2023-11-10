use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

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
        let buf_reader = BufReader::new(&mut stream);
        let request = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<_>>();

        let first_line = request[0].split_whitespace().collect::<Vec<_>>();
        let method = first_line[0];
        let path = first_line[1];

        let headers= request
            .iter()
            .skip(1)
            .map(|line| {
                let mut parts = line.splitn(2, ": ");
                (parts.next().unwrap().to_string().to_lowercase(), parts.next().unwrap().to_string())
            })
            .collect::<HashMap<String, String>>();


        Self {
            path: path.into(),
            method: Method::from_str(method).unwrap(),
            params: HashMap::new(),
            headers,
        }
    }
}

#[derive(Debug)]
pub struct Response {
    pub status_code: u16,
    pub body: String,
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

impl From<Response> for String {
    fn from(value: Response) -> Self {
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
            value.status_code,
            value.get_status_message(),
            value.content_type,
            value.body.len(),
            value.body
        )
    }
}

#[derive(Debug)]
pub struct Route {
    matcher: regex::Regex,
    handler: fn(Request) -> Response,
    params: Vec<String>,
}

pub type RouteBag = Vec<Route>;

pub struct Router {
    routes: HashMap<Method, RouteBag>,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::NotFound
    }
}

impl Route {
    fn new(path: &str, handler: fn(Request) -> Response) -> Self {
        let params_regex = regex::Regex::new(r":([a-zA-Z0-9]+)").unwrap();
        let params = params_regex
            .find_iter(path)
            .map(|m| m.as_str().replace(':', "").to_string())
            .collect();

        let segments = params_regex.replace_all(path, "(?<$1>\\S+)");

        Self {
            matcher: regex::Regex::new(&format!("^{}$", segments)).unwrap(),
            params,
            handler,
        }
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }
    pub fn get(&mut self, path: &str, handler: fn(Request) -> Response) {
        let route_bag =
            if let std::collections::hash_map::Entry::Vacant(e) = self.routes.entry(Method::Get) {
                e.insert(Vec::new())
            } else {
                self.routes.get_mut(&Method::Get).unwrap()
            };

        let route = Route::new(path, handler);
        route_bag.push(route);
    }

    pub fn execute(&self, mut connection: TcpStream) -> Result<(), Error> {
        let mut request = Request::from(&connection);
        let route = self.get_route(&mut request);

        let response = route
            .map(|route| {
                let handler = route.handler;

                handler(request)
            })
            .or_else(|| Some(Response {
                status_code: 404,
                body: "Not Found".into(),
                content_type: "text/plain".into(),
            })).unwrap();

        let response_str: String = response.into();
        connection.write_all(response_str.as_bytes())?;
        Ok(())
    }

    fn get_route(&self, request: &mut Request) -> Option<&Route> {
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
}
