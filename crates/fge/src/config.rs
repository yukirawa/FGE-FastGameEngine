use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Top-level `game.toml` manifest definition (`PROJECT.md` Section 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManifest {
    pub game: GameSection,
    #[serde(default)]
    pub build: BuildSection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSection {
    pub title: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,
    #[serde(default)]
    pub vsync: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildSection {
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            sources: Vec::new(),
        }
    }
}

const fn default_width() -> u32 {
    1280
}

const fn default_height() -> u32 {
    720
}

const fn default_target_fps() -> u32 {
    60
}

fn default_entry() -> String {
    String::from("src/main.c")
}

impl GameManifest {
    /// Validates manifest invariants (`!title.trim().is_empty()`, `width > 0`, `height > 0`, `target_fps > 0`, `!entry.trim().is_empty()`).
    pub fn validate(&self) -> Result<(), AppError> {
        todo!("GameManifest::validate")
    }
}

/// Parses and validates a `game.toml` string.
pub fn parse_manifest_str(_toml_str: &str) -> Result<GameManifest, AppError> {
    todo!("parse_manifest_str")
}

/// Loads `game.toml` from `path` (if `path` is a directory, reads `path.join("game.toml")`).
pub fn load_manifest_from_path(_path: &Path) -> Result<GameManifest, AppError> {
    todo!("load_manifest_from_path")
}
