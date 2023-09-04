# `barge` Security Policy

## Supported Versions

The `barge` software is in a pre-release state, and as such, only nightly-build
eligible source codes are eligible. States represented by the commits on the
`master` branch shall not emit any warning or error messages during either
`cargo build`.

Warnings emitted only be `cargo clippy` may be present, but shall be removed as
soon as detected.

## Potential Vulnerabilities in Dependencies

The repository is configured to emit vulnerability alerts if any dependency of
the project has a known vulnerability. Howevery, `cargo update` and
`cargo audit` shall be run frequently.

## Reporting a Vulnerability

The preferred method to report a vulnerability is by using GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
feature. However, pull requests or e-mail reports are also accepted.
