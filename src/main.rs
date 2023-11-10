// Uncomment this block to pass the first stage
use std::{
    net::TcpListener, sync::Arc,
};

use router::{Request, Response};

mod router;

fn main() -> Result<(), std::io::Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let mut router = router::Router::new();
    router.get("/", home_page);
    router.get("/echo/:str", echo);
    router.get("/user-agent", user_agent);

    let router = Arc::new(router);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let router = router.clone();
                std::thread::spawn(move || {
                    router.execute(stream)
                });
            }
            Err(e) => {
                println!("error: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}


fn home_page(_request: Request) -> Response{
    Response {
        status_code: 200,
        body: "".into(),
        content_type: "text/plain".into(),
    }
}


fn echo(request: Request) -> Response {
    Response {
        status_code: 200,
        body: request.params.get("str").unwrap().to_string(),
        content_type: "text/plain".into(),
    }
}

fn user_agent(request: Request) -> Response {
    Response {
        status_code: 200,
        body: request.headers.get("user-agent").unwrap().to_string(),
        content_type: "text/plain".into(),
    }
}
