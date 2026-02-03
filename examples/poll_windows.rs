use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};
use std::{thread, time::Duration};

fn main() -> std::io::Result<()> {
    println!("Starting to poll windows every 2 seconds...");

    loop {
        let mut socket = Socket::connect()?;
        let reply = socket.send(Request::Windows)?;

        match reply {
            Ok(Response::Windows(windows)) => {
                let focused = windows.iter().find(|w| w.is_focused);
                if let Some(fw) = focused {
                    println!(
                        "Focused: ID={}, App={:?}, Title={:?}, PID={:?}",
                        fw.id, fw.app_id, fw.title, fw.pid
                    );
                } else {
                    println!("No window focused");
                }
            }
            other => {
                println!("Unexpected response: {:?}", other);
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}
