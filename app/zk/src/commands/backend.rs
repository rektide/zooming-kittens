use clap::ValueEnum;

/// Native observation source, shared by the independent desktop consumers.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Backend {
    Niri,
    Umbriel,
}

impl Backend {
    pub fn require(self) -> std::io::Result<()> {
        match self {
            Self::Niri => crate::require_feature(cfg!(feature = "niri"), "niri"),
            Self::Umbriel => crate::require_feature(cfg!(feature = "umbriel"), "umbriel"),
        }
    }
}
