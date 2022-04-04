# `zero2prod`

Following along with https://zero2prod.com/.

## Features

Per the user stories:

- > As a blog visitor,<br>
  > I want to subscribe to the newsletter,<br>
  > So that I can receive email updates when new content is published on the blog;

- > As the blog author,<br>
  > I want to send an email to all my subscribers,<br>
  > So that I can notify them when new content is published;

- > As a subscriber,<br>
  > I want to be able to unsubscribe from the newsletter,<br>
  > So that I can stop receiving email updates from the blog.

Explicitly out of scope:

- Manage multiple newsletters;
- Segment subscribers in multiple audiences;
- Track opening and click rates.

## Divergence from the book

- [axum](https://docs.rs/axum/latest/axum/) is used as the web framework, rather than actix-web.
  axum feels a bit lighter, is a first-party tokio citizen, and benefits from the `hyper` and `tower` ecosystems.
  On the downside it's a bit less mature, so some integrations may be missing.

- No config files – configuration can only be supplied through environment variables.
  I prefer this as it keeps configuration management simple and consistent across all environments, and avoids any of the layering needed with a config file based approach.
  [Envy](https://docs.rs/envy/latest/envy/) is used to drive this.

- Request IDs are set by middleware, rather than per-handler tracing spans.
  This ensures **all** requests are augmented with a request ID.
  The middleware also sets the request ID in a response header, so it could potentially be shown to clients on errors to give a correlation ID for debugging without exposing internal errors or logging PII.

- An alpine-based image is used.
  This gives significantly lighter images (~10MiB), although there were some hiccups getting it working in DigitalOcean App Platform (see the comment in the [`Dockerfile`](Dockerfile)).
