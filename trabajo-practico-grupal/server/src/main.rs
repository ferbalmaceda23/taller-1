use model::{
    message::{Message, MessageType},
    network::Network,
    persistence::PersistenceType,
    responses::errors::ErrorResponse,
    server::Server,
    session::Session,
};
use server::{
    client_handler::handle_client,
    database::handle_database,
    load::{load_channels, load_clients, load_network_clients},
    server_errors::ServerError,
    server_handler::{handle_father_comunication, handle_server, read_from_stdin},
    socket::{read_socket, write_socket},
};
use std::{
    collections::HashMap,
    env::args,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, RwLock},
};

static MAIN_SERVER_ARGS: usize = 3;
static CHILDREN_SERVER_ARGS: usize = 6;

fn main() -> Result<(), ServerError> {
    let argv = args().collect::<Vec<String>>();
    let server;
    if argv.len() == MAIN_SERVER_ARGS {
        server =
            Server::new_main_server("0.0.0.0".to_owned(), argv[1].to_owned(), argv[2].to_owned());
    } else if argv.len() == CHILDREN_SERVER_ARGS {
        let father = Some((
            argv[3].to_owned(),
            Arc::new(TcpStream::connect(format!("{}:{}", argv[4], argv[5]))?),
        ));
        server = Server::new_child_server(
            "0.0.0.0".to_owned(),
            argv[1].to_owned(),
            argv[2].to_owned(),
            father,
        );
    } else {
        return Err(ServerError::InvalidArgs);
    }
    server_run(server)?;
    Ok(())
}

/// Function that runs the server and handles the clients/servers connections
/// # Arguments
/// * `server` - the struct of the server.
fn server_run(server: Server) -> Result<(), ServerError> {
    let address = format!("{}:{}", server.ip, server.port);
    let listener = TcpListener::bind(address.to_owned())?;
    let server_name = server.name.clone();
    println!("Listening on {}", address);

    //uncomment to test multiserver in the same repository
    // from here
    let mut hash_clients = HashMap::new();
    let mut hash_network_clients = HashMap::new();
    let mut hash_channels = HashMap::new();
    let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
    if server.father.is_none() {
        hash_clients = load_clients()?;
        hash_network_clients = load_network_clients(&hash_clients);
        hash_channels = load_channels()?;
        handle_database(db_rx);
    }
    // to here

    //  uncomment to test multiserver in different repositories
    /*
    // from here
    let hash_clients = load_clients()?;
    let hash_network_clients = load_network_clients(&hash_clients);
    let hash_channels = load_channels()?;

    let (db_tx, db_rx) = std::sync::mpsc::channel::<(PersistenceType, String)>();
    handle_database(db_rx);
    // to here
    */

    let arc_clients = Arc::new(RwLock::new(hash_clients));
    let arc_network_clients = Arc::new(RwLock::new(hash_network_clients));
    let arc_sockets = Arc::new(Mutex::new(HashMap::new()));
    let arc_channels = Arc::new(RwLock::new(hash_channels));

    let hash_servers = HashMap::<String, u8>::new();
    let arc_servers = Arc::new(RwLock::new(hash_servers));
    let arc_server = Arc::new(RwLock::new(server));

    let session = Session {
        clients: arc_clients,
        sockets: arc_sockets,
        channels: arc_channels,
        database_sender: db_tx,
    };

    let network = Network {
        server: arc_server,
        servers: arc_servers,
        clients: arc_network_clients,
    };

    let server_lock = network.server.as_ref().write()?;
    let mut servers_lock = network.servers.as_ref().write()?;

    if let Some((father_name, father_socket)) = server_lock.father.to_owned() {
        servers_lock.insert(father_name.to_owned(), 1);
        drop(servers_lock);
        drop(server_lock);
        handle_father_comunication(session.clone(), network.clone(), father_name, father_socket)?;
    } else {
        read_from_stdin(None, &session, &network);
        drop(servers_lock);
        drop(server_lock);
    }

    for stream in listener.incoming() {
        let arc_socket = Arc::new(stream?);
        let session_clone = session.clone();
        let network_clone = network.clone();
        let sn = server_name.clone();
        std::thread::spawn(move || {
            match handle_connection(arc_socket, session_clone, network_clone, &sn) {
                Ok(_) => (),
                Err(e) => println!("Error handling connection: {:?}", e),
            }
        });
    }

    Ok(())
}

/// Function that matches the message to decide if it
/// handles a server or a client connection.
/// # Arguments
/// * `arc_socket` - Reference of new connection socket.
/// * `session` - The session of the current server.
/// * `network` - The network of the current server.
/// * `server_name` - The name of the current server.
fn handle_connection(
    arc_socket: Arc<TcpStream>,
    session: Session,
    network: Network,
    server_name: &String,
) -> Result<(), ServerError> {
    let mut message_str = read_socket(arc_socket.clone())?;
    let message;
    loop {
        if let Ok(msg) = Message::serialize(message_str) {
            message = msg;
            break;
        } else {
            let response = ErrorResponse::UnknownCommand {
                command: "".to_string(), // como se el comando si me tiro error el serialize?
            }
            .to_string();
            write_socket(arc_socket.clone(), &response)?;
            message_str = read_socket(arc_socket.clone())?;
        }
    }
    if message.command == MessageType::Server {
        handle_server(arc_socket, message, session, network)?;
    } else {
        handle_client(arc_socket, message, session, network, server_name)?;
    }

    Ok(())
}
