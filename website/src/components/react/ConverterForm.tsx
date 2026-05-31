import { ArrowRight, Check, Copy, Loader2 } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { type SnippetLang, tokenize } from '@/lib/highlight';
import { cn } from '@/lib/utils';

import { describeMethod } from './canonical-methods';
import type {
  ConvertErrorBody,
  ConvertRequestBody,
  ConvertResponseV2,
  Link,
} from './converter-types';

const EXAMPLE_URL =
  'https://www.google.com/amp/s/news.sky.com/story/amp/gravely-concerning-claims-of-russian-interference-in-general-election-to-spread-support-for-farages-reform-13161235';

// v2 schema defaults — kept in sync with `convert_v2.rs::default_*`. Source
// of truth is the backend; we mirror them here so the form can pre-populate
// the optional-settings panel.
const DEFAULTS = {
  guessAndCheck: true,
  maxDepth: 3,
  redirect: false,
  generateMarkdownComment: false,
} as const;

const SNIPPET_LANG_LABELS: Record<string, string> = {
  html: 'HTML',
  url: 'URL',
  json: 'JSON',
  js: 'JavaScript',
};

interface ResolveState {
  status: 'idle' | 'pending' | 'success' | 'no-amp' | 'error';
  links?: Link[];
  // Set on success when the user asked for it via the "Generate Reddit
  // comment" toggle; `null` when the backend resolved no AMP URLs.
  comment?: string | null;
  errorMessage?: string;
}

export default function ConverterForm() {
  const formId = useId();
  const queryInputRef = useRef<HTMLInputElement>(null);
  const formRef = useRef<HTMLFormElement>(null);

  const [showOptional, setShowOptional] = useState(false);
  const [guessAndCheck, setGuessAndCheck] = useState<boolean>(DEFAULTS.guessAndCheck);
  const [maxDepth, setMaxDepth] = useState<number>(DEFAULTS.maxDepth);
  const [redirect, setRedirect] = useState<boolean>(DEFAULTS.redirect);
  const [generateMarkdownComment, setGenerateMarkdownComment] = useState<boolean>(
    DEFAULTS.generateMarkdownComment,
  );
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

    // We always POST `redirect: false` to the backend even when the user
    // toggled "Forward me to the canonical" — the SPA does the navigation
    // itself once the response comes back. Sending `redirect: true` would
    // return a 303 the browser can't follow out of an XHR.
    const body: ConvertRequestBody = {
      query,
      guessAndCheck,
      maxDepth,
      redirect: false,
      generateMarkdownComment,
    };

    try {
      const response = await fetch('/api/v2/convert', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          // Tag the cache row with the call's origin so per-source
          // analytics (`SELECT entry_type, COUNT(*) FROM links GROUP BY 1`)
          // reflect actual website usage.
          'X-AmputatorBot-Entry-Type': 'ONLINE',
        },
        body: JSON.stringify(body),
      });

      if (response.status === 200) {
        const envelope = (await response.json()) as ConvertResponseV2;
        setResolve({ status: 'success', links: envelope.links, comment: envelope.comment });

        // Honor "Forward me to the canonical" by JS-navigating to whichever
        // link the result-pane logic would have shown. Mirrors the legacy
        // `?r=true` 303 behavior.
        if (redirect) {
          const first = envelope.links[0];
          const target = first?.canonical?.url ?? first?.ampCanonical?.url;
          if (target) window.location.href = target;
        }
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
    if (!queryInputRef.current || !formRef.current) return;
    queryInputRef.current.value = EXAMPLE_URL;
    // requestSubmit fires the form's submit event (running validation + our
    // onSubmit handler) — matches the legacy site, where clicking "Try with
    // an example" both filled the field AND ran the conversion.
    formRef.current.requestSubmit();
  }

  return (
    <Card className="mx-auto w-full max-w-2xl">
      <CardContent className="space-y-4 py-6">
        <form id={formId} ref={formRef} onSubmit={onSubmit} className="space-y-3">
          <label htmlFor={`${formId}-q`} className="sr-only">
            AMP URL or text containing one
          </label>
          <Input
            id={`${formId}-q`}
            ref={queryInputRef}
            name="q"
            type="text"
            required
            autoComplete="off"
            placeholder="Paste a URL — or text containing one (e.g. a Reddit comment)"
            disabled={resolve.status === 'pending'}
            className="h-12 text-base md:text-base"
          />
          <div className="flex flex-wrap items-center gap-3">
            <Button
              type="submit"
              variant="brand"
              size="lg"
              disabled={resolve.status === 'pending'}
              className="w-full sm:w-auto"
            >
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
            <fieldset className="mt-5 space-y-3 rounded-md border border-border bg-muted/30 px-4 py-3 text-sm">
              <legend className="px-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Optional settings
              </legend>
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  checked={guessAndCheck}
                  onChange={(e) => setGuessAndCheck(e.target.checked)}
                  className="mt-0.5 size-4 rounded border-border accent-brand"
                />
                <span>
                  <span className="font-medium">Guess-and-check if necessary.</span> When no
                  canonical signal is present in the page, guess the canonical from the URL pattern
                  and verify it by article-similarity scoring.
                </span>
              </label>
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  checked={redirect}
                  onChange={(e) => setRedirect(e.target.checked)}
                  className="mt-0.5 size-4 rounded border-border accent-brand"
                />
                <span>
                  <span className="font-medium">Forward me to the canonical.</span> After resolving,
                  navigate this tab to the canonical URL automatically.
                  <code className="mx-1">?r=true</code>flag.
                </span>
              </label>
              <label className="flex items-start gap-2">
                <input
                  type="checkbox"
                  checked={generateMarkdownComment}
                  onChange={(e) => setGenerateMarkdownComment(e.target.checked)}
                  className="mt-0.5 size-4 rounded border-border accent-brand"
                />
                <span>
                  <span className="font-medium">Generate Reddit comment.</span> Show a
                  copy-paste-ready Reddit reply alongside the canonical — the same markdown the
                  AmputatorBot bot posts when it finds an AMP URL on a subreddit it's installed in.
                </span>
              </label>
              <label className="flex items-center gap-2">
                <span className="font-medium">Maximum redirects to follow:</span>
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
      {state.comment ? <RedditCommentPanel markdown={state.comment} /> : null}
    </div>
  );
}

