use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoomType {
    Absolute,
    Additive,
    Multiplicative,
}

/// Plain policy values. Loading files, environment and CLI overrides belongs to
/// the application, not this library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZoomConfig {
    pub absolute: Option<f64>,
    pub additive: Option<f64>,
    pub multiplicative: Option<f64>,
    pub step_size: f64,
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            absolute: None,
            additive: None,
            multiplicative: None,
            step_size: 1.0,
        }
    }
}

impl ZoomConfig {
    pub fn validate(&self) -> Result<(), String> {
        let mut types = Vec::new();
        if self.absolute.is_some() {
            types.push("absolute");
        }
        if self.additive.is_some() {
            types.push("additive");
        }
        if self.multiplicative.is_some() {
            types.push("multiplicative");
        }
        if types.len() > 1 {
            return Err(format!(
                "Multiple zoom types configured: {}. Only one zoom type may be set.",
                types.join(", ")
            ));
        }
        Ok(())
    }

    pub fn active_type(&self) -> Option<ZoomType> {
        if self.absolute.is_some() {
            Some(ZoomType::Absolute)
        } else if self.additive.is_some() {
            Some(ZoomType::Additive)
        } else if self.multiplicative.is_some() {
            Some(ZoomType::Multiplicative)
        } else {
            None
        }
    }

    pub fn value(&self) -> Option<f64> {
        match self.active_type() {
            Some(ZoomType::Absolute) => self.absolute,
            Some(ZoomType::Additive) => self.additive,
            Some(ZoomType::Multiplicative) => self.multiplicative,
            None => None,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.active_type().is_some()
    }
}
