import { ArrowRight, Check, Copy, Loader2 } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/utils';

import type { ConvertErrorBody, ConvertRequestBody, Link } from './converter-types';

// One example URL Joris shipped on the legacy site's "Try with an example"
// button. Keeping it identical so the new form's "try example" experience is
// instantly recognizable to existing users.
const EXAMPLE_URL =
  'https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3-assembly-line-inside-tent-elon-musk/amp/';

// v2 schema defaults — kept in sync with `convert_v2.rs::default_*`. Source
// of truth is the backend; we mirror them here so the form can pre-populate
// the optional-settings panel.
const DEFAULTS = {
  guessAndCheck: true,
  maxDepth: 3,
} as const;

interface ResolveState {
  status: 'idle' | 'pending' | 'success' | 'no-amp' | 'error';
  links?: Link[];
  errorMessage?: string;
}

export default function ConverterForm() {
  const formId = useId();
  const queryInputRef = useRef<HTMLInputElement>(null);

  const [showOptional, setShowOptional] = useState(false);
  const [guessAndCheck, setGuessAndCheck] = useState<boolean>(DEFAULTS.guessAndCheck);
  const [maxDepth, setMaxDepth] = useState<number>(DEFAULTS.maxDepth);
  const [resolve, setResolve] = useState<ResolveState>({ status: 'idle' });

  // Backwards compatibility: the legacy site rendered the same converter at
  // `/` and accepted a `?q=` query param to pre-fill the input. Thousands of
  // historical AmputatorBot Reddit comments and DMs deep-link with that
  // pattern (`https://www.amputatorbot.com/?q=<url>`). Pre-fill but don't
  // auto-submit — the user might be there to inspect, not convert.
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const initial = new URLSearchParams(window.location.search).get('q');
    if (initial && queryInputRef.current && !queryInputRef.current.value) {
      queryInputRef.current.value = initial;
    }
  }, []);

  async function onSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const query = String(data.get('q') ?? '').trim();
    if (!query) return;

    setResolve({ status: 'pending' });

    const body: ConvertRequestBody = {
      query,
      guessAndCheck,
      maxDepth,
      redirect: false,
      entryType: 'ONLINE',
    };

    try {
      const response = await fetch('/api/v2/convert', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (response.status === 200) {
        const links = (await response.json()) as Link[];
        setResolve({ status: 'success', links });
        return;
      }

      // The backend returns 406 + errorMessage when the input had no AMP URL.
      // Treat that as its own "expected" state — not a failure — so the UI
      // can show a softer message than for genuine errors.
      if (response.status === 406) {
        setResolve({ status: 'no-amp' });
        return;
      }

      const error = (await response.json()) as ConvertErrorBody;
      setResolve({
        status: 'error',
        errorMessage: error.errorMessage ?? `Request failed with status ${response.status}.`,
      });
    } catch {
      setResolve({
        status: 'error',
        errorMessage: 'Network error: could not reach the AmputatorBot API. Please try again.',
      });
    }
  }

  function fillExample() {
    if (queryInputRef.current) {
      queryInputRef.current.value = EXAMPLE_URL;
      queryInputRef.current.focus();
    }
  }

  return (
    <Card className="mx-auto w-full max-w-2xl">
      <CardHeader>
        <CardTitle className="text-xl">Remove AMP from your URLs</CardTitle>
        <CardDescription>
          Paste an AMP link and AmputatorBot will return the canonical version.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <form id={formId} onSubmit={onSubmit} className="space-y-3">
          <label htmlFor={`${formId}-q`} className="sr-only">
            AMP URL
          </label>
          <Input
            id={`${formId}-q`}
            ref={queryInputRef}
            name="q"
            type="url"
            required
            inputMode="url"
            autoComplete="off"
            placeholder="https://www.google.com/amp/s/example.eu/article/amp/"
            disabled={resolve.status === 'pending'}
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" variant="brand" size="lg" disabled={resolve.status === 'pending'}>
              {resolve.status === 'pending' ? (
                <>
                  <Loader2 className="animate-spin" />
                  Resolving…
                </>
              ) : (
                <>
                  Submit URL
                  <ArrowRight />
                </>
              )}
            </Button>
            <button
              type="button"
              onClick={fillExample}
              className="text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
            >
              Try an example
            </button>
            <button
              type="button"
              onClick={() => setShowOptional((v) => !v)}
              className="ml-auto text-sm text-muted-foreground hover:text-foreground"
              aria-expanded={showOptional}
            >
              {showOptional ? '− Hide options' : '+ Show options'}
            </button>
          </div>

          {showOptional ? (
            <fieldset className="space-y-3 rounded-md border border-border bg-muted/30 px-4 py-3 text-sm">
              <legend className="px-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Optional settings
              </legend>
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={guessAndCheck}
                  onChange={(e) => setGuessAndCheck(e.target.checked)}
                  className="size-4 rounded border-border accent-brand"
                />
                <span>
                  <span className="font-medium">Guess-and-check fallback.</span> When no canonical
                  signal is present in the HTML, guess from URL patterns and verify by article
                  similarity.
                </span>
              </label>
              <label className="flex items-center gap-2">
                <span className="font-medium">Max redirect depth:</span>
                <select
                  value={maxDepth}
                  onChange={(e) => setMaxDepth(Number(e.target.value))}
                  className="rounded-md border border-input bg-background px-2 py-1 text-sm"
                >
                  {[0, 1, 2, 3, 4].map((n) => (
                    <option key={n} value={n}>
                      {n}
                      {n === DEFAULTS.maxDepth ? ' (default)' : ''}
                    </option>
                  ))}
                </select>
              </label>
            </fieldset>
          ) : null}
        </form>

        <ConverterResult state={resolve} />
      </CardContent>
    </Card>
  );
}

