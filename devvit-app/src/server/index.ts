// AmputatorBot Devvit server — Hono.
//
// Wires two Devvit trigger endpoints to the shared orchestration in
// `triggers/handler.ts`:
//
//   POST /internal/triggers/comment-submit  ← onCommentSubmit
//   POST /internal/triggers/post-submit     ← onPostSubmit
//
// Plus a trivial /api/health for liveness probes during local Docker runs.
// Everything declarative — trigger registrations, settings schema, allow-
// listed http domains — lives in `devvit.json`. This file is just the
// HTTP adapter: parse the trigger payload, build a `TriggerInput`, hand it
// to `handleAmpTrigger` with the real Devvit clients, log the outcome.

import { createServer, getServerPort, reddit, redis } from '@devvit/web/server';
import {
  type OnCommentSubmitRequest,
  type OnPostSubmitRequest,
  T1,
  T3,
  type TriggerResponse,
} from '@devvit/web/shared';
import { getRequestListener } from '@hono/node-server';
import { Hono } from 'hono';

import { BackendClient } from './backend/client.ts';
import { loadInstallSettings } from './settings.ts';
import { handleAmpTrigger } from './triggers/handler.ts';

// Backend base URL. Hardcoded so the published bundle has zero env coupling
// — Devvit's server runtime exposes a sandboxed `process.env` whose support
// matrix isn't worth the type-defs gymnastics here. For `devvit playtest`
// against a local Rust backend, edit this constant and rebuild.
const BACKEND_BASE_URL = 'https://www.amputatorbot.com';
const backend = new BackendClient({ baseUrl: BACKEND_BASE_URL });

// Lazily resolve the per-install app account's Reddit username, then cache
// it for the lifetime of this server process. Used by the trigger handler
// to skip self-replies. `reddit.getAppUser()` is the canonical way to get
// this — each subreddit install has its own app account, so the value is
// stable per install but differs across installs.
//
// Cached as a Promise so concurrent triggers during cold-start share one
// fetch instead of stampeding.
let botUsernamePromise: Promise<string> | undefined;
function getBotUsername(): Promise<string> {
  if (botUsernamePromise === undefined) {
    botUsernamePromise = reddit
      .getAppUser()
      .then((u) => u?.username ?? '')
      .catch((err) => {
        // If the lookup fails, fall back to an empty string — the handler
        // treats that as "guard disabled" and relies on the AMP filter +
        // dedup. Log loudly so we notice if this becomes the steady state.
        console.error(`getAppUser failed; self-reply guard disabled: ${err}`);
        return '';
      });
  }
  return botUsernamePromise;
}

const app = new Hono();

app.get('/api/health', (c) => c.json({ ok: true, version: '0.1.0' }));

app.post('/internal/triggers/comment-submit', async (c) => {
  const body = await c.req.json<OnCommentSubmitRequest>();
  const comment = body.comment;
  if (!comment?.id) return c.json<TriggerResponse>({});

  const [settings, botUsername] = await Promise.all([loadInstallSettings(), getBotUsername()]);
  const outcome = await handleAmpTrigger(
    {
      kind: 'comment',
      // T1(...) is a runtime asserter — throws if the id doesn't have the
      // `t1_` prefix. Devvit guarantees it for onCommentSubmit, so the
      // throw would only fire if Reddit ever changed the protocol.
      id: T1(comment.id),
      body: comment.body ?? '',
      author: body.author?.name,
    },
    { redis, reddit, backend, settings, botUsername },
  );
  console.log(`comment-submit ${comment.id}: ${JSON.stringify(outcome)}`);
  return c.json<TriggerResponse>({});
});

app.post('/internal/triggers/post-submit', async (c) => {
  const body = await c.req.json<OnPostSubmitRequest>();
  const post = body.post;
  if (!post?.id) return c.json<TriggerResponse>({});

  const [settings, botUsername] = await Promise.all([loadInstallSettings(), getBotUsername()]);
  // Join title + url + selftext so URL extraction picks up all three — link
  // posts put the AMP URL in `url`, self-posts in `selftext`, occasional
  // weirdness in the title. Newline separator keeps URL boundaries clean.
  const text = [post.title, post.url, post.selftext].filter(Boolean).join('\n');
  const outcome = await handleAmpTrigger(
    { kind: 'post', id: T3(post.id), body: text, author: body.author?.name },
    { redis, reddit, backend, settings, botUsername },
  );
  console.log(`post-submit ${post.id}: ${JSON.stringify(outcome)}`);
  return c.json<TriggerResponse>({});
});

// Devvit's createServer wraps our handler (auth middleware, request context).
// Hono speaks Fetch API; @hono/node-server's getRequestListener adapts it to
// the Node http.IncomingMessage / ServerResponse shape that createServer expects.
const server = createServer(getRequestListener(app.fetch));

server.on('error', (err: Error) => {
  console.error(`server error: ${err.stack}`);
});

server.listen(getServerPort(), () => {
  console.log(`amputatorbot-devvit-app listening on :${getServerPort()}`);
});
