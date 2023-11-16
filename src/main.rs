// Uncomment this block to pass the first stage
use std::{env, net::TcpListener, sync::Arc};

use router::{Request, Response};
use tokio::sync::OnceCell;

mod router;
mod router_refact;

struct AppContext {
    file_directory: String,
}

static APP_CONTEXT: OnceCell<AppContext> = OnceCell::const_new();

fn main() -> Result<(), std::io::Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    let args = env::args().collect::<Vec<String>>();
    let file_directory: &str = args.get(2).and_then(|arg| Some(arg.as_str())).unwrap_or("");
    let _ = APP_CONTEXT
        .set(AppContext {
            file_directory: file_directory.to_string(),
        })
        .map_err(|_| ());

    let mut router = router::Router::new();
    router.get("/", home_page);
    router.get("/echo/:str", echo);
    router.get("/user-agent", user_agent);
    router.get("/files/:file", file_reading);
    router.post("/files/:file", file_uploading);

    let router = Arc::new(router);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let router = router.clone();
                std::thread::spawn(move || router.execute(stream));
            }
            Err(e) => {
                println!("error: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

fn home_page(_request: Request) -> Response {
    Response {
        status_code: 200,
        body: "".into(),
        content_type: "text/plain".into(),
    }
}

fn echo(request: Request) -> Response {
    Response {
        status_code: 200,
        body: request.params.get("str").unwrap().bytes().collect(),
        content_type: "text/plain".into(),
    }
}

fn user_agent(request: Request) -> Response {
    Response {
        status_code: 200,
        body: request.headers.get("user-agent").unwrap().bytes().collect(),
        content_type: "text/plain".into(),
    }
}

fn file_reading(request: Request) -> Response {
    let file_name = request.params.get("file").unwrap();
    let file_path = format!(
        "{}/{}",
        APP_CONTEXT.get().unwrap().file_directory,
        file_name
    );
    let file_content = std::fs::read(file_path);
    if file_content.is_err() {
        return Response {
            status_code: 404,
            body: "Not Found".into(),
            content_type: "text/plain".into(),
        };
    }

    let file_content = file_content.unwrap();
    Response {
        status_code: 200,
        body: file_content,
        content_type: "application/octet-stream".into(),
    }
}

fn file_uploading(request: Request) -> Response {
    let file_name = request.params.get("file").unwrap();
    let file_content = request.body;
    if file_content.is_empty() {
        return Response {
            status_code: 400,
            body: "Bad Request".into(),
            content_type: "text/plain".into(),
        }
    }

    let file_path = format!(
        "{}/{}",
        APP_CONTEXT.get().unwrap().file_directory,
        file_name
    );

    if let Err(err) = std::fs::write(file_path, file_content) {
        println!("error: {}", err);
        return Response {
            status_code: 500,
            body: "Internal Server Error".into(),
            content_type: "text/plain".into(),
        }
    }


    Response {
        status_code: 201,
        body: "File Created".into(),
        content_type: "text/plain".into(),
    }
}
