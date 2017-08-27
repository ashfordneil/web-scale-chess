extern crate common;
extern crate serde_json;
extern crate toml;
extern crate itertools;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::env;
use std::fs::File;
use std::path::Path;
use std::net::{SocketAddr, TcpListener};

use common::{Action, Board, Piece, PieceColour, PieceKind, StateChange, Vote};

use itertools::Itertools;

#[derive(Deserialize, Debug)]
struct Config {
    host: SocketAddr,
}

impl Config {
    fn from_file<P: AsRef<Path> + Clone>(path: P) -> Config {
        let mut file = File::open(&path).expect("Could not open config file.");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Reading config file failed");
        toml::from_str(&contents).expect("Format file incorrectly formatted")
    }
}

fn init_board() -> Board {
    let mut inner = [[None; 8]; 8];
    inner[0][0] = Some(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
    });
    inner[0][1] = Some(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::Black,
    });
    inner[0][2] = Some(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::Black,
    });
    inner[0][3] = Some(Piece {
        kind: PieceKind::Queen,
        colour: PieceColour::Black,
    });
    inner[0][4] = Some(Piece {
        kind: PieceKind::King,
        colour: PieceColour::Black,
    });
    inner[0][5] = Some(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::Black,
    });
    inner[0][6] = Some(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::Black,
    });
    inner[0][7] = Some(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::Black,
    });

    inner[1][0] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][1] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][2] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][3] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][4] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][5] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][6] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });
    inner[1][7] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::Black,
    });

    inner[6][0] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][1] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][2] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][3] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][4] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][5] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][6] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });
    inner[6][7] = Some(Piece {
        kind: PieceKind::Pawn,
        colour: PieceColour::White,
    });

    inner[7][0] = Some(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
    });
    inner[7][1] = Some(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
    });
    inner[7][2] = Some(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
    });
    inner[7][3] = Some(Piece {
        kind: PieceKind::Queen,
        colour: PieceColour::White,
    });
    inner[7][4] = Some(Piece {
        kind: PieceKind::King,
        colour: PieceColour::White,
    });
    inner[7][5] = Some(Piece {
        kind: PieceKind::Bishop,
        colour: PieceColour::White,
    });
    inner[7][6] = Some(Piece {
        kind: PieceKind::Knight,
        colour: PieceColour::White,
    });
    inner[7][7] = Some(Piece {
        kind: PieceKind::Rook,
        colour: PieceColour::White,
    });

    Board(inner)
}

fn piece_between<T>(board: &[[Option<T>; 8]; 8], start: (u8, u8), stop: (u8, u8)) -> bool {
    let (x0, y0) = start;
    let (x1, y1) = stop;
    let dx = x1 as i8 - x0 as i8;
    let dy = y1 as i8 - y0 as i8;

    assert!(dx != 0 || dy != 0);

    if dx.abs() == dy.abs() {
        let xs: Box<Iterator<Item = u8>> = if dx > 0 {
            Box::new(x0 + 1..x1)
        } else {
            Box::new((x0 + 1..x1).rev())
        };
        let ys: Box<Iterator<Item = u8>> = if dy > 0 {
            Box::new(y0 + 1..y1)
        } else {
            Box::new((y0 + 1..y1).rev())
        };
        match xs.zip(ys)
            .find(|&(x, y)| board[y as usize][x as usize].is_some())
        {
            Some((x, y)) => {
                info!("Diagonal collision at ({},{})", x, y);
                true
            }
            _ => false,
        }
    } else {
        (dx == 0 && dy > 0 &&
            board[y0 as usize + 1..y1 as usize]
                .iter()
                .map(|x| &x[x0 as usize])
                .any(|x| x.is_some())) ||
            (dx == 0 && dy < 0 &&
                board[y1 as usize + 1..y0 as usize]
                    .iter()
                    .map(|x| &x[x0 as usize])
                    .any(|x| x.is_some())) ||
            (dy == 0 && dx > 0 &&
                board[y0 as usize][x0 as usize + 1..x1 as usize]
                    .iter()
                    .any(|x| x.is_some())) ||
            (dy == 0 && dx < 0 &&
                board[y0 as usize][x1 as usize + 1..x0 as usize]
                    .iter()
                    .any(|x| x.is_some()))
    }
}

