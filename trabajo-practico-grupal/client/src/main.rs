use client::run::client_run;
use client::run_interface::client_run_interface;
use model::client_errors::ClientError;
use std::env::args;

static CLIENT_ARGS: usize = 3;

fn main() -> Result<(), ClientError> {
    let argv = args().collect::<Vec<String>>();
    if argv.len() == CLIENT_ARGS {
        let address = argv[1].clone() + ":" + &argv[2];
        println!("Connecting to {address:?}");
        return client_run(&address);
    }

    client_run_interface();

    Ok(())
}
