use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::LoaderError;

static NEXT_BUILD_ID: AtomicU64 = AtomicU64::new(1);

/// Supported native C11 compiler toolchains for compiling game shared libraries.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompilerKind {
    Tcc,
    Clang,
    Gcc,
}

impl CompilerKind {
    /// Parses a compiler name (`"gcc"`, `"clang"`, or `"tcc"`, case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "gcc" => Some(Self::Gcc),
            "clang" => Some(Self::Clang),
            "tcc" => Some(Self::Tcc),
            _ => None,
        }
    }

    /// Returns the executable name for this compiler toolchain.
    pub fn executable_name(&self) -> &'static str {
        match self {
            Self::Tcc => "tcc",
            Self::Clang => "clang",
            Self::Gcc => "gcc",
        }
    }

    /// Detects an available C11 compiler (`FGE_CC` override first, then `gcc`, `clang`, `tcc`).
    pub fn detect() -> Result<(Self, PathBuf), LoaderError> {
        Self::detect_with_preference(None)
    }

    /// Detects an available C11 compiler (`FGE_CC` override first, then optional preference, then `gcc`, `clang`, `tcc`).
    pub fn detect_with_preference(
        pref: Option<CompilerKind>,
    ) -> Result<(Self, PathBuf), LoaderError> {
        if let Some(cc) = std::env::var_os("FGE_CC").filter(|v| !v.is_empty()) {
            let cc_path = PathBuf::from(cc);
            if probe_compiler(&cc_path) {
                return Ok((Self::from_path(&cc_path), cc_path));
            }
            return Err(LoaderError::CompilerNotFound);
        }

        if let Some(preferred) = pref {
            let candidate = PathBuf::from(preferred.executable_name());
            if probe_compiler(&candidate) {
                return Ok((preferred, candidate));
            }
        }

        for kind in [Self::Gcc, Self::Clang, Self::Tcc] {
            if Some(kind) == pref {
                continue;
            }
            let candidate = PathBuf::from(kind.executable_name());
            if probe_compiler(&candidate) {
                return Ok((kind, candidate));
            }
        }

        Err(LoaderError::CompilerNotFound)
    }

    fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if name.contains("tcc") {
            Self::Tcc
        } else if name.contains("clang") {
            Self::Clang
        } else {
            Self::Gcc
        }
    }
}

fn probe_compiler(path: &Path) -> bool {
    matches!(
        Command::new(path).arg("--version").output(),
        Ok(output) if output.status.success()
    )
}

/// Build specification for compiling a multi-file C11 game project into a shared library.
#[derive(Clone, Debug)]
pub struct CBuildPlan {
    pub project_dir: PathBuf,
    pub entry: String,
    pub sources: Vec<String>,
    pub out_dir: PathBuf,
    pub optimize: bool,
    pub preferred_compiler: Option<CompilerKind>,
}

/// Embedded copy of `include/fge.h` for standalone single-binary header staging.
pub const EMBEDDED_FGE_H: &str = include_str!("../../../include/fge.h");

/// Stages `EMBEDDED_FGE_H` to `<out_dir>/include/fge.h` and returns `<out_dir>/include`.
pub fn stage_embedded_header(out_dir: &Path) -> Result<PathBuf, LoaderError> {
    let include_dir = out_dir.join("include");
    std::fs::create_dir_all(&include_dir)?;

    let header_path = include_dir.join("fge.h");
    let needs_write = match std::fs::read_to_string(&header_path) {
        Ok(existing) => existing != EMBEDDED_FGE_H,
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&header_path, EMBEDDED_FGE_H)?;
    }

    Ok(include_dir)
}

/// Resolves and deduplicates `plan.entry` + `plan.sources` relative to `plan.project_dir`.
pub fn resolve_source_files(plan: &CBuildPlan) -> Result<Vec<PathBuf>, LoaderError> {
    let mut resolved = Vec::with_capacity(1 + plan.sources.len());
    let mut seen = HashSet::with_capacity(1 + plan.sources.len());

    let raw_sources = std::iter::once(&plan.entry).chain(plan.sources.iter());
    for rel_path in raw_sources {
        let candidate = plan.project_dir.join(rel_path);
        if !candidate.is_file() {
            return Err(LoaderError::SourceNotFound(candidate));
        }

        let canonical = std::fs::canonicalize(&candidate)?;
        if seen.insert(canonical.clone()) {
            resolved.push(canonical);
        }
    }

    Ok(resolved)
}

/// Compiles the C11 project into a generation-versioned `.so` under `plan.out_dir`.
pub fn compile_shared_library(plan: &CBuildPlan, generation: u64) -> Result<PathBuf, LoaderError> {
    let (_kind, compiler_path) = CompilerKind::detect_with_preference(plan.preferred_compiler)?;
    let include_dir = stage_embedded_header(&plan.out_dir)?;
    let source_files = resolve_source_files(plan)?;

    std::fs::create_dir_all(&plan.out_dir)?;
    let build_id = NEXT_BUILD_ID.fetch_add(1, Ordering::Relaxed);
    let so_path = plan.out_dir.join(format!(
        "libfge_game_gen{}_{}_{}.so",
        generation,
        std::process::id(),
        build_id
    ));

    let opt_flag = if plan.optimize { "-O2" } else { "-O0" };
    let src_include_dir = plan.project_dir.join("src");

    let mut cmd = Command::new(&compiler_path);
    cmd.arg("-shared")
        .arg("-fPIC")
        .arg("-std=c11")
        .arg(opt_flag)
        .arg("-I")
        .arg(&include_dir)
        .arg("-I")
        .arg(&src_include_dir)
        .arg("-I")
        .arg(&plan.project_dir);

    for src in &source_files {
        cmd.arg(src);
    }

    cmd.arg("-lm").arg("-o").arg(&so_path);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(LoaderError::CompileFailed {
            compiler: compiler_path.display().to_string(),
            stderr,
            status: output.status.code(),
        });
    }

    Ok(so_path)
}
