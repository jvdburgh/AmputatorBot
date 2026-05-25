// AmputatorBot Devvit server — Hono.
// M1: hello-world only — proves the project bundles + uploads.
// Triggers (onCommentSubmit/onPostSubmit/onModMail/onAppInstall) wired in M4
// per docs/amputatorbot-devvit-migration-plan-v7.md.

import { createServer, getServerPort } from '@devvit/web/server';
import { getRequestListener } from '@hono/node-server';
import { Hono } from 'hono';

const app = new Hono();

app.get('/api/health', (c) => c.json({ ok: true, version: '0.1.0' }));

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
