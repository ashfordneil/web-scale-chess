#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize, Debug)]
pub struct Action {
    from: (u8, u8),
    to: (u8, u8),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Vote {
    action: Action,
    weight: u32,
}
