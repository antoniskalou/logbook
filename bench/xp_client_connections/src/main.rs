use std::{io::Read, time::Duration, net::TcpStream};

pub const SERVER_ADDR: &str = "127.0.0.1:52000";
pub const NUM_CLIENTS: i32 = 100;

fn main() {
    let n = std::env::args().nth(1)
        .expect("Usage: xp_client_connections <NUM CLIENTS>")
        .parse()
        .expect("invalid parameter, must be a number");
    let streams = (0..n)
        .map(|i| {
            let i = i + 1;
            println!("#{i} opening connection...");
            let stream = TcpStream::connect(SERVER_ADDR).unwrap();
            // stream.set_nonblocking(true).unwrap();
            stream.set_read_timeout(Some(Duration::from_millis(10))).unwrap();
            stream
        })
        .collect::<Vec<TcpStream>>();

    let mut buf = [0; 256];
    loop {
        for (i, mut stream) in streams.iter().enumerate() {
            let i = i + 1;
            match stream.read(&mut buf) {
                Ok(n) => println!("#{i} message received, {n} bytes"),
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => eprintln!("Error reading stream: {e}"),
            }
        }
    }
}
