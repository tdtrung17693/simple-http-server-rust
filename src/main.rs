// Uncomment this block to pass the first stage
use std::{env, net::TcpListener, sync::Arc};

use router::{Request, Response, Router};

mod handler;
mod impl_handler;
mod router;

struct AppContext {
    file_directory: String,
}

fn main() -> Result<(), std::io::Error> {
    let args = env::args().collect::<Vec<String>>();
    let current_directory = env::current_dir().map(|dir| dir.to_string_lossy().to_string()).unwrap_or("./".into());
    let file_directory: String = args.get(2).map(|arg| arg.to_string()).unwrap_or(format!("{current_directory}/files"));
    let state = Arc::new(AppContext {
        file_directory,
    });

    println!("File server directory: {}", state.file_directory);
    let router = router::Router::new()
        .get("/files/:file", file_reading)
        .post("/files/:file", file_uploading)
        .with_state(state)
        .get("/", home_page)
        .get("/echo/:str", echo)
        .get("/user-agent", user_agent);

    let app = App::new(router, 4221);
    app.start()
}

struct App {
    router: Arc<Router<()>>,
    port: u16,
}

impl App {
    pub fn new(router: Router<()>, port: u16) -> Self {
        Self {
            router: Arc::new(router),
            port,
        }
    }

    pub fn start(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).unwrap();
        println!("Listening on port {}...", self.port);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let router = self.router.clone();
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

fn file_reading(request: Request, state: Arc<AppContext>) -> Response {
    let file_name = request.params.get("file").unwrap();
    let file_path = format!(
        "{}/{}",
        state.file_directory,
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

fn file_uploading(request: Request, state: Arc<AppContext>) -> Response {
    let file_name = request.params.get("file").unwrap();
    let file_content = request.body;
    if file_content.is_empty() {
        return Response {
            status_code: 400,
            body: "Bad Request".into(),
            content_type: "text/plain".into(),
        };
    }

    let file_path = format!(
        "{}/{}",
        state.file_directory,
        file_name
    );

    if let Err(err) = std::fs::write(file_path, file_content) {
        println!("error: {}", err);
        return Response {
            status_code: 500,
            body: "Internal Server Error".into(),
            content_type: "text/plain".into(),
        };
    }

    Response {
        status_code: 201,
        body: "File Created".into(),
        content_type: "text/plain".into(),
    }
}
