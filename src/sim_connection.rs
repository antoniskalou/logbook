use crate::aircraft::Aircraft;

#[derive(Debug)]
pub enum SimMessage {
    Open,
    Quit,
    SimData(Aircraft),
    Waiting,
    Unknown,
}

pub trait SimConnection {
    type Error;

    fn next_message(&mut self) -> Result<SimMessage, Self::Error>;
}
