use crate::aircraft::Aircraft;

#[derive(Debug)]
pub enum SimMessage {
    Connecting,
    Disconnected,
    SimData(Aircraft),
    Unknown,
}

pub trait SimConnection {
    type Error;

    fn next_message(&mut self) -> Result<SimMessage, Self::Error>;
}
