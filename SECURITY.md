# Security Policy

## Supported Versions

Security fixes are provided for the latest released version and the current default branch.

## Scope

Code Memory runs locally and may index source code, Git history, and stored knowledge. Reports are in scope when they involve project-root escape, unauthorized file access or modification, disclosure beyond the authorized local client, execution or corruption caused by untrusted repository content, the optional loopback web UI, or the npm release-binary installation path.

Expected access by the authorized local user or MCP client to the selected project, and behavior of the host AI model, are outside scope unless Code Memory crosses one of these boundaries.

## Reporting a Vulnerability

Report suspected vulnerabilities through [GitHub private vulnerability reporting](https://github.com/mstuart/code-memory/security/advisories/new). Do not open a public issue.

Include the affected version, environment, reproduction steps, impact, and any suggested mitigation. Do not include secrets or personal data. Remediation and disclosure will be coordinated through the private advisory.
