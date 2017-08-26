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

use tungstenite::{accept, WebSocket, Message};
use tungstenite::HandshakeError::{self, Interrupted};
use tungstenite::util::NonBlockingError;

use common::{Vote, StateChange};

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::io::{self, BufReader, ErrorKind};
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
    socket: BufReader<TcpStream>,
    buffer: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct VoteCall {
    vote_call: (),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum DownstreamMessage {
    StateChange(StateChange),
    VoteCall(VoteCall),
}

impl Upstream {
    fn new(socket: BufReader<TcpStream>) -> Upstream {
        Upstream {
            socket: socket,
            buffer: String::new(),
        }
    }
}

struct State {
    poll: Poll,
    listener: TcpListener,
    clients: Slab<Client>,
    upstream: Upstream,
    voting: bool,
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

impl State {
    fn new_client(&mut self) -> Result<(), tungstenite::error::Error> {
        let stream = self.listener.accept()?.0;

        let mut websocket = accept(stream, None);
        while let Err(Interrupted(in_progress)) = websocket {
            websocket = in_progress.handshake();
        }
        if let Err(HandshakeError::Failure(e)) = websocket {
            return Err(e);
        }
        let websocket = websocket.unwrap();

        let client = Client::new(websocket);

        let index = self.clients.insert(client);
        let client = self.clients.get(index).unwrap();
        self.poll.register(
            client.websocket.get_ref(),
            client_conn_token(index),
            Ready::readable(),
            PollOpt::edge()
            )?;

        info!("Connection established: {}",
                 client.websocket.get_ref().peer_addr()?);

        Ok(())
    }

    fn upstream_event(&mut self, event: &Event) -> io::Result<()> {
        if event.readiness().is_readable() {
            match self.upstream.socket.read_line(&mut self.upstream.buffer) {
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => return Ok(()),
                result @ Err(_) => {
                    result.expect("Error reading from upstream");
                },
                _ => (),
            }

            let message: DownstreamMessage = match serde_json::from_str(
                    self.upstream.buffer.trim_right()) {
                Ok(message) => message,
                Err(e) => {
                    warn!("Error parsing message from upstream: {:?}: \"{}\"",
                          e, self.upstream.buffer.trim_right());
                    self.upstream.buffer.clear();
                    return Ok(());
                },
            };

            let outgoing_message = serde_json::to_string(&message).unwrap();
            for (index, client) in &mut self.clients {
                let result = client.websocket.write_message(Message::text(outgoing_message.clone()));
                if let Err(e) = result {
                    match e.into_non_blocking() {
                        None => self.poll.register(
                            client.websocket.get_ref(),
                            client_conn_token(index),
                            Ready::readable(),
                            PollOpt::edge()
                            )?,
                        Some(e) => warn!("Error sending to websocket: {:?}", e),
                    }
                }
/*
                let e = match result {
                    Err(e) => match e.into_non_blocking() {
                        None => self.poll.register(
                            client.websocket.get_ref(),
                            client_conn_token(index),
                            Ready::readable(),
                            PollOpt::edge()
                            )?,
                        _ => warn!("Error sending to websocket: {:?}", a),
                    },
                    Ok(()) => (),
                }
                */
            }

            //println!("Upstream: {:?}", message);
            println!("##############");
            println!("{:?}", message);
            println!("{}", self.upstream.buffer);

            self.upstream.buffer.clear();
        }

        return Ok(());
    }

    fn register_client_readable(&self, index: usize) -> io::Result<()> {
        let client = self.clients.get(index).unwrap();

        self.poll.register(
            client.websocket.get_ref(),
            client_conn_token(index),
            Ready::readable(),
            PollOpt::edge()
            )
    }

    fn client_event(&mut self, event: &Event) -> Result<(), tungstenite::error::Error> {
        let index = client_conn_untoken(event.token());

        if event.readiness().is_readable() {
            let message = {
                let mut message;
                {
                    let client = self.clients.get_mut(index).unwrap();
                    message = client.websocket.read_message();
                }
                match message {
                    Ok(message) => message,
                    Err(e) => match e.into_non_blocking() {
                        None => {
                            self.register_client_readable(index);
                            return Ok(());
                        }
                        Some(e) => {
                            self.clients.remove(index);
                            return Err(e);
                        },
                    },
                }
            };

            let client = self.clients.get_mut(index).unwrap();
            if let Ok(message) = message.into_text() {
                match serde_json::from_str(&message) {
                    Ok(vote) => {
                        info!("Vote received: {:?}", vote);
                        println!("{:?}", vote);
                        client.vote = Some(vote);
                    },
                    Err(e) => warn!("Invalid message received: {}: \"{}\"", e, message),
                }

                //client.websocket.write_message(msg).unwrap();
            } else {
                warn!("Non-text message received");
            }
        }

        if event.readiness().is_writable() {
            /*
            let result = client.websocket.write_pending();
            if let Err(e) = result {
                match e.into_non_blocking() {
                    None => {
                        self.register_client_readable(index);
                        return Ok(());
                    },
                    Some(e) => warn!("Error sending to websocket: {:?}", e),
                }
            }
            */

            let mut message;
            {
                let client = self.clients.get_mut(index).unwrap();
                message = client.websocket.read_message();
            }
            match message {
                Ok(message) => (),
                Err(e) => match e.into_non_blocking() {
                    None => {
                        self.register_client_readable(index);
                        return Ok(());
                    }
                    Some(e) => {
                        self.clients.remove(index);
                        return Err(e);
                    },
                },
            }
        }

        Ok(())
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
    info!{"Connected to upstream {}", upstream_address};

    let poll = Poll::new().unwrap();
    poll.register(&listener, SERVER, Ready::readable(), PollOpt::edge()).unwrap();
    poll.register(&upstream_conn, UPSTREAM, Ready::readable(), PollOpt::edge()).unwrap();

    let mut events = Events::with_capacity(1024);

    let upstream_reader = BufReader::new(upstream_conn);

    let mut state = State {
        poll: poll,
        listener: listener,
        clients: Slab::new(),
        upstream: Upstream::new(upstream_reader),
        voting: false,
    };

    loop {
        state.poll.poll(&mut events, None).unwrap();

        for event in &events {
            match event.token() {
                SERVER => match state.new_client() {
                    Err(e) => warn!("Client accept failed: {:?}", e),
                    _ => (),
                },
                UPSTREAM => {
                    let _ = state.upstream_event(&event);
                },
                client @ Token(_) if is_client(client) => {
                    let _ = state.client_event(&event);
                }
                Token(_) =>
                    (),
            }
        }
    }
}