/// Returns true if a move is possible (excluding check) and false otherwise
fn process_sans_check_check(
    board: &[[Option<Piece>; 8]; 8],
    from: (u8, u8),
    to: (u8, u8),
    turn: PieceColour,
) -> bool {
    let (x0, y0) = from;
    let (x1, y1) = to;
    debug!(
        "({},{}), ({},{}) -> {:?}",
        x0,
        y0,
        x1,
        y1,
        board[y0 as usize][x0 as usize]
    );

    if let Some(Piece { kind, colour }) = board[y0 as usize][x0 as usize] {
        if colour != turn {
            info!(
                "Move rejected as piece colour ({:?}) != current turn player ({:?})",
                colour,
                turn
            );
            return false;
        }

        if x0 == x1 && y0 == y1 {
            info!(
                "Move rejected as initial coordinates ({}, {}) == final coordinates ({}, {})",
                x0,
                y0,
                x1,
                y1
            );
            return false;
        }

        let dx = x1 as i8 - x0 as i8;
        let dy = y1 as i8 - y0 as i8;

        match kind {
            PieceKind::King => {
                if dx.abs() <= 1 && dy.abs() <= 1 {
                    // no possible way to be moving through things if you only move 1 square
                } else {
                    info!("Move rejected as king cannot move more than 1 square");
                    return false;
                }
            }
            PieceKind::Queen => if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
                if piece_between(&board, (x0, y0), (x1, y1)) {
                    info!("Move rejected as there is a piece in front of the queen");
                    return false;
                }
            } else {
                info!("Move rejected as the queen must move in a straight line");
                return false;
            },
            PieceKind::Bishop => if dx.abs() == dy.abs() {
                if piece_between(&board, (x0, y0), (x1, y1)) {
                    info!("Move rejected as there is a piece in front of the bishop");
                    return false;
                }
            } else {
                info!("Move rejected as the bishop must move in a diagonal line");
                return false;
            },
            PieceKind::Knight => {
                if dx.abs() == 2 && dy.abs() == 1 {
                    // horsy can jump over things
                } else if dx.abs() == 1 && dy.abs() == 2 {
                    // horsy can jump over things
                } else {
                    info!("Move rejected as horsy must move in an L");
                    return false;
                }
            }
            PieceKind::Rook => if dx == 0 || dy == 0 {
                if piece_between(&board, (x0, y0), (x1, y1)) {
                    info!("Move rejected as there is a piece in front of the rook");
                    return false;
                }
            } else {
                info!("Move rejected as the rook must move in a straight line");
                return false;
            },
            PieceKind::Pawn => if dx == 0 {
                if (dy == 1 && colour == PieceColour::Black &&
                    board[y1 as usize][x1 as usize].is_none()) ||
                    (dy == -1 && colour == PieceColour::White &&
                        board[y1 as usize][x1 as usize].is_none())
                {
                    debug!("Pawn moving 1 square");
                // pawn just moving forwards, minding its business
                } else if (dy == 2 && colour == PieceColour::Black && y0 == 1 &&
                    !piece_between(&board, (x0, y0), (x0, y0 + 3))) ||
                    (dy == -2 && colour == PieceColour::White && y0 == 6 &&
                        !piece_between(&board, (x0, y0), (x0, y0 - 3)))
                {
                    // pawn just moving forwards - twice
                    debug!("Pawn moving 2 squares");
                } else if dx.abs() == 1 {
                    debug!("Pawn capturing");
                    match colour {
                        PieceColour::White => if dy != -1 ||
                            board[y1 as usize][x1 as usize].is_none() ||
                            board[y1 as usize][x1 as usize].unwrap().colour != PieceColour::Black
                        {
                            info!("Pawn can only move in the X direction if its capturing");
                            return false;
                        },
                        PieceColour::Black => if dy != 1 ||
                            board[y1 as usize][x1 as usize].is_none() ||
                            board[y1 as usize][x1 as usize].unwrap().colour != PieceColour::White
                        {
                            info!("Pawn can only move in the X direction if its capturing");
                            return false;
                        },
                    }
                } else {
                    info!("Pawns cannot move like that");
                    return false;
                }
            } else {
                info!("Pawns cannot move like that");
                return false;
            },
        }

        if board[y1 as usize][x1 as usize]
            .iter()
            .any(|x| x.colour == colour)
        {
            info!("Cannot take your own piece");
            return false;
        }
    } else {
        info!(
            "Cannot move piece at coordinates ({}, {}) as there is no piece there",
            x0,
            y0
        );
        return false;
    }

    true
}

