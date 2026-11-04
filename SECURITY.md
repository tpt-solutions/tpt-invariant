# Security Policy

## Supported versions

Only the latest published release receives security fixes. The project is
pre-release, so no version is currently supported for production use.

## Reporting a vulnerability

Please do **not** open a public issue for security problems.

Report privately through GitHub's "Report a vulnerability" feature on the
repository's Security tab. If that is unavailable, contact the maintainers
(TPT Solutions) directly.

Include:

- a description of the issue and its impact;
- a minimal reproduction (a small crate or source file);
- the affected version or commit.

We aim to acknowledge reports within 7 days.

Note that tpt-invariant is a verifier. A "proved" result that is actually
unsound is a security-relevant bug, so please report those too.
