// pattern: Functional Core

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportRecordKind {
    Window,
    Workspace,
    Output,
}

impl ExportRecordKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Window => "window",
            Self::Workspace => "workspace",
            Self::Output => "output",
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
            "window" | "windows" => Ok(Self::Window),
            "workspace" | "workspaces" => Ok(Self::Workspace),
            "output" | "outputs" => Ok(Self::Output),
            other => Err(FilterParseError::UnknownKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportRecordId {
    Window(u64),
    Workspace(u64),
    Output(String),
}

impl ExportRecordId {
    pub fn kind(&self) -> ExportRecordKind {
        match self {
            Self::Window(_) => ExportRecordKind::Window,
            Self::Workspace(_) => ExportRecordKind::Workspace,
            Self::Output(_) => ExportRecordKind::Output,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFilter {
    All(ExportRecordKind),
    Id(ExportRecordId),
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

    let kind = ExportRecordKind::from_str(kind.trim())?;
    let value = value.trim();
    if value.is_empty() {
        return Err(FilterParseError::InvalidId {
            kind,
            value: value.to_owned(),
        });
    }

    parse_id_filter(kind, value).map(ExportFilter::Id)
}

fn parse_id_filter(
    kind: ExportRecordKind,
    value: &str,
) -> Result<ExportRecordId, FilterParseError> {
    match kind {
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
    }
}

pub fn matches_filters(record_id: &ExportRecordId, filters: &[ExportFilter]) -> bool {
    filters.is_empty()
        || filters.iter().any(|filter| match filter {
            ExportFilter::All(kind) => record_id.kind() == *kind,
            ExportFilter::Id(id) => record_id == id,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valueless_filters_as_wildcards() {
        let filters = parse_filter_expression("window,output").expect("filters parse");

        assert_eq!(
            filters,
            vec![
                ExportFilter::All(ExportRecordKind::Window),
                ExportFilter::All(ExportRecordKind::Output),
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
    fn matches_any_filter() {
        let filters = parse_filter_expression("window=2,workspace").expect("filters parse");

        assert!(matches_filters(&ExportRecordId::Window(2), &filters));
        assert!(!matches_filters(&ExportRecordId::Window(3), &filters));
        assert!(matches_filters(&ExportRecordId::Workspace(9), &filters));
    }

    #[test]
    fn empty_filters_match_everything() {
        assert!(matches_filters(&ExportRecordId::Window(1), &[]));
    }
}
