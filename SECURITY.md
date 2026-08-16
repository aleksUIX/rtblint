# Security Policy

RTBlint parses untrusted JSON payloads, so parser robustness matters.

## Supported versions

Only the latest released version receives vulnerability fixes.

## Reporting a vulnerability

Do not open a public GitHub issue for security vulnerabilities.

Report privately via
[GitHub Security Advisories](https://github.com/aleksUIX/rtblint/security/advisories/new)
or email aleks@vastlint.org with details and a reproducing payload if you have
one.

You will receive a response within 48 hours acknowledging the report. We aim
to disclose a fix within 7 days for critical issues and 30 days for others.
We follow coordinated vulnerability disclosure.

Crashes on malformed input that only affect the CLI locally are fine to
report as regular bug reports.

A vulnerability here means panics on crafted input, memory issues in the WASM
boundary, or anything exploitable in a host that validates untrusted bid
requests.
