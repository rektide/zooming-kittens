// pattern: Imperative Shell

use clap::ValueEnum;
use serde::Serialize;
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    Json,
    Jsonl,
}

pub fn write_records<W, T>(
    mut writer: W,
    records: &[T],
    format: ExportFormat,
) -> Result<(), serde_json::Error>
where
    W: Write,
    T: Serialize,
{
    match format {
        ExportFormat::Json => {
            serde_json::to_writer(&mut writer, records)?;
            writer.write_all(b"\n").map_err(serde_json::Error::io)
        }
        ExportFormat::Jsonl => {
            for record in records {
                serde_json::to_writer(&mut writer, record)?;
                writer.write_all(b"\n").map_err(serde_json::Error::io)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_writes_one_array_document() {
        let records = vec![json!({ "type": "window", "id": 1 }), json!({ "type": "output", "id": "eDP-1" })];
        let mut output = Vec::new();

        write_records(&mut output, &records, ExportFormat::Json).expect("writes json");

        assert_eq!(
            String::from_utf8(output).expect("utf8"),
            "[{\"id\":1,\"type\":\"window\"},{\"id\":\"eDP-1\",\"type\":\"output\"}]\n"
        );
    }

    #[test]
    fn jsonl_writes_one_record_per_line() {
        let records = vec![json!({ "type": "window", "id": 1 }), json!({ "type": "output", "id": "eDP-1" })];
        let mut output = Vec::new();

        write_records(&mut output, &records, ExportFormat::Jsonl).expect("writes jsonl");

        assert_eq!(
            String::from_utf8(output).expect("utf8"),
            "{\"id\":1,\"type\":\"window\"}\n{\"id\":\"eDP-1\",\"type\":\"output\"}\n"
        );
    }
}