fn process_move(board: &mut Board, turn: PieceColour, action: Action) -> PieceColour {
    let not_turn = match turn {
        PieceColour::White => PieceColour::Black,
        PieceColour::Black => PieceColour::White,
    };
    let Action {
        from: (x0, y0),
        to: (x1, y1),
    } = action;
    let &mut Board(ref mut inner) = board;

    if x0 >= 8 || x1 >= 8 || y0 >= 8 || y1 >= 8 {
        // das bad
        return turn;
    }

    if !process_sans_check_check(&inner, (x0, y0), (x1, y1), turn) {
        return turn;
    }

    let king_pos = inner
        .iter()
        .enumerate()
        .filter_map(|(y, &row)| {
            row.iter()
                .enumerate()
                .filter_map(|(x, piece)| {
                    piece.and_then(|Piece { kind, colour }| {
                        if kind == PieceKind::King && colour == turn {
                            Some((x as u8, y as u8))
                        } else {
                            None
                        }
                    })
                })
                .next()
        })
        .next()
        .unwrap();

    if let Some((x, y)) = (0..8).cartesian_product((0..8)).find(|&pos| {
        process_sans_check_check(&inner, pos, king_pos, not_turn)
    }) {
        info!(
            "Cannot move into check. Vulnerable from piece at ({}, {})",
            x,
            y
        );
        return turn;
    }



    inner[y1 as usize][x1 as usize] = inner[y0 as usize][x0 as usize].take();

    not_turn
}

fn main() {
    env_logger::init().unwrap();

    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        panic!("USAGE: engine configpath");
    }
    let config = Config::from_file(&args[1]);
    let (mut input, mut output) = {
        let listen = TcpListener::bind(config.host).unwrap();
        let (raw_input, _) = listen.accept().unwrap();
        (
            BufReader::new(raw_input.try_clone().unwrap()),
            BufWriter::new(raw_input.try_clone().unwrap()),
        )
    };

    let mut state = StateChange {
        board: init_board(),
        turn: PieceColour::White,
    };
    loop {
        let mut buffer = String::new();
        #[cfg(debug)]
        serde_json::to_writer_pretty(&mut output, &state).unwrap();
        #[cfg(not(debug))]
        serde_json::to_writer(&mut output, &state).unwrap();
        writeln!(&mut output, "").unwrap();
        output.flush().unwrap();

        input.read_line(&mut buffer).unwrap();
        let Vote { action, weight } = serde_json::from_str(buffer.trim_right()).unwrap();
        debug!("New move: {:?} (weight = {})", action, weight);
        if weight > 0 {
            state.turn = process_move(&mut state.board, state.turn, action);
            let mut other_board = state.board.to_owned();
            if !(0..8)
                .cartesian_product((0..8))
                .cartesian_product((0..8).cartesian_product((0..8)))
                .map(|(from, to)| Action { from, to })
                .any(|action| {
                    process_move(&mut other_board, state.turn, action) != state.turn
                }) {
                info!("Checkmate. Winner {:?}", state.turn);
                state.board = init_board();
                state.turn = PieceColour::White;
            }
        }
    }
}
