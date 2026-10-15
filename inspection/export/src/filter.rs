// pattern: Functional Core

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportRecordKind {
    Version,
    Window,
    Workspace,
    Output,
    Layer,
    Cast,
    KeyboardLayouts,
    Overview,
}

impl ExportRecordKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Version => "version",
            Self::Window => "window",
            Self::Workspace => "workspace",
            Self::Output => "output",
            Self::Layer => "layer",
            Self::Cast => "cast",
            Self::KeyboardLayouts => "keyboard_layouts",
            Self::Overview => "overview",
        }
    }
}

impl fmt::Display for ExportRecordKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ExportRecordKind {
    type Err = FilterParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "version" => Ok(Self::Version),
            "window" | "windows" => Ok(Self::Window),
            "workspace" | "workspaces" => Ok(Self::Workspace),
            "output" | "outputs" => Ok(Self::Output),
            "layer" | "layers" => Ok(Self::Layer),
            "cast" | "casts" => Ok(Self::Cast),
            "keyboard_layouts" | "keyboard-layouts" => Ok(Self::KeyboardLayouts),
            "overview" | "overview_state" | "overview-state" => Ok(Self::Overview),
            other => Err(FilterParseError::UnknownKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportRecordId {
    Version,
    Window(u64),
    Workspace(u64),
    Output(String),
    Layer(String),
    Cast(u64),
    KeyboardLayouts,
    Overview,
}

impl ExportRecordId {
    pub fn kind(&self) -> ExportRecordKind {
        match self {
            Self::Version => ExportRecordKind::Version,
            Self::Window(_) => ExportRecordKind::Window,
            Self::Workspace(_) => ExportRecordKind::Workspace,
            Self::Output(_) => ExportRecordKind::Output,
            Self::Layer(_) => ExportRecordKind::Layer,
            Self::Cast(_) => ExportRecordKind::Cast,
            Self::KeyboardLayouts => ExportRecordKind::KeyboardLayouts,
            Self::Overview => ExportRecordKind::Overview,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFilter {
    All(ExportRecordKind),
    Id(ExportRecordId),
    Type(String),
}

pub struct ExportRecordMatch<'a> {
    pub record_type: &'a str,
    pub ids: &'a [ExportRecordId],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterParseError {
    UnknownKind(String),
    InvalidId {
        kind: ExportRecordKind,
        value: String,
    },
}

impl fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKind(kind) => write!(f, "unknown export filter kind: {kind}"),
            Self::InvalidId { kind, value } => {
                write!(f, "invalid {kind} export filter id: {value}")
            }
        }
    }
}

impl std::error::Error for FilterParseError {}

pub fn parse_filter_expression(expression: &str) -> Result<Vec<ExportFilter>, FilterParseError> {
    // Future direction: filters should support nested predicates such as
    // `window.app_id=kitty`, `window.pid=1234`, or `output.logical.width=1920`.
    // A filter is expected to match zero or more records, not exactly one record.
    // Keep exact-id filters as one predicate kind rather than baking that
    // assumption into export traversal.
    expression
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_filter_part)
        .collect()
}

fn parse_filter_part(part: &str) -> Result<ExportFilter, FilterParseError> {
    let Some((kind, value)) = part.split_once('=') else {
        return Ok(ExportFilter::All(ExportRecordKind::from_str(part)?));
    };

    let kind = kind.trim();
    let value = value.trim();
    if value.is_empty() {
        let kind = ExportRecordKind::from_str(kind)?;
        return Err(FilterParseError::InvalidId {
            kind,
            value: value.to_owned(),
        });
    }

    if kind == "type" {
        return Ok(ExportFilter::Type(value.to_owned()));
    }

    let kind = ExportRecordKind::from_str(kind)?;
    parse_id_filter(kind, value).map(ExportFilter::Id)
}

fn parse_id_filter(
    kind: ExportRecordKind,
    value: &str,
) -> Result<ExportRecordId, FilterParseError> {
    match kind {
        ExportRecordKind::Version
        | ExportRecordKind::KeyboardLayouts
        | ExportRecordKind::Overview => Err(FilterParseError::InvalidId {
            kind,
            value: value.to_owned(),
        }),
        ExportRecordKind::Window => value
            .parse::<u64>()
            .map(ExportRecordId::Window)
            .map_err(|_| FilterParseError::InvalidId {
                kind,
                value: value.to_owned(),
            }),
        ExportRecordKind::Workspace => value
            .parse::<u64>()
            .map(ExportRecordId::Workspace)
            .map_err(|_| FilterParseError::InvalidId {
                kind,
                value: value.to_owned(),
            }),
        ExportRecordKind::Output => Ok(ExportRecordId::Output(value.to_owned())),
        ExportRecordKind::Layer => Ok(ExportRecordId::Layer(value.to_owned())),
        ExportRecordKind::Cast => value
            .parse::<u64>()
            .map(ExportRecordId::Cast)
            .map_err(|_| FilterParseError::InvalidId {
                kind,
                value: value.to_owned(),
            }),
    }
}

