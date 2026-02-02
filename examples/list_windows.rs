use niri_ipc::socket::Socket;
use niri_ipc::{Request, Response};

fn main() -> std::io::Result<()> {
    let mut socket = Socket::connect()?;
    
    println!("Requesting windows...");
    let reply = socket.send(Request::Windows)?;
    
    match reply {
        Ok(Response::Windows(windows)) => {
            println!("Found {} windows:", windows.len());
            for window in windows {
                println!("  ID: {}, App ID: {:?}, Title: {:?}, PID: {:?}",
                    window.id, window.app_id, window.title, window.pid);
            }
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    
    Ok(())
}
