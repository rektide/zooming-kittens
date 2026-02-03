use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};

fn main() -> std::io::Result<()> {
    let mut socket = Socket::connect()?;

    println!("Requesting event stream...");
    let reply = socket.send(Request::EventStream)?;

    if !matches!(reply, Ok(Response::Handled)) {
        eprintln!("Failed to get event stream: {:?}", reply);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get event stream",
        ));
    }

    println!("Listening for events...");
    let mut read_event = socket.read_events();

    loop {
        match read_event() {
            Ok(event) => {
                println!("Event: {:?}", event);
            }
            Err(e) => {
                eprintln!("Error reading event: {:?}", e);
                return Err(e);
            }
        }
    }
}
