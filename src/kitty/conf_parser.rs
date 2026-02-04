use std::fs;
use std::path::PathBuf;

const KITTY_CONF_NAME: &str = "kitty.conf";

pub fn get_kitty_config_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Config directory not found".to_string())?
        .join("kitty");

    let conf_path = config_dir.join(KITTY_CONF_NAME);

    if !conf_path.exists() {
        return Err(format!(
            "kitty.conf not found at {}. Please create one.",
            conf_path.display()
        ));
    }

    Ok(conf_path)
}

pub fn parse_font_size(config_path: Option<PathBuf>) -> Result<f64, String> {
    let conf_path = config_path
        .or_else(|| get_kitty_config_path().ok())
        .ok_or_else(|| "No config path provided and could not find default".to_string())?;

    let content =
        fs::read_to_string(&conf_path).map_err(|e| format!("Failed to read config file: {}", e))?;

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("font_size") {
            let rest = rest.trim();
            if rest.is_empty() {
                return Err("font_size found but has no value".to_string());
            }

            return rest
                .parse::<f64>()
                .map_err(|e| format!("Failed to parse font_size value '{}': {}", rest, e));
        }
    }

    Err("font_size not found in kitty.conf".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_font_size_valid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "font_size 12.5").unwrap();
        writeln!(temp_file, "other_config value").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12.5);
    }

    #[test]
    fn test_parse_font_size_with_comments() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "font_size 14.0").unwrap();
        writeln!(temp_file, "# Another comment").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 14.0);
    }

    #[test]
    fn test_parse_font_size_integer() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "font_size 12").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12.0);
    }

    #[test]
    fn test_parse_font_size_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "other_config value").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_parse_font_size_invalid() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "font_size invalid").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn test_parse_font_size_empty() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "font_size").unwrap();

        let result = parse_font_size(Some(temp_file.path().to_path_buf()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("has no value"));
    }
}
