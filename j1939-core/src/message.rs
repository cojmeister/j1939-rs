/// A J1939 CAN message containing header information and payload data.
///
/// This structure represents a complete J1939 message with all necessary
/// fields for communication on a J1939 network. J1939 is a vehicle bus
/// standard used in commercial vehicles and heavy-duty equipment.
///
/// # Fields
///
/// * `priority` - Message priority (0-7, where 0 is highest priority)
/// * `pgn` - Parameter Group Number identifying the message type
/// * `source_address` - Address of the sending device (0-253)
/// * `data` - Message payload data (up to 223 bytes for multi-packet messages)
/// * `length` - Actual number of bytes used in the data field (0-223)
///
/// # Examples
///
/// ```
/// use j1939_core::message::J1939Message;
///
/// // Create a new message using default
/// let mut msg = J1939Message::default();
/// msg.priority = 3;
/// msg.pgn = 0xF004; // Engine temperature
/// msg.source_address = 0x80; // Engine ECU
/// msg.data[0] = 75; // Temperature value
/// msg.length = 1;
///
/// // Create a message with specific data
/// let mut msg = J1939Message::default();
/// msg.priority = 6;
/// msg.pgn = 0xFECA;
/// msg.source_address = 0x17;
/// msg.data[0] = 0x12;
/// msg.data[1] = 0x34;
/// msg.data[2] = 0x56;
/// msg.data[3] = 0x78;
/// msg.length = 4;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct J1939Message {
    pub priority: u8,
    pub pgn: u32,
    pub source_address: u8,
    pub data: [u8; 223],
    pub length: u8,
}

impl Default for J1939Message {
    /// Creates a default J1939 message with standard values.
    ///
    /// The default message has:
    /// - Priority: 6 (normal priority)
    /// - PGN: 0 (undefined)
    /// - Source address: 0 (null address)
    /// - Data: all zeros
    /// - Length: 0 (no data)
    ///
    /// # Examples
    ///
    /// ```
    /// use j1939_core::message::J1939Message;
    ///
    /// let msg = J1939Message::default();
    /// assert_eq!(msg.priority, 6);
    /// assert_eq!(msg.pgn, 0);
    /// assert_eq!(msg.source_address, 0);
    /// assert_eq!(msg.length, 0);
    /// ```
    fn default() -> Self {
        Self {
            priority: 6,
            pgn: 0,
            source_address: 0,
            data: [0; 223],
            length: 0,
        }
    }
}

/// Trait for encoding structured data into J1939 message format.
///
/// Implement this trait to convert your data structures into J1939 messages
/// that can be transmitted over the network. The trait provides a standard
/// interface for serializing data into the message payload.
///
/// # Examples
///
/// ```
/// use j1939_core::message::{J1939Message, Marshall};
///
/// struct EngineData {
///     temperature: u8,
///     rpm: u16,
/// }
///
/// impl Marshall for EngineData {
///     fn marshall(&self, msg: &mut J1939Message) -> j1939_core::Result<()> {
///         msg.pgn = 0xF004; // Engine temperature PGN
///         msg.data[0] = self.temperature;
///         msg.data[1] = (self.rpm & 0xFF) as u8;
///         msg.data[2] = ((self.rpm >> 8) & 0xFF) as u8;
///         msg.length = 3;
///         Ok(())
///     }
/// }
/// ```
pub trait Marshall {
    /// Encodes this data structure into a J1939 message.
    ///
    /// # Arguments
    ///
    /// * `msg` - Mutable reference to the message to populate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if marshalling fails.
    fn marshall(&self, msg: &mut J1939Message) -> crate::Result<()>;
}

/// Trait for decoding J1939 messages into structured data.
///
/// Implement this trait to convert received J1939 messages back into your
/// data structures. The trait provides a standard interface for deserializing
/// message payloads into typed data.
///
/// # Examples
///
/// ```
/// use j1939_core::message::{J1939Message, Unmarshall};
///
/// struct EngineData {
///     temperature: u8,
///     rpm: u16,
/// }
///
/// impl Unmarshall for EngineData {
///     fn unmarshall(msg: &J1939Message) -> j1939_core::Result<Self> {
///         if msg.length < 3 {
///             return Err(j1939_core::Error::InvalidLength);
///         }
///
///         Ok(EngineData {
///             temperature: msg.data[0],
///             rpm: msg.data[1] as u16 | ((msg.data[2] as u16) << 8),
///         })
///     }
/// }
/// ```
pub trait Unmarshall: Sized {
    /// Decodes a J1939 message into this data structure.
    ///
    /// # Arguments
    ///
    /// * `msg` - Reference to the message to decode
    ///
    /// # Returns
    ///
    /// Returns the decoded data structure on success, or an error if
    /// unmarshalling fails (e.g., insufficient data, invalid format).
    fn unmarshall(msg: &J1939Message) -> crate::Result<Self>;
}