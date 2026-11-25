use std::path::PathBuf;

use thiserror::Error;

/// Errors produced during C11 compilation, shared library loading, or hot-reload polling.
#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("no supported C11 compiler found (checked FGE_CC, tcc, clang, gcc)")]
    CompilerNotFound,

    #[error("C source file not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("C compilation failed with {compiler} (exit status {status:?}):\n{stderr}")]
    CompileFailed {
        compiler: String,
        stderr: String,
        status: Option<i32>,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("dynamic library load error: {0}")]
    DynamicLoad(#[from] libloading::Error),

    #[error("missing required game entrypoint symbol: {symbol}")]
    MissingSymbol { symbol: &'static str },

    #[error("core error: {0}")]
    Core(#[from] fge_core::error::CoreError),
}