function ConverterResult({ state }: { state: ResolveState }) {
  if (state.status === 'idle' || state.status === 'pending') return null;

  if (state.status === 'no-amp') {
    return (
      <div className="rounded-md border border-border bg-muted/30 p-4 text-sm">
        <p className="font-medium">No AMP link detected.</p>
        <p className="mt-1 text-muted-foreground">
          AmputatorBot looks for tells like <code>/amp</code>, <code>?amp=</code>, <code>amp.</code>
          , and cached domains such as <code>www.google.com/amp/s/</code>. The URL you submitted
          doesn't match any of them.
        </p>
      </div>
    );
  }

  if (state.status === 'error') {
    return (
      <div
        role="alert"
        className="rounded-md border border-destructive/40 bg-destructive/5 p-4 text-sm text-destructive"
      >
        <p className="font-medium">Something went wrong.</p>
        <p className="mt-1">{state.errorMessage}</p>
      </div>
    );
  }

  // Success.
  const links = state.links ?? [];
  if (links.length === 0) {
    return (
      <div className="rounded-md border border-border bg-muted/30 p-4 text-sm text-muted-foreground">
        No links were resolvable in your input.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {links.map((link, idx) => (
        <LinkResult key={link.origin.url ?? idx} link={link} />
      ))}
    </div>
  );
}

function LinkResult({ link }: { link: Link }) {
  // Prefer the non-AMP canonical; fall back to amp_canonical (set when the
  // origin was a cached AMP page and only an AMP canonical was reachable).
  // Mirrors the redirect-target logic in backend/src/routes/convert.rs.
  const chosen = link.canonical ?? link.ampCanonical;

  if (!chosen?.url) {
    return (
      <div className="rounded-md border border-border bg-muted/30 p-4 text-sm">
        <p className="font-medium">No canonical found.</p>
        <p className="mt-1 break-all text-muted-foreground">
          Origin: <code>{link.origin.url}</code>
        </p>
      </div>
    );
  }

  return (
    <div className="rounded-md border border-border bg-card p-4 text-sm">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">Canonical</p>
      <p className="mt-1 break-all">
        <a
          href={chosen.url}
          rel="noopener noreferrer"
          target="_blank"
          className="font-medium text-brand underline-offset-4 hover:underline"
        >
          {chosen.url}
        </a>
      </p>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <CopyButton value={chosen.url} />
        {chosen.type ? (
          <span className="rounded-sm bg-muted px-1.5 py-0.5">via {chosen.type}</span>
        ) : null}
        {typeof chosen.urlSimilarity === 'number' ? (
          <span className="rounded-sm bg-muted px-1.5 py-0.5">
            similarity {(chosen.urlSimilarity * 100).toFixed(0)}%
          </span>
        ) : null}
        {chosen.isValid === false ? (
          <span className="rounded-sm bg-amber-100 px-1.5 py-0.5 text-amber-900">
            low confidence
          </span>
        ) : null}
        {link.canonical === null && link.ampCanonical !== null ? (
          <span className="rounded-sm bg-amber-100 px-1.5 py-0.5 text-amber-900">
            AMP fallback — no non-AMP version reachable
          </span>
        ) : null}
      </div>
      <details className="mt-3 text-xs text-muted-foreground">
        <summary className="cursor-pointer select-none">Origin & all candidates</summary>
        <p className="mt-2 break-all">
          <span className="font-medium">Origin:</span> {link.origin.url}
        </p>
        {link.canonicals.length > 1 ? (
          <ul className="mt-2 space-y-1">
            {link.canonicals.map((c) => (
              <li key={`${c.type ?? 'unknown'}-${c.url ?? ''}`} className="break-all">
                <code>{c.type}</code> → {c.url}
              </li>
            ))}
          </ul>
        ) : null}
      </details>
    </div>
  );
}

function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Older browsers / permission denied — silently ignore. The user can
      // still select-and-copy from the link text.
    }
  }
  return (
    <button
      type="button"
      onClick={copy}
      className={cn(
        'inline-flex items-center gap-1 rounded-sm bg-muted px-1.5 py-0.5 transition-colors hover:bg-accent',
        copied && 'bg-emerald-100 text-emerald-900',
      )}
      aria-label="Copy canonical URL"
    >
      {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
      {copied ? 'Copied' : 'Copy'}
    </button>
  );
}
