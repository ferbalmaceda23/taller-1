/// Enum that identifies the type of operation that
/// must be done by the database.
#[derive(Debug)]
pub enum PersistenceType {
    /// inserts a new client in the database
    ClientSave,
    /// updates an existing client (identified by nickname) in the database
    ClientUpdate(String),
    /// deletes an existing client (identified by nickname) in the database
    ClientDelete(String),
    /// inserts a new channel in the database
    ChannelSave,
    /// updates an existing channel (identified by name) in the database
    ChannelUpdate(String),
    /// deletes an existing channel (identified by name) in the database
    ChannelDelete(String),
}
