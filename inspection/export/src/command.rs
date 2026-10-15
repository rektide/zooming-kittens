// pattern: Imperative Shell

use crate::filter::{ExportFilter, parse_filter_expression};
use crate::record::ExportOutputMode;
use crate::serde_json::{ExportFormat, write_records};
use crate::snapshot::collect_snapshot_records;
use crate::stream::{event_to_record, filter_stream_record};
use clap::Args;
use niri_ipc::{Request, Response};

#[derive(Args, Debug)]
pub struct ExportCommand {
    #[arg(
        long,
        value_enum,
        help = "Output format: json writes one array, jsonl writes one record per line"
    )]
    pub format: Option<ExportFormat>,

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

    #[arg(long, help = "Emit current niri snapshot records")]
    pub snapshot: bool,

    #[arg(long, help = "Emit niri event stream records")]
    pub stream: bool,

    #[arg(
        long,
        default_value = "1",
        help = "First synthetic stream_event_id for stream records"
    )]
    pub starting_id: u64,

    #[arg(long, help = "Stop after this many emitted stream records")]
    pub limit: Option<usize>,
}

pub async fn run_export(command: ExportCommand) -> Result<(), Box<dyn std::error::Error>> {
    let filters = parse_filters(command.filter.as_deref())?;
    let no_mode_selected = !command.snapshot && !command.stream;
    let export_snapshot = command.snapshot || no_mode_selected;
    let export_stream = command.stream || no_mode_selected;
    let format = resolve_format(command.format, export_stream);

    if export_stream && format == ExportFormat::Json && command.limit.is_none() {
        return Err("export --stream --format json requires --limit; use --format jsonl for unbounded streams".into());
    }

    let output_mode = if command.current_mode {
        ExportOutputMode::CurrentMode
    } else {
        ExportOutputMode::Full
    };

    if format == ExportFormat::Json {
        let mut records = Vec::new();
        if export_snapshot {
            records.extend(read_snapshot_records(&filters, output_mode)?);
        }
        if export_stream {
            records.extend(read_stream_records(
                &filters,
                command.starting_id,
                command.limit,
            )?);
        }
        let stdout = std::io::stdout();
        let handle = stdout.lock();
        write_records(handle, &records, format)?;
        return Ok(());
    }

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    if export_snapshot {
        let records = read_snapshot_records(&filters, output_mode)?;
        write_records(&mut handle, &records, format)?;
    }

    if export_stream {
        write_stream_records(&mut handle, &filters, command.starting_id, command.limit)?;
    }

    Ok(())
}

fn read_snapshot_records(
    filters: &[ExportFilter],
    output_mode: ExportOutputMode,
) -> Result<Vec<crate::record::ExportRecord>, Box<dyn std::error::Error>> {
    let mut socket = niri_ipc::socket::Socket::connect()?;

    let version = match socket.send(Request::Version)? {
        Ok(Response::Version(version)) => version,
        Ok(other) => return Err(format!("unexpected version response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request version: {error}").into()),
    };

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

    let layers = match socket.send(Request::Layers)? {
        Ok(Response::Layers(layers)) => layers,
        Ok(other) => return Err(format!("unexpected layers response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request layers: {error}").into()),
    };

    let casts = match socket.send(Request::Casts)? {
        Ok(Response::Casts(casts)) => casts,
        Ok(other) => return Err(format!("unexpected casts response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request casts: {error}").into()),
    };

    let keyboard_layouts = match socket.send(Request::KeyboardLayouts)? {
        Ok(Response::KeyboardLayouts(keyboard_layouts)) => keyboard_layouts,
        Ok(other) => return Err(format!("unexpected keyboard layouts response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request keyboard layouts: {error}").into()),
    };

    let overview = match socket.send(Request::OverviewState)? {
        Ok(Response::OverviewState(overview)) => overview,
        Ok(other) => return Err(format!("unexpected overview response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request overview: {error}").into()),
    };

    Ok(collect_snapshot_records(
        version,
        windows,
        workspaces,
        outputs,
        layers,
        casts,
        keyboard_layouts,
        overview,
        filters,
        output_mode,
    ))
}

