extern crate common;
extern crate serde_json;
extern crate toml;

#[macro_use]
extern crate serde_derive;

use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::env;
use std::fs::File;
use std::path::Path;
use std::net::{SocketAddr, TcpListener};

use common::{PieceKind, PieceColour, Piece, Board, StateChange, Action, Vote};

#[derive(Deserialize, Debug)]
struct Config {
    host: SocketAddr
}

impl Config {
    fn from_file<P: AsRef<Path> + Clone>(path: P) -> Config {
        let mut file = File::open(&path).expect("Could not open config file.");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Reading config file failed");
        toml::from_str(&contents).expect("Format file incorrectly formatted")
    }
}

fn init_board() -> Board {
    let mut inner = [[None; 8]; 8];
    inner[0][0] = Some(Piece { kind: PieceKind::Rook, colour: PieceColour::Black });
    inner[0][1] = Some(Piece { kind: PieceKind::Knight, colour: PieceColour::Black });
    inner[0][2] = Some(Piece { kind: PieceKind::Bishop, colour: PieceColour::Black });
    inner[0][3] = Some(Piece { kind: PieceKind::Queen, colour: PieceColour::Black });
    inner[0][4] = Some(Piece { kind: PieceKind::King, colour: PieceColour::Black });
    inner[0][5] = Some(Piece { kind: PieceKind::Bishop, colour: PieceColour::Black });
    inner[0][6] = Some(Piece { kind: PieceKind::Knight, colour: PieceColour::Black });
    inner[0][7] = Some(Piece { kind: PieceKind::Rook, colour: PieceColour::Black });

    inner[1][0] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][1] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][2] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][3] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][4] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][5] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][6] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });
    inner[1][7] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::Black });

    inner[6][0] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][1] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][2] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][3] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][4] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][5] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][6] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });
    inner[6][7] = Some(Piece { kind: PieceKind::Pawn, colour: PieceColour::White });

    inner[7][0] = Some(Piece { kind: PieceKind::Rook, colour: PieceColour::White });
    inner[7][1] = Some(Piece { kind: PieceKind::Knight, colour: PieceColour::White });
    inner[7][2] = Some(Piece { kind: PieceKind::Bishop, colour: PieceColour::White });
    inner[7][3] = Some(Piece { kind: PieceKind::Queen, colour: PieceColour::White });
    inner[7][4] = Some(Piece { kind: PieceKind::King, colour: PieceColour::White });
    inner[7][5] = Some(Piece { kind: PieceKind::Bishop, colour: PieceColour::White });
    inner[7][6] = Some(Piece { kind: PieceKind::Knight, colour: PieceColour::White });
    inner[7][7] = Some(Piece { kind: PieceKind::Rook, colour: PieceColour::White });

    Board(inner)
}

fn process_move(board: &mut Board, turn: PieceColour, action: Action) -> PieceColour {
    let Action { from: (x0, y0), to: (x1, y1) } = action;
    let &mut Board(ref mut inner) = board;

    if x0 >= 8 || x1 >= 8 || y0 >= 8 || y1 >= 8 {
        // das bad
        return turn;
    }

    if let Some(piece) = inner[y0 as usize][x0 as usize].take() {
        inner[y1 as usize][x1 as usize] = Some(piece);
    } else {
        return turn;
    }

    match turn {
        PieceColour::White => PieceColour::Black,
        PieceColour::Black => PieceColour::White
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        panic!("USAGE: engine configpath");
    }
    let config = Config::from_file(&args[1]);
    let (mut input, mut output) = {
        let listen = TcpListener::bind(config.host).unwrap();
        let (raw_input, _) = listen.accept().unwrap();
        (BufReader::new(raw_input.try_clone().unwrap()),
            BufWriter::new(raw_input.try_clone().unwrap()))
    };

    let mut buffer = String::new();
    let mut state = StateChange { board: init_board(), turn: PieceColour::White };
    loop {
        serde_json::to_writer(&mut output, &state).unwrap();
        writeln!(&mut output, "").unwrap();
        output.flush().unwrap();
        input.read_line(&mut buffer).unwrap();
        let Vote { action, .. } = serde_json::from_str(&*buffer).unwrap();
        state.turn = process_move(&mut state.board, state.turn, action);
    }
}
