//! `DATABASE` — cache lookup against previously-resolved canonicals.
//!
//! Async because it talks to Postgres. Not dispatched via [`super::try_method`]
//! — the orchestrator awaits it directly, keeping the sync dispatch fn sync.
//!
//! Gated by `ctx.flags.use_db`. The orchestrator turns this off after a
//! non-AMP canonical is found by any earlier method, mirroring the legacy
//! Python's `use_db` flag.
//!
//! DB outages don't crash canonical-finding — a failed lookup logs a warning
//! and is treated as a cache miss, exactly like the legacy Python's
//! `try/except` swallowed connection errors.
//!
//! Ports `archive/helpers/canonical_methods.py:89-96`.

use super::MethodContext;
use crate::canonical::Database;

/// Look up the cached canonical for the URL being resolved at the current
/// depth (`ctx.url`, not `ctx.original_url` — matches legacy behavior).
///
/// Returns `Some(url)` on cache hit, `None` on miss or when `use_db` is off.
pub async fn find<D: Database>(ctx: &MethodContext<'_>, db: &D) -> Option<String> {
    if !ctx.flags.use_db {
        return None;
    }

    match db.lookup_canonical(ctx.url).await {
        Ok(opt) => opt,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                url = %ctx.url,
                "DATABASE lookup failed; treating as cache miss"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::{Future, ready};
    use std::sync::Mutex;

    use anyhow::Result;

    use super::*;
    use crate::canonical::methods::CanonicalFlags;
    use crate::canonical::page::Page;

    /// In-memory [`Database`] for tests. Maps original_url → canonical_url.
    /// Optional toggle to simulate connection failures.
    #[derive(Default)]
    struct MockDatabase {
        rows: HashMap<String, String>,
        fail: Mutex<bool>,
    }

    impl MockDatabase {
        fn empty() -> Self {
            Self::default()
        }

        fn with(mut self, original: &str, canonical: &str) -> Self {
            self.rows
                .insert(original.to_string(), canonical.to_string());
            self
        }

        fn failing() -> Self {
            Self {
                rows: HashMap::new(),
                fail: Mutex::new(true),
            }
        }
    }

    impl Database for MockDatabase {
        fn lookup_canonical(
            &self,
            original_url: &str,
        ) -> impl Future<Output = Result<Option<String>>> + Send {
            if *self.fail.lock().unwrap() {
                return ready(Err(anyhow::anyhow!("simulated DB failure")));
            }
            ready(Ok(self.rows.get(original_url).cloned()))
        }
    }

    /// Throwaway [`Page`] — the DATABASE method only touches the URL string,
    /// page content is irrelevant.
    fn empty_page(url: &str) -> Page {
        Page {
            current_url: url.to_string(),
            status_code: 200,
            title: String::new(),
            html: String::new(),
        }
    }

    fn method_ctx<'a>(page: &'a Page, url: &'a str, flags: CanonicalFlags) -> MethodContext<'a> {
        MethodContext {
            page,
            url,
            original_url: url,
            flags,
        }
    }

    #[tokio::test]
    async fn returns_cached_url_on_hit() {
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let canonical = "https://example.eu/article";
        let db = MockDatabase::empty().with(amp, canonical);
        let page = empty_page(amp);
        let ctx = method_ctx(&page, amp, CanonicalFlags::default());

        assert_eq!(find(&ctx, &db).await.as_deref(), Some(canonical));
    }

    #[tokio::test]
    async fn returns_none_on_miss() {
        let db = MockDatabase::empty();
        let url = "https://amp.example/x";
        let page = empty_page(url);
        let ctx = method_ctx(&page, url, CanonicalFlags::default());

        assert_eq!(find(&ctx, &db).await, None);
    }

    #[tokio::test]
    async fn returns_none_when_use_db_disabled() {
        let amp = "https://www.google.com/amp/s/example.eu/article";
        let db = MockDatabase::empty().with(amp, "https://example.eu/article");
        let page = empty_page(amp);

        let flags = CanonicalFlags {
            use_db: false,
            use_gac: true,
            use_mr: true,
        };
        let ctx = method_ctx(&page, amp, flags);

        assert_eq!(
            find(&ctx, &db).await,
            None,
            "should ignore cache when use_db=false"
        );
    }

    #[tokio::test]
    async fn treats_db_error_as_cache_miss() {
        let db = MockDatabase::failing();
        let url = "https://amp.example/x";
        let page = empty_page(url);
        let ctx = method_ctx(&page, url, CanonicalFlags::default());

        assert_eq!(
            find(&ctx, &db).await,
            None,
            "DB errors should not crash the resolver"
        );
    }
}
