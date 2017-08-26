extern crate common;
extern crate serde_json;

use std::io;
use std::io::prelude::*;

use common::{PieceKind, PieceColour, Piece, Board, StateChange, Action};

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
    let input = io::stdin();
    let mut buffer = String::new();
    let mut state = StateChange { board: init_board(), turn: PieceColour::White };
    loop {
        serde_json::to_writer(io::stdout(), &state).unwrap();
        println!("");
        input.lock().read_line(&mut buffer).unwrap();
        let action = serde_json::from_str(&*buffer).unwrap();
        state.turn = process_move(&mut state.board, state.turn, action);
    }
}
