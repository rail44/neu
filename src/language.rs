pub(crate) enum Language {
    Rust,
    JavaScript,
    Unknown,
}

impl Language {
    pub(crate) fn from_extension(extension: &str) -> Self {
        use Language::*;
        match extension {
            "rs" => Rust,
            "js" => JavaScript,
            _ => Unknown,
        }
    }

    pub(crate) fn from_path(path: &str) -> Self {
        let parts = path.split('.');
        Self::from_extension(parts.last().unwrap())
    }
}