function RedditCommentPanel({ markdown }: { markdown: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(markdown);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard API unavailable — the textarea is selectable as a fallback.
    }
  }
  return (
    <div className="rounded-md border border-border bg-card p-4 text-sm">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">
        Reddit comment (Markdown)
      </p>
      <textarea
        readOnly
        value={markdown}
        rows={Math.min(12, Math.max(6, markdown.split('\n').length))}
        className="mt-2 w-full resize-y rounded-md border border-input bg-background px-3 py-2 font-mono text-xs leading-relaxed"
      />
      <div className="mt-3">
        <Button
          type="button"
          onClick={copy}
          variant={copied ? 'secondary' : 'default'}
          size="default"
          className={cn('w-full sm:w-auto', copied && 'bg-emerald-100 text-emerald-900')}
          aria-label="Copy Reddit comment markdown"
        >
          {copied ? <Check /> : <Copy />}
          {copied ? 'Copied to clipboard' : 'Copy Reddit comment'}
        </Button>
      </div>
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

  const method = describeMethod(chosen.type);
  const isAmpFallback = link.canonical === null && link.ampCanonical !== null;

  return (
    <div className="rounded-md border border-border bg-card p-4 text-sm">
      <p className="text-xs uppercase tracking-wide text-muted-foreground">Canonical URL</p>
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
      <div className="mt-3">
        <CopyButton value={chosen.url} />
      </div>

      <HowWeFoundIt
        method={method}
        urlSimilarity={chosen.urlSimilarity}
        isLowConfidence={chosen.isValid === false}
        isAmpFallback={isAmpFallback}
      />

      <details className="mt-4 text-xs text-muted-foreground">
        <summary className="cursor-pointer select-none">Input URL & all candidates</summary>
        <p className="mt-2 break-all">
          <span className="font-medium">Input URL:</span> {link.origin.url}
        </p>
        {link.canonicals.length > 1 ? (
          <ul className="mt-2 space-y-1">
            {link.canonicals.map((c) => {
              const m = describeMethod(c.type);
              return (
                <li key={`${c.type ?? 'unknown'}-${c.url ?? ''}`} className="break-all">
                  <span className="font-medium" title={m.explanation}>
                    {m.label}
                  </span>{' '}
                  → {c.url}
                </li>
              );
            })}
          </ul>
        ) : null}
      </details>
    </div>
  );
}

