// pattern: Imperative Shell

use crate::export::filter::{ExportFilter, parse_filter_expression};
use crate::export::record::ExportOutputMode;
use crate::export::serde_json::{ExportFormat, write_records};
use crate::export::snapshot::collect_snapshot_records;
use clap::Args;
use niri_ipc::{Request, Response};

#[derive(Args, Debug)]
pub struct ExportCommand {
    #[arg(
        long,
        value_enum,
        default_value = "json",
        help = "Output format: json writes one array, jsonl writes one record per line"
    )]
    pub format: ExportFormat,

    #[arg(
        long,
        value_name = "FILTERS",
        help = "Comma-separated filters like window=2,window=4,output. Omit to export all records"
    )]
    pub filter: Option<String>,

    #[arg(
        long,
        help = "For output records, replace the full modes list/current_mode index with the current mode object"
    )]
    pub current_mode: bool,
}

pub async fn run_export(command: ExportCommand) -> Result<(), Box<dyn std::error::Error>> {
    let filters = parse_filters(command.filter.as_deref())?;
    let mut socket = niri_ipc::socket::Socket::connect()?;

    let windows = match socket.send(Request::Windows)? {
        Ok(Response::Windows(windows)) => windows,
        Ok(other) => return Err(format!("unexpected windows response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request windows: {error}").into()),
    };

    let workspaces = match socket.send(Request::Workspaces)? {
        Ok(Response::Workspaces(workspaces)) => workspaces,
        Ok(other) => return Err(format!("unexpected workspaces response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request workspaces: {error}").into()),
    };

    let outputs = match socket.send(Request::Outputs)? {
        Ok(Response::Outputs(outputs)) => outputs,
        Ok(other) => return Err(format!("unexpected outputs response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request outputs: {error}").into()),
    };

    let output_mode = if command.current_mode {
        ExportOutputMode::CurrentMode
    } else {
        ExportOutputMode::Full
    };
    let records = collect_snapshot_records(windows, workspaces, outputs, &filters, output_mode);
    let stdout = std::io::stdout();
    let handle = stdout.lock();
    write_records(handle, &records, command.format)?;

    Ok(())
}

fn parse_filters(filter: Option<&str>) -> Result<Vec<ExportFilter>, Box<dyn std::error::Error>> {
    match filter {
        Some(filter) => Ok(parse_filter_expression(filter)?),
        None => Ok(Vec::new()),
    }
}
