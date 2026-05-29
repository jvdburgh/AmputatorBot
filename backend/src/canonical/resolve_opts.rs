//! [`ResolveOpts`] — per-request canonical-finding configuration.

/// Per-request options for [`crate::canonical::resolve`].
///
/// Ports the `max_depth`, `use_db`, `use_gac`, `use_mr` parameters of the
/// Python `get_canonicals` function. Defaults match `praw-python-archive/static/static.py`
/// + the Python defaults (depth 3, all gated methods enabled).
#[derive(Debug, Clone, Copy)]
pub struct ResolveOpts {
    /// Maximum recursion depth when a method returns another AMP URL —
    /// the orchestrator will refetch and try again, bounded by this. Matches
    /// `praw-python-archive/static/static.py:MAX_DEPTH = 3`.
    pub max_depth: u32,
    /// Try the DATABASE method (cache lookup).
    pub use_db: bool,
    /// Try the GUESS_AND_CHECK method (resource-heavy: extra HTTP fetch + readability extraction).
    pub use_gac: bool,
    /// Try the META_REDIRECT method.
    pub use_mr: bool,
}

impl Default for ResolveOpts {
    fn default() -> Self {
        Self {
            max_depth: 3,
            use_db: true,
            use_gac: true,
            use_mr: true,
        }
    }
}
