Pos: Page: 433/511 – "Our integration test should pass now!"

- Make sure the session stuff is doing the right things:
  - Don't persist a session at all if not logged in
  - Always generate a new session ID on login (e.g. don't use the cookie)
  - Load entire session object in `FromRequest`
  - Persist the updated session in the middleware, *iff* it's been modified
    - Reuse `Slot` from `axum-sqlx-tx`? `actix-session` uses `Rc`, but axum requires `Send`
