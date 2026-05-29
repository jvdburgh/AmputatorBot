import { useEffect, useState } from 'react';

// Small live-count badge. Fetches `/api/v2/stats` on mount and renders the
// number with thousands separators. While loading or if the request fails,
// renders the conservative "1.7M+" placeholder so the section never shows
// a missing value to the user. The backend caches the count for 1h so this
// is cheap even at homepage traffic.

interface StatsResponse {
  convertedTotal: number;
}

const FALLBACK = '1.7M+';

export default function ConvertedCount() {
  const [count, setCount] = useState<string>(FALLBACK);

  useEffect(() => {
    let cancelled = false;
    fetch('/api/v2/stats')
      .then((r) => (r.ok ? (r.json() as Promise<StatsResponse>) : null))
      .then((data) => {
        if (cancelled || !data) return;
        // Locale-aware grouping ("1,742,891"). en-US is a safe default —
        // the rest of the copy is English-only.
        setCount(new Intl.NumberFormat('en-US').format(data.convertedTotal));
      })
      .catch(() => {
        // Network failure — keep the placeholder. No error UI; this is a
        // decorative number, not a load-bearing one.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return <span className="font-mono text-brand">{count}</span>;
}
