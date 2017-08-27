extern crate tungstenite;
extern crate url;

#[macro_use]
extern crate serde_derive;

extern crate serde_json;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate rand;

extern crate common;

use tungstenite::{accept, WebSocket, Message, handshake};
use tungstenite::HandshakeError::{self, Interrupted};
use tungstenite::util::NonBlockingError;

use common::{Vote, StateChange, Action};

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::io::{self, BufReader, ErrorKind};
use std::env;
use std::time::{self, Duration};
use std::net::SocketAddr;

use rand::Rng;

fn main() {
    println!("Hello, world!");

    let mut sockets = Vec::new();

    for i in (0..5000) {
        let mut url = url::Url::parse("ws://127.0.0.1").unwrap();
        url.set_port(Some(2826));
        let request = handshake::client::Request::from(url);

        let mut websocket = tungstenite::connect(request).unwrap().0;

        let x1 = rand::thread_rng().gen_range(0, 8);
        let x2 = rand::thread_rng().gen_range(0, 8);
        let y1 = rand::thread_rng().gen_range(0, 8);
        let y2 = rand::thread_rng().gen_range(0, 8);
        let v = Vote {
            action: Action {
                from: (x1, y1),
                to: (x2, y2),
            },
            weight: 1,
        };

        let string = serde_json::to_string(&v).unwrap();
        let message = Message::text(string);
        
        websocket.write_message(message);
        websocket.write_pending();

        sockets.push(websocket);
    }

    loop {
    }
}
