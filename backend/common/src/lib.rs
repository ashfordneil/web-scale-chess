#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    pub from: (u8, u8),
    pub to: (u8, u8),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Vote {
    action: Action,
    weight: u32,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum PieceKind {
    King,
    Queen,
    Bishop,
    Knight,
    Rook,
    Pawn
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum PieceColour {
    White,
    Black
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Piece {
    pub kind: PieceKind,
    pub colour: PieceColour
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Board(pub [[Option<Piece>; 8]; 8]);


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateChange {
    pub board: Board,
    pub turn: PieceColour
}
