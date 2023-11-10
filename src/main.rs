// Uncomment this block to pass the first stage
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

fn main() -> Result<(), std::io::Error> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    // println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(|| {
                    println!("accepted new connection");
                    handle_connection(stream).expect("failed to handle");
                    println!("connection closed");
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

fn handle_connection(mut stream: TcpStream) -> Result<(), std::io::Error> {
    let buf_reader = BufReader::new(&mut stream);
    let request = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let first_line = request[0].split_whitespace().collect::<Vec<_>>();
    let method = first_line[0];
    let path = first_line[1];

    router(method, path, &mut stream)?;
    println!("request handled");

    Ok(())
}

fn router(method: &str, path: &str, stream: &mut TcpStream) -> Result<(), std::io::Error> {
    println!("method: {}", method);
    println!("path: {}", path);

    if method == "GET" && path == "/" {
        Ok(stream.write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())?)
    } else if method == "GET" && path.starts_with("/echo/") {
        Ok(echo(&path[6..], stream)?)
    } else {
        Ok(stream.write_all("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes())?)
    }
}

fn echo(str: &str, stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let response = build_text_response(str);
    stream.write_all(response.as_bytes())?;
    Ok(())
}

fn build_text_response(text: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", text.len(), text)
}
