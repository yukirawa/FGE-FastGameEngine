use std::path::{Path, PathBuf};

use crate::error::LoaderError;

/// Supported native C11 compiler toolchains for compiling game shared libraries.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompilerKind {
    Tcc,
    Clang,
    Gcc,
}

impl CompilerKind {
    /// Returns the executable name for this compiler toolchain.
    pub fn executable_name(&self) -> &'static str {
        todo!()
    }

    /// Detects an available C11 compiler (`FGE_CC` override first, then `tcc`, `clang`, `gcc`).
    pub fn detect() -> Result<(Self, PathBuf), LoaderError> {
        todo!()
    }
}

/// Build specification for compiling a multi-file C11 game project into a shared library.
#[derive(Clone, Debug)]
pub struct CBuildPlan {
    pub project_dir: PathBuf,
    pub entry: String,
    pub sources: Vec<String>,
    pub out_dir: PathBuf,
    pub optimize: bool,
}

/// Embedded copy of `include/fge.h` for standalone single-binary header staging.
pub const EMBEDDED_FGE_H: &str = include_str!("../../../include/fge.h");

/// Stages `EMBEDDED_FGE_H` to `<out_dir>/include/fge.h` and returns `<out_dir>/include`.
#[allow(unused_variables)]
pub fn stage_embedded_header(out_dir: &Path) -> Result<PathBuf, LoaderError> {
    todo!()
}

/// Resolves and deduplicates `plan.entry` + `plan.sources` relative to `plan.project_dir`.
#[allow(unused_variables)]
pub fn resolve_source_files(plan: &CBuildPlan) -> Result<Vec<PathBuf>, LoaderError> {
    todo!()
}

/// Compiles the C11 project into a generation-versioned `.so` under `plan.out_dir`.
#[allow(unused_variables)]
pub fn compile_shared_library(plan: &CBuildPlan, generation: u64) -> Result<PathBuf, LoaderError> {
    todo!()
}