interface HowWeFoundItProps {
  method: ReturnType<typeof describeMethod>;
  urlSimilarity: number | null | undefined;
  isLowConfidence: boolean;
  isAmpFallback: boolean;
}

function HowWeFoundIt({
  method,
  urlSimilarity,
  isLowConfidence,
  isAmpFallback,
}: HowWeFoundItProps) {
  return (
    <div className="mt-4 rounded-md border border-border bg-muted/30 p-3 text-xs">
      <p className="text-[10px] uppercase tracking-wide text-muted-foreground">How we found it</p>
      <p className="mt-1 text-sm font-medium text-foreground">{method.label}</p>
      <p className="mt-1 text-muted-foreground">{method.explanation}</p>
      {method.snippet ? <Snippet code={method.snippet} language={method.snippetLanguage} /> : null}
      {typeof urlSimilarity === 'number' ? (
        <p className="mt-2 text-muted-foreground">
          Article-text similarity to the original page:{' '}
          <span className="font-mono text-foreground">{(urlSimilarity * 100).toFixed(0)}%</span>{' '}
          <span className="text-muted-foreground">
            (100% = identical text; we accept above 35% with a "low confidence" flag, above 60% as
            high confidence)
          </span>
          .
        </p>
      ) : null}
      {isLowConfidence ? (
        <p className="mt-2 rounded-sm bg-amber-100 px-2 py-1 text-amber-900">
          Low confidence — the canonical was guessed and verified, but the similarity score is in
          the 35–60% band rather than the high-confidence zone.
        </p>
      ) : null}
      {isAmpFallback ? (
        <p className="mt-2 rounded-sm bg-amber-100 px-2 py-1 text-amber-900">
          AMP fallback — the non-AMP version of the canonical wasn't reachable, so this is the AMP
          version we found.
        </p>
      ) : null}
    </div>
  );
}

function Snippet({ code, language }: { code: string; language: SnippetLang | undefined }) {
  const tokens = tokenize(code, language);
  return (
    <figure className="mt-3 rounded-md border border-border bg-background">
      {language ? (
        <figcaption className="border-b border-border px-3 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
          {SNIPPET_LANG_LABELS[language] ?? language}
        </figcaption>
      ) : null}
      <pre className="overflow-x-auto px-3 py-2 text-[12px] leading-relaxed text-foreground">
        <code>
          {tokens.map((t, idx) =>
            t.cls ? (
              // biome-ignore lint/suspicious/noArrayIndexKey: tokens are a derived stable list per snippet, never reordered
              <span key={idx} className={`tok-${t.cls}`}>
                {t.text}
              </span>
            ) : (
              // biome-ignore lint/suspicious/noArrayIndexKey: see above
              <span key={idx}>{t.text}</span>
            ),
          )}
        </code>
      </pre>
    </figure>
  );
}

function CopyButton({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      setTimeout(() => setCopied(false), 3000);
    } catch {
      // Older browsers / permission denied — silently ignore. The user can
      // still select-and-copy from the link text.
    }
  }
  return (
    <Button
      type="button"
      onClick={copy}
      variant={copied ? 'secondary' : 'outline'}
      size="default"
      className={cn('w-full sm:w-auto', copied && 'bg-emerald-100 text-emerald-900')}
      aria-label="Copy canonical URL"
    >
      {copied ? <Check /> : <Copy />}
      {copied ? 'Copied to clipboard' : 'Copy canonical URL'}
    </Button>
  );
}