fn parse_filters(filter: Option<&str>) -> Result<Vec<ExportFilter>, Box<dyn std::error::Error>> {
    match filter {
        Some(filter) => Ok(parse_filter_expression(filter)?),
        None => Ok(Vec::new()),
    }
}

fn resolve_format(format: Option<ExportFormat>, stream: bool) -> ExportFormat {
    match format {
        Some(format) => format,
        None if stream => ExportFormat::Jsonl,
        None => ExportFormat::Json,
    }
}

fn read_stream_records(
    filters: &[ExportFilter],
    starting_id: u64,
    limit: Option<usize>,
) -> Result<Vec<crate::record::ExportRecord>, Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    read_stream(filters, starting_id, limit, |record| {
        records.push(record);
        Ok(())
    })?;
    Ok(records)
}

fn write_stream_records<W: std::io::Write>(
    writer: &mut W,
    filters: &[ExportFilter],
    starting_id: u64,
    limit: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    read_stream(filters, starting_id, limit, |record| {
        write_records(&mut *writer, &[record], ExportFormat::Jsonl)?;
        Ok(())
    })
}

fn read_stream<F>(
    filters: &[ExportFilter],
    starting_id: u64,
    limit: Option<usize>,
    mut emit: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(crate::record::ExportRecord) -> Result<(), Box<dyn std::error::Error>>,
{
    let mut socket = niri_ipc::socket::Socket::connect()?;
    match socket.send(Request::EventStream)? {
        Ok(Response::Handled) => {}
        Ok(other) => return Err(format!("unexpected event stream response: {other:?}").into()),
        Err(error) => return Err(format!("failed to request event stream: {error}").into()),
    }

    let mut read_event = socket.read_events();
    let mut stream_event_id = starting_id;
    let mut emitted = 0usize;

    while limit.map(|limit| emitted < limit).unwrap_or(true) {
        let event = read_event()?;
        let record = event_to_record(event, stream_event_id);
        stream_event_id = stream_event_id.saturating_add(1);
        if let Some(record) = filter_stream_record(record, filters) {
            emit(record)?;
            emitted += 1;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        command: ExportCommand,
    }

    #[test]
    fn default_command_preserves_combined_snapshot_stream_defaults() {
        let command = Cli::try_parse_from(["export"]).unwrap().command;
        assert!(!command.snapshot);
        assert!(!command.stream);
        assert_eq!(command.starting_id, 1);
        assert_eq!(command.limit, None);
        assert_eq!(command.filter, None);
        assert_eq!(resolve_format(command.format, true), ExportFormat::Jsonl);
    }

    #[test]
    fn snapshot_defaults_to_json() {
        let command = Cli::try_parse_from(["export", "--snapshot"])
            .unwrap()
            .command;
        assert!(command.snapshot);
        assert!(!command.stream);
        assert_eq!(
            resolve_format(command.format, command.stream),
            ExportFormat::Json
        );
    }

    #[test]
    fn explicit_stream_options_preserve_native_ids_and_filters() {
        let command = Cli::try_parse_from([
            "export",
            "--stream",
            "--format",
            "json",
            "--starting-id",
            "42",
            "--limit",
            "3",
            "--filter",
            "window=2,output",
            "--current-mode",
        ])
        .unwrap()
        .command;
        assert!(command.stream);
        assert!(command.current_mode);
        assert_eq!(command.starting_id, 42);
        assert_eq!(command.limit, Some(3));
        assert_eq!(resolve_format(command.format, true), ExportFormat::Json);
        assert_eq!(parse_filters(command.filter.as_deref()).unwrap().len(), 2);
    }
}
