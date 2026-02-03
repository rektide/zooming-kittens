pub mod process;
pub use crate::registry::KittyRegistry;
pub mod resizer;

pub use process::find_kitty_master_pid;
pub use resizer::KittyResizer;
