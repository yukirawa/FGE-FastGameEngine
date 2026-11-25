use std::path::PathBuf;

use crate::snapshot::HeadlessTestReport;

/// Configuration for a GPU-free, deterministic headless simulation test run.
#[derive(Clone, Debug)]
pub struct HeadlessConfig {
    pub project_dir: PathBuf,
    pub project_title: String,
    pub width: u32,
    pub height: u32,
    pub entry: String,
    pub sources: Vec<String>,
    pub frames: u64,
    pub seed: u64,
    pub dt: f32,
    pub snapshot_entity_limit: Option<usize>,
}

impl Default for HeadlessConfig {
    fn default() -> Self {
        Self {
            project_dir: PathBuf::from("."),
            project_title: String::from("FGE Headless Test"),
            width: 1280,
            height: 720,
            entry: String::from("src/main.c"),
            sources: Vec::new(),
            frames: 300,
            seed: 42,
            dt: 1.0 / 60.0,
            snapshot_entity_limit: None,
        }
    }
}

/// Compiles, loads, and executes a C11 game project without opening a window or initializing `wgpu`.
#[allow(unused_variables)]
pub fn run_headless_test(cfg: &HeadlessConfig) -> HeadlessTestReport {
    todo!()
}
