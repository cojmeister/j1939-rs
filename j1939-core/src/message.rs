/// J1939 CAN message structure
#[derive(Debug, Clone, Copy)]
pub struct J1939Message {
    pub priority: u8,
    pub pgn: u32,
    pub source_address: u8,
    pub data: [u8; 8],
    pub length: u8,
}

impl Default for J1939Message {
    fn default() -> Self {
        Self {
            priority: 6,
            pgn: 0,
            source_address: 0,
            data: [0; 8],
            length: 0,
        }
    }
}

/// Trait for marshalling data into J1939 messages
pub trait Marshall {
    fn marshall(&self, msg: &mut J1939Message) -> crate::Result<()>;
}

/// Trait for unmarshalling J1939 messages into data
pub trait Unmarshall: Sized {
    fn unmarshall(msg: &J1939Message) -> crate::Result<Self>;
}