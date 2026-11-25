use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Top-level `game.toml` manifest definition (`PROJECT.md` Section 4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameManifest {
    pub game: GameSection,
    #[serde(default)]
    pub build: BuildSection,
    #[serde(default)]
    pub spatial: SpatialSection,
    #[serde(default)]
    pub render: RenderSection,
    #[serde(default)]
    pub audio: AudioSection,
    #[serde(default)]
    pub ui: UiSection,
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
    #[serde(default)]
    pub compiler: Option<String>,
}

impl Default for BuildSection {
    fn default() -> Self {
        Self {
            entry: default_entry(),
            sources: Vec::new(),
            compiler: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialSection {
    #[serde(default = "default_cell_size")]
    pub cell_size: f32,
    #[serde(default = "default_buckets")]
    pub buckets: usize,
}

impl Default for SpatialSection {
    fn default() -> Self {
        Self {
            cell_size: default_cell_size(),
            buckets: default_buckets(),
        }
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YSortModeConfig {
    #[default]
    Fast,
    Composite,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderSection {
    #[serde(default)]
    pub y_sort_mode: YSortModeConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSection {
    #[serde(default = "default_master_volume")]
    pub master_volume: f32,
    #[serde(default)]
    pub muted: bool,
}

impl Default for AudioSection {
    fn default() -> Self {
        Self {
            master_volume: default_master_volume(),
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSection {
    #[serde(default)]
    pub font: Option<String>,
}

const fn default_master_volume() -> f32 {
    0.8
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

const fn default_cell_size() -> f32 {
    64.0
}

const fn default_buckets() -> usize {
    4096
}

impl GameManifest {
    /// Validates manifest invariants (`!title.trim().is_empty()`, `width > 0`, `height > 0`, `target_fps > 0`, `!entry.trim().is_empty()`, valid `build.compiler`, valid `spatial` config).
    pub fn validate(&self) -> Result<(), AppError> {
        if self.game.title.trim().is_empty() {
            return Err(AppError::InvalidManifest {
                reason: String::from("game.title must not be empty"),
            });
        }
        if self.game.width == 0 {
            return Err(AppError::InvalidManifest {
                reason: String::from("game.width must be greater than 0"),
            });
        }
        if self.game.height == 0 {
            return Err(AppError::InvalidManifest {
                reason: String::from("game.height must be greater than 0"),
            });
        }
        if self.game.target_fps == 0 {
            return Err(AppError::InvalidManifest {
                reason: String::from("game.target_fps must be greater than 0"),
            });
        }
        if self.build.entry.trim().is_empty() {
            return Err(AppError::InvalidManifest {
                reason: String::from("build.entry must not be empty"),
            });
        }
        if let Some(ref c) = self.build.compiler {
            let normalized = c.trim().to_ascii_lowercase();
            if !matches!(normalized.as_str(), "gcc" | "clang" | "tcc") {
                return Err(AppError::InvalidManifest {
                    reason: format!(
                        "build.compiler must be one of \"gcc\", \"clang\", or \"tcc\" (got {:?})",
                        c
                    ),
                });
            }
        }
        if !(self.spatial.cell_size.is_finite() && self.spatial.cell_size > 0.0) {
            return Err(AppError::InvalidManifest {
                reason: String::from("spatial.cell_size must be finite and greater than 0.0"),
            });
        }
        if !(self.spatial.buckets > 0 && self.spatial.buckets.is_power_of_two()) {
            return Err(AppError::InvalidManifest {
                reason: String::from("spatial.buckets must be a positive power of two"),
            });
        }
        if !(self.audio.master_volume.is_finite() && self.audio.master_volume >= 0.0) {
            return Err(AppError::InvalidManifest {
                reason: String::from("audio.master_volume must be finite and non-negative"),
            });
        }
        if let Some(ref font) = self.ui.font {
            if font.trim().is_empty() {
                return Err(AppError::InvalidManifest {
                    reason: String::from("ui.font must not be empty when specified"),
                });
            }
        }
        Ok(())
    }
}

/// Parses and validates a `game.toml` string.
pub fn parse_manifest_str(toml_str: &str) -> Result<GameManifest, AppError> {
    let manifest: GameManifest = toml::from_str(toml_str)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Loads `game.toml` from `path` (if `path` is a directory, reads `path.join("game.toml")`).
pub fn load_manifest_from_path(path: &Path) -> Result<GameManifest, AppError> {
    let manifest_path = if path.is_dir() {
        path.join("game.toml")
    } else {
        path.to_path_buf()
    };
    let contents = std::fs::read_to_string(&manifest_path)?;
    parse_manifest_str(&contents)
}
