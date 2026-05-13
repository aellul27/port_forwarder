use serde::Deserialize;
use std::env;
use std::error::Error;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, UdpSocket};

#[derive(Debug, Deserialize, Clone)]
struct ConfigEntry {
    pub input_ip: String,
    pub input_port: u16,
    pub output_ip: String,
    pub output_port: u16,
    pub protocol: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config.json>", args[0]);
        std::process::exit(1);
    }

    let config_path = &args[1];
    let config_data = fs::read_to_string(config_path)?;
    let config: Vec<ConfigEntry> = serde_json::from_str(&config_data)?;

    let mut handles = vec![];

    for entry in config {
        let handle = tokio::spawn(async move {
            if let Err(e) = run_forwarder(entry).await {
                eprintln!("Error in forwarder: {}", e);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

async fn run_forwarder(entry: ConfigEntry) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listen_addr: SocketAddr = format!("{}:{}", entry.output_ip, entry.output_port).parse()?;
    let target_addr: SocketAddr = format!("{}:{}", entry.input_ip, entry.input_port).parse()?;

    println!(
        "Listening on {} ({}), forwarding to {}",
        listen_addr, entry.protocol, target_addr
    );

    match entry.protocol.to_lowercase().as_str() {
        "tcp" => {
            let listener = TcpListener::bind(listen_addr).await?;
            loop {
                let (mut client_stream, peer_addr) = listener.accept().await?;
                println!("New TCP connection from {}", peer_addr);

                tokio::spawn(async move {
                    if let Ok(mut server_stream) = TcpStream::connect(target_addr).await {
                        let (mut client_read, mut client_write) = client_stream.split();
                        let (mut server_read, mut server_write) = server_stream.split();

                        let client_to_server = tokio::io::copy(&mut client_read, &mut server_write);
                        let server_to_client = tokio::io::copy(&mut server_read, &mut client_write);

                        let _ = tokio::try_join!(client_to_server, server_to_client);
                    } else {
                        eprintln!("Failed to connect to target {}", target_addr);
                    }
                });
            }
        }
        "udp" => {
            let socket = Arc::new(UdpSocket::bind(listen_addr).await?);
            let mut buf = vec![0u8; 65535];
            
            let client_map: Arc<tokio::sync::Mutex<std::collections::HashMap<SocketAddr, Arc<UdpSocket>>>> = 
                Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

            // Bidirectional stateful UDP proxy multiplexer
            loop {
                let (size, peer) = socket.recv_from(&mut buf).await?;
                let mut map = client_map.lock().await;

                let target_sock = if let Some(sock) = map.get(&peer) {
                    sock.clone()
                } else {
                    let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
                    let _ = sock.connect(target_addr).await;
                    
                    let sock_recv = sock.clone();
                    let socket_send = socket.clone();
                    
                    // Task to handle target-to-client traffic
                    tokio::spawn(async move {
                        let mut b = vec![0u8; 65535];
                        while let Ok(n) = sock_recv.recv(&mut b).await {
                            if n == 0 { break; }
                            let _ = socket_send.send_to(&b[..n], peer).await;
                        }
                    });
                    
                    map.insert(peer, sock.clone());
                    sock
                };

                let _ = target_sock.send(&buf[..size]).await;
            }
        }
        _ => {
            eprintln!("Unsupported protocol: {}", entry.protocol);
            Ok(())
        }
    }
}
