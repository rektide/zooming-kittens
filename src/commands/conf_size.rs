use crate::kitty::conf_parser;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct ConfSizeCommand {
    #[arg(short, long, help = "Path to kitty.conf file")]
    config_path: Option<String>,
}

pub fn handle_conf_size_command(cmd: ConfSizeCommand) -> std::io::Result<()> {
    let config_path = cmd.config_path.map(|p| std::path::PathBuf::from(p));

    match conf_parser::parse_font_size(config_path) {
        Ok(size) => {
            println!("{}", size);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }
}
