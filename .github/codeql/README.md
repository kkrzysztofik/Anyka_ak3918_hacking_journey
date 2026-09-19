# CodeQL configuration

`codeql-config.yml` lists vendor/third-party paths to exclude from analysis.

## This config is inert today

The repo runs CodeQL **Default Setup**, and Default Setup does not read config
files from the repository — only the languages and query suite picked in
**Settings → Code security → CodeQL analysis**. Nothing in
`codeql-config.yml` currently affects a scan. It is kept so the settings are
not lost if the repo ever switches to Advanced Setup.

The step that used to be documented here — "edit and re-save Default Setup so
it picks up the config" — does not work. Do not re-add it.

## Path filters cannot exclude inline test modules

Even under Advanced Setup, `paths-ignore` only removes whole files. Most of
this repo's test code lives in `#[cfg(test)] mod tests` blocks *inside*
production `.rs` files, so a path filter can never reach it. Queries such as
`rust/hard-coded-cryptographic-value` and `rust/cleartext-logging` fire on
those fixtures.

Those alerts are handled per-alert in the Security tab, dismissed as **used in
tests**. That keeps the query live for production code, which is the point —
a path filter or a query filter would blind it everywhere.

## Switching to Advanced Setup

`.github/workflows/codeql.yml_bak` is a ready workflow that passes
`config-file: ./.github/codeql/codeql-config.yml` to the init action. To use
it, rename it to `codeql.yml` **and disable Default Setup in the GitHub UI**,
or every commit gets scanned twice.

Worth doing only if vendor-path noise (e.g. `cross-compile/anyka_reference/**`)
comes back in volume; one-off vendor alerts are cheaper to dismiss.
