# Security Policy

RTBlint parses untrusted JSON payloads, so parser robustness matters.

## Reporting a vulnerability

Email aleks@vastlint.org with details and a reproducing payload if you have
one. You should get a response within a few days. Please do not open a public
issue for anything you believe is exploitable (panics on crafted input,
memory issues in the WASM boundary, and similar).

Crashes on malformed input that only affect the CLI locally are fine to
report as regular bug reports.

## Supported versions

Only the latest released version receives fixes.
