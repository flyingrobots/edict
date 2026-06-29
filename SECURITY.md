# Security Policy

Edict is pre-1.0 alpha software (see [`ROADMAP.md`](./ROADMAP.md)). It is a
compiler front end and a local command-line tool: it has no network service,
no database, no authentication surface, and does not execute compiled programs.
The most relevant security properties are deterministic parsing/validation that
does not panic on malformed input, and a minimal, audited dependency surface.

## Supported Versions

Only the most recent published `v*-alpha.*` prerelease is supported. The alpha
train moves forward; older alpha tags do not receive backported fixes. Release
tags are immutable and are never moved, deleted, or recreated — fixes ship in a
new release.

| Version | Supported |
| --- | --- |
| Latest `v0.x.0-alpha.*` prerelease | ✅ |
| Any earlier alpha tag | ❌ |

## Reporting a Vulnerability

Please report suspected vulnerabilities **privately**, not in a public issue:

1. Preferred: open a private advisory via the repository **Security** tab →
   **Report a vulnerability** (GitHub Security Advisories).
2. Alternatively, email <james@flyingrobots.dev> with details and, if possible,
   a minimal reproduction.

Please include the affected commit or release tag, the input or invocation that
triggers the issue, and the observed versus expected behavior. We aim to
acknowledge reports within a few business days. Because Edict has no runtime or
network surface, the most likely report classes are parser/validator denial of
service (panics, unbounded resource use) and contract/encoding integrity issues
in the canonical encoder or digest.

## Disclosure

We prefer coordinated disclosure: please give us a reasonable window to ship a
fix in a new release before any public write-up. We will credit reporters who
wish to be credited.