pub fn matches_record(record: ExportRecordMatch<'_>, filters: &[ExportFilter]) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| match filter {
            ExportFilter::All(kind) => record.ids.iter().any(|id| id.kind() == *kind),
            ExportFilter::Id(id) => record.ids.iter().any(|record_id| record_id == id),
            ExportFilter::Type(record_type) => record.record_type == record_type,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valueless_filters_as_wildcards() {
        let filters = parse_filter_expression("version,window,output,layer,keyboard_layouts,overview")
            .expect("filters parse");

        assert_eq!(
            filters,
            vec![
                ExportFilter::All(ExportRecordKind::Version),
                ExportFilter::All(ExportRecordKind::Window),
                ExportFilter::All(ExportRecordKind::Output),
                ExportFilter::All(ExportRecordKind::Layer),
                ExportFilter::All(ExportRecordKind::KeyboardLayouts),
                ExportFilter::All(ExportRecordKind::Overview),
            ]
        );
    }

    #[test]
    fn parses_id_filters() {
        let filters = parse_filter_expression("window=2,window=4,workspace=3,output=eDP-1")
            .expect("filters parse");

        assert_eq!(
            filters,
            vec![
                ExportFilter::Id(ExportRecordId::Window(2)),
                ExportFilter::Id(ExportRecordId::Window(4)),
                ExportFilter::Id(ExportRecordId::Workspace(3)),
                ExportFilter::Id(ExportRecordId::Output(String::from("eDP-1"))),
            ]
        );
    }

    #[test]
    fn parses_type_filter() {
        let filters = parse_filter_expression("type=window_layouts_changed").expect("filters parse");

        assert_eq!(
            filters,
            vec![ExportFilter::Type(String::from("window_layouts_changed"))]
        );
    }

    #[test]
    fn rejects_unknown_filter_kind() {
        let error = parse_filter_expression("monitor").expect_err("unknown kind fails");

        assert_eq!(error, FilterParseError::UnknownKind(String::from("monitor")));
    }

    #[test]
    fn rejects_invalid_numeric_id() {
        let error = parse_filter_expression("window=nope").expect_err("bad id fails");

        assert_eq!(
            error,
            FilterParseError::InvalidId {
                kind: ExportRecordKind::Window,
                value: String::from("nope"),
            }
        );
    }

    #[test]
    fn type_filter_matches_record_type() {
        let filters = parse_filter_expression("type=window_closed").expect("filters parse");
        let ids = vec![ExportRecordId::Window(4)];

        assert!(matches_record(
            ExportRecordMatch {
                record_type: "window_closed",
                ids: &ids,
            },
            &filters
        ));
        assert!(!matches_record(
            ExportRecordMatch {
                record_type: "window_focus_changed",
                ids: &ids,
            },
            &filters
        ));
    }

    #[test]
    fn id_and_kind_filters_match_record_ids() {
        let filters = parse_filter_expression("window=2,workspace").expect("filters parse");
        let window_ids = vec![ExportRecordId::Window(2)];
        let other_window_ids = vec![ExportRecordId::Window(3)];
        let workspace_ids = vec![ExportRecordId::Workspace(9)];

        assert!(matches_record(
            ExportRecordMatch {
                record_type: "window_closed",
                ids: &window_ids,
            },
            &filters
        ));
        assert!(!matches_record(
            ExportRecordMatch {
                record_type: "window_closed",
                ids: &other_window_ids,
            },
            &filters
        ));
        assert!(matches_record(
            ExportRecordMatch {
                record_type: "workspace_activated",
                ids: &workspace_ids,
            },
            &filters
        ));
    }

    #[test]
    fn empty_filters_match_everything() {
        assert!(matches_record(
            ExportRecordMatch {
                record_type: "window",
                ids: &[ExportRecordId::Window(1)],
            },
            &[]
        ));
    }
}
