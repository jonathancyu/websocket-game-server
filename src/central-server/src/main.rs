use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use common::threadpool::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    let pool = ThreadPool::new(8);
    for stream in listener.incoming().take(10) {
        let stream = stream.unwrap();
        pool.execute(|| handle_connection(stream));
    }
}
fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let response = "HTTP/1.1 200 OK\r\n\r\n";

    stream.write_all(response.as_bytes()).unwrap();
    println!("Request: {http_request:#?}");
}
