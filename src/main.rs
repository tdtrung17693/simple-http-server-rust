// Uncomment this block to pass the first stage
use std::{
    io::{BufRead, BufReader, Read, Write},
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
    let mut data: Vec<u8> = vec![];
    let mut buf_reader = BufReader::new(&mut stream);
    let request = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect::<Vec<_>>();

    stream.write_all("HTTP/1.1 200 OK\r\n\r\n".as_bytes())?;
    println!("response sent");

    Ok(())
}
