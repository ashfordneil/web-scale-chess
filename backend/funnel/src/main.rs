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

extern crate rand;

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
use std::time::{self, Duration};
use std::net::SocketAddr;

use rand::Rng;

#[derive(Serialize, Deserialize)]
struct Config {
    host: SocketAddr,
    upstream: SocketAddr,
    vote_length: Duration,
    vote_timeout: Duration,
    timeout_change: Duration,
    start_vote: bool,
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
    timeout: u32,
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

struct State<'a> {
    config: &'a Config,
    poll: Poll,
    listener: TcpListener,
    clients: Slab<Client>,
    upstream: Upstream,
    latest_state: Option<StateChange>,
    voting: bool,
    next_vote_send: Option<time::Instant>,
    next_vote_start: Option<time::Instant>,
}

fn read_config<P: AsRef<Path> + Clone>(path: P) -> Config {
    let mut file = File::open(&path)
        .expect(&format!("Could not open config file: {:?}", path.as_ref()));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Reading config file failed");
    toml::from_str(&contents).expect("Format file incorrectly formatted")
}

fn duration_millis(d: Duration) -> u32 {
    d.as_secs() as u32 * 1000 + d.subsec_nanos() / 1_000_000
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

impl<'a> State<'a> {
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
        {
            let client = self.clients.get_mut(index).unwrap();
            self.poll.register(
                client.websocket.get_ref(),
                client_conn_token(index),
                Ready::readable(),
                PollOpt::edge()
                )?;

            info!("Connection established: {}",
                     client.websocket.get_ref().peer_addr()?);
        }

        let mut message = None;

        if let Some(ref state) = self.latest_state {
            message = Some(serde_json::to_string(state).unwrap());
        }

        if let Some(message) = message {
            self.send_client_message(index, message)?;
        }

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

            let mut message: DownstreamMessage = match serde_json::from_str(
                    self.upstream.buffer.trim_right()) {
                Ok(message) => message,
                Err(e) => {
                    warn!("Badly formatted message from upstream: {:?}: \"{}\"",
                          e, self.upstream.buffer.trim_right());
                    self.upstream.buffer.clear();
                    return Ok(());
                },
            };

            info!("Received from upstream: {:?}", message);

            if let DownstreamMessage::VoteCall(ref mut vote_call) = message {
                info!("Vote call!");
                self.next_vote_send = Some(time::Instant::now() + (Duration::from_millis(vote_call.timeout as u64) - self.config.timeout_change));
                vote_call.timeout = vote_call.timeout - duration_millis(self.config.timeout_change);
            }

            let outgoing_message = serde_json::to_string(&message).unwrap();
            let mut clients = Vec::new();
            for (index, _) in &self.clients {
                clients.push(index);
            }
            for &index in &clients {
                self.send_client_message(index, outgoing_message.clone());
            }
                /*
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
                */

            if let DownstreamMessage::StateChange(state) = message {
                info!("UPDATING LATEST STATE");
                self.latest_state = Some(state);
                self.next_vote_start = Some(time::Instant::now() + self.config.vote_length);
            }

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

    fn client_readable_event(&mut self, event: &Event) -> Result<(), tungstenite::error::Error> {
        let index = client_conn_untoken(event.token());

        let message = {
            let message;
            {
                let client = self.clients.get_mut(index).unwrap();
                message = client.websocket.read_message();
            }
            match message {
                Ok(message) => message,
                Err(e) => match e.into_non_blocking() {
                    None => {
                        self.register_client_readable(index)?;
                        return Ok(());
                    }
                    Some(e) => {
                        self.clients.remove(index);
                        return Err(e);
                    },
                },
            }
        };

        let message = match message {
            Message::Text(text) => match serde_json::from_str(&text) {
                Ok(decoded) => {
                    info!("Received text from client: {:?}", decoded);
                    decoded
                }
                Err(e) => {
                    warn!("Badly formatted text received from client {}", e);
                    return Ok(());
                }
            },
            Message::Binary(vec) => match serde_json::from_str(std::str::from_utf8(&vec).unwrap()) {
                Ok(decoded) => {
                    info!("Received binary from client: {:?}", decoded);
                    decoded
                }
                Err(e) => {
                    warn!("Badly formatted binary received from client {}", e);
                    return Ok(());
                }
            },
            _ => panic!("dammit no ping pong"),
        };


/*

        let message: Vote = match message.into_text() {
            Ok(text) =>             Err(e) => {
                warn!("Non-text message received {}", e);
                return Err(e);
            },
        };
        */

        {
            let client = self.clients.get_mut(index).unwrap();
            client.vote = Some(message);
        }

        /*
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
        }
        */

        Ok(())
    }

    fn client_writable_event(&mut self, event: &Event) -> Result<(), tungstenite::error::Error> {
        let index = client_conn_untoken(event.token());

        let message;
        {
            let client = self.clients.get_mut(index).unwrap();
            message = client.websocket.read_message();
        }
        match message {
            Ok(_) => (),
            Err(e) => match e.into_non_blocking() {
                None => {
                    let _ = self.register_client_readable(index);
                    return Ok(());
                }
                Some(e) => {
                    self.clients.remove(index);
                    return Err(e);
                },
            },
        }

        Ok(())
    }

    fn send_client_message(&mut self, index: usize, message: String) -> io::Result<()> {
        let client = self.clients.get_mut(index).unwrap();

        let result = client.websocket.write_message(Message::text(message));
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

        if let Err(e) = client.websocket.write_pending() {
            match e.into_non_blocking() {
                None => self.poll.register(
                    client.websocket.get_ref(),
                    client_conn_token(index),
                    Ready::writable(),
                    PollOpt::edge()
                    )?,
                Some(e) => warn!("Error sending to websocket: {:?}", e),
            }
        }

        Ok(())
    }

    fn start_vote(&mut self) {
        info!("Starting a vote call");
        let vote_call = VoteCall { timeout: duration_millis(self.config.vote_timeout) };
        let vote_message = DownstreamMessage::VoteCall(vote_call);
        let string = serde_json::to_string(&vote_message).unwrap();
        let mut clients = Vec::new();
        for (index, _) in &self.clients {
            clients.push(index);
        }
        for &index in &clients {
            self.send_client_message(index, string.clone());
        }
    }

    fn send_vote_upstream(&mut self) {
        let mut voted = Vec::new();

        for (index, client) in &self.clients {
            if let Some(_) = client.vote {
                voted.push(index);
            }
        }

        // TODO: weighting
        // TODO: send when 0 votes

        info!("Sending votes for {} upstream", voted.len());

        if voted.len() == 0 {
            return;
        }

        let index = rand::thread_rng().gen_range(0, voted.len());

        let mut vote = None;
        std::mem::swap(&mut vote, &mut self.clients.get_mut(index).unwrap().vote);

        if let Some(mut vote) = vote {
            vote.weight = voted.len() as u32;

            let mut message = serde_json::to_string(&vote).unwrap();
            message.push('\n');
            let bytes = message.as_bytes();

            println!("{}", message);

            let mut sent = 0;
            while sent < bytes.len() {
                match self.upstream.socket.get_ref().write(&bytes[sent..]) {
                    Ok(size) => sent += size,
                    Err(e) => match e.kind() {
                        ErrorKind::WouldBlock => continue,
                        _ => {
                            warn!("Sending message upstream failed");
                            break;
                        },
                    }
                }
            }

            info!("Votes sent!");
        }

        for (index, client) in &mut self.clients {
            client.vote = None;
        }
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

    let listener = TcpListener::bind(&config.host).expect("Could not bind to host");
    info!{"Listening on {}", config.host};

    let upstream_conn = TcpStream::connect(&config.upstream).expect("Could not connect to upsteam");
    info!{"Connected to upstream {}", config.upstream};

    let poll = Poll::new().unwrap();
    poll.register(&listener, SERVER, Ready::readable(), PollOpt::edge()).unwrap();
    poll.register(&upstream_conn, UPSTREAM, Ready::readable(), PollOpt::edge()).unwrap();

    let mut events = Events::with_capacity(1024);

    let upstream_reader = BufReader::new(upstream_conn);

    let mut state = State {
        config: &config,
        poll: poll,
        listener: listener,
        clients: Slab::new(),
        upstream: Upstream::new(upstream_reader),
        latest_state: None,
        voting: false,
        next_vote_send: None,
        next_vote_start: None,
    };

    if config.start_vote {
        state.next_vote_start = Some(time::Instant::now() + config.vote_length);
    }

    loop {
        let time = time::Instant::now();
        let mut timeout = None;

        let mut starting_yet = false;
        if let Some(next_vote_start) = state.next_vote_start {
            if time >= next_vote_start {
                starting_yet = true;
            } else {
                starting_yet = false;
                timeout = Some(next_vote_start - time);
            }
        }
        if starting_yet {
            state.start_vote();
            state.next_vote_start = None;
            state.next_vote_send = Some(time + config.vote_timeout);
        }

        let mut sending_yet = false;
        if let Some(next_vote_send) = state.next_vote_send {
            if time >= next_vote_send {
                sending_yet = true;
            } else {
                sending_yet = false;
                timeout = Some(next_vote_send - time);
            }
        }
        if sending_yet {
            info!("About to send vote");
            state.send_vote_upstream();
            state.next_vote_send = None;
        }
/*
        if state.next_vote + config.vote_timeout > time {
            timeout = Some(state.next_vote + config.vote_timeout - time);
        }

        let mut time_until_vote;
        if time > state.next_vote {
            state.next_vote += config.vote_length;
            time_until_vote = state.next_vote - time::Instant::now();
            state.send_vote_upstream();
        }
        time_until_vote = state.next_vote - time::Instant::now();
        */

        state.poll.poll(&mut events, timeout).unwrap();

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
                    if event.readiness().is_readable() {
                        let _ = state.client_readable_event(&event);
                    }
                    if event.readiness().is_writable() {
                        let _ = state.client_writable_event(&event);
                    }
                }
                Token(_) =>
                    (),
            }
        }
    }
}
