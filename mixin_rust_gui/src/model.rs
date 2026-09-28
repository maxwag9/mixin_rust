use std::path::PathBuf;
use strum_macros::Display;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Mods,
    Log,
    Settings,
}

#[derive(Clone, Deserialize)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub signature: Option<ModSignature>,
    pub verification: SignatureStatus,
    pub path: PathBuf,
}

#[derive(Clone, Deserialize)]
pub struct ModSignature {
    pub public_key: String,
    pub signature: String
}

#[derive(Clone, Copy, PartialEq, Eq, Display, Debug, Deserialize)]
pub enum SignatureStatus {
    Unsigned,
    Invalid,
    Valid,
    Trusted
}


pub struct Project {
    pub path: Option<PathBuf>,
    pub name: String,
    pub version: String,
    pub cargo_manifest: Option<PathBuf>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            path: None,
            name: "No project selected".into(),
            version: "-".into(),
            cargo_manifest: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BuildState {
    Idle,
    Building,
    Succeeded,
    Failed,
}

impl BuildState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Ready",
            Self::Building => "Building",
            Self::Succeeded => "Build succeeded",
            Self::Failed => "Build failed",
        }
    }
}
