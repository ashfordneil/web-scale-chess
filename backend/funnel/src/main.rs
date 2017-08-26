extern crate slab;
extern crate mio;
extern crate tungstenite;

#[macro_use]
extern crate serde_derive;

extern crate serde_json;
extern crate toml;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate common;

use slab::Slab;

use mio::*;
use mio::net::{TcpListener,TcpStream};

use tungstenite::accept;
use tungstenite::WebSocket;
use tungstenite::HandshakeError::Interrupted;

use common::{Vote, Action};

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::io;
use std::env;

#[derive(Serialize, Deserialize)]
struct Config {
    host: String,
    upstream: String,
}


struct Client {
    vote: Option<Vote>,
    websocket: WebSocket<TcpStream>,
}

impl Client {
    fn new(socket: WebSocket<TcpStream>) -> Client {
        Client {
            vote: None,
            websocket: socket,
        }
    }
}

struct Upstream {
    socket: TcpStream,
    buffer: String,
}

impl Upstream {
    fn new(socket: TcpStream) -> Upstream {
        Upstream {
            socket: socket,
            buffer: String::new(),
        }
    }
}

fn read_config<P: AsRef<Path> + Clone>(path: P) -> Config {
    let mut file = File::open(&path)
        .expect(&format!("Could not open config file: {:?}", path.as_ref()));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Reading config file failed");
    toml::from_str(&contents).expect("Format file incorrectly formatted")
}


const SERVER: Token = Token(0);
const UPSTREAM: Token = Token(1);
const FIRST_CLIENT: Token = Token(2);

fn client_conn_token(index: usize) -> Token {
    Token(index + FIRST_CLIENT.0)
}

fn client_conn_untoken(token: Token) -> usize {
    token.0 - FIRST_CLIENT.0
}

fn is_client(token: Token) -> bool {
    token.0 >= FIRST_CLIENT.0
}

fn new_client(poll: &Poll, listener: &TcpListener, clients: &mut Slab<Client>) -> io::Result<()> {
    let stream = listener.accept()?.0;

    let mut websocket = accept(stream, None);
    while let Err(Interrupted(in_progress)) = websocket {
        websocket = in_progress.handshake();
    }
    let websocket = websocket.unwrap();

    let client = Client::new(websocket);

    let index = clients.insert(client);
    let client = clients.get(index).unwrap();
    poll.register(
        client.websocket.get_ref(),
        client_conn_token(index),
        Ready::readable(),
        PollOpt::edge()
        )?;

    info!("Connection established: {}",
             client.websocket.get_ref().peer_addr()?);

    Ok(())
}

fn client_event(poll: &Poll, event: &Event, clients: &mut Slab<Client>) {
    let index = client_conn_untoken(event.token());
    let client = clients.get_mut(index).unwrap();

    if event.readiness().is_readable() {
        let msg = client.websocket.read_message().unwrap();

        if let Ok(msg) = msg.into_text() {
            match serde_json::from_str(&msg) {
                Ok(vote) => {
                    info!("Vote received: {:?}", vote);
                    println!("{:?}", vote);
                    client.vote = Some(vote);
                },
                Err(e) => warn!("Invalid message received: {}: \"{}\"", e, msg),
            }

            //client.websocket.write_message(msg).unwrap();
        } else {
            warn!("Non-text message received");
        }
    }
}

fn upstream_event(poll: &Poll, event: &Event, upstream: &mut Upstream) {
    if event.readiness().is_readable() {
        match upstream.socket.read_to_string(&mut upstream.buffer) {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return,
            result @ Err(_) => {
                result.expect("Error reading from upstream");
            },
            _ => (),
        }
        println!("Upstream: {}", upstream.buffer);
    }
}

fn main() {
    env_logger::init().unwrap();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!{"USAGE: funnel configpath"};
        std::process::exit(1);
    }

    let config = read_config(&args[1]);

    let listener_address = config.host.parse().expect("Host not a valid address");
    let listener = TcpListener::bind(&listener_address).expect("Could not bind to host");
    info!{"Listening on {}", listener_address};

    let upstream_address = config.upstream.parse().expect("Upstream not a valid address");
    let upstream_conn = TcpStream::connect(&upstream_address).expect("Could not connect to upsteam");
    upstream_conn.set_nodelay(true);
    info!{"Connected to upstream {}", upstream_address};

    let poll = Poll::new().unwrap();
    poll.register(&listener, SERVER, Ready::readable(), PollOpt::edge()).unwrap();
    poll.register(&upstream_conn, UPSTREAM, Ready::readable(), PollOpt::level()).unwrap();

    let mut events = Events::with_capacity(1024);

    let mut clients = Slab::new();
    let mut upstream = Upstream::new(upstream_conn);

    loop {
        poll.poll(&mut events, None).unwrap();

        for event in &events {
            match event.token() {
                SERVER => {
                    let _ = new_client(&poll, &listener, &mut clients);
                },
                UPSTREAM =>
                    upstream_event(&poll, &event, &mut upstream),
                client @ Token(_) if is_client(client) =>
                    client_event(&poll, &event, &mut clients),
                Token(_) =>
                    (),
            }
        }
    }
}
