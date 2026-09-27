use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Mods,
    Build,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModKind {
    Hard,
    Soft,
    Script,
}

impl ModKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hard => "Hard Mod",
            Self::Soft => "Soft Mod",
            Self::Script => "Script",
        }
    }
}

#[derive(Clone)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub kind: ModKind,
    pub enabled: bool,
    pub trusted: bool,
}

impl ModEntry {
    pub fn demo_data() -> Vec<Self> {
        vec![
            Self {
                id: "better_traffic".into(),
                name: "Better Traffic".into(),
                version: "0.3.0".into(),
                author: "Example Author".into(),
                description: "Replaces parts of traffic simulation with a custom implementation.".into(),
                kind: ModKind::Hard,
                enabled: true,
                trusted: true,
            },
            Self {
                id: "underground_rail".into(),
                name: "Underground Rail".into(),
                version: "1.1.2".into(),
                author: "Example Author".into(),
                description: "Adds a new underground railway simulation mixin.".into(),
                kind: ModKind::Hard,
                enabled: true,
                trusted: true,
            },
            Self {
                id: "debug_tools".into(),
                name: "Debug Tools".into(),
                version: "0.5.1".into(),
                author: "Mixin Rust Team".into(),
                description: "Development helpers exposed as a normal runtime mod.".into(),
                kind: ModKind::Soft,
                enabled: false,
                trusted: true,
            },
            Self {
                id: "weather_experiment".into(),
                name: "Extreme Weather".into(),
                version: "0.1.0".into(),
                author: "Random Modder".into(),
                description: "Experimental weather hooks for testing the mod pipeline.".into(),
                kind: ModKind::Hard,
                enabled: false,
                trusted: false,
            },
        ]
    }
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
