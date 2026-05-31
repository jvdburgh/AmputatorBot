//! `DATABASE` — cache lookup for previously-resolved canonicals. Gated by
//! `ctx.flags.use_db`. DB errors are logged and treated as a cache miss so
//! outages don't crash canonical-finding.

use super::MethodContext;
use crate::canonical::Database;

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
