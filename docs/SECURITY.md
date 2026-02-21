# Security Policy

## Supported versions
Cosmos OSS currently ships builds directly from `main`. Once we cut official releases we will publish a support matrix here. Until then, please assume only the latest commit is supported.

## Reporting a vulnerability
1. **Do not open a public issue.**
2. Go to the GitHub repository and click **Security → Report a vulnerability**, or email `security@cosmos-oss.org` (placeholder inbox).
3. Include:
   - A clear description of the issue
   - Steps to reproduce
   - The commit hash / version you tested
   - Any proof-of-concept exploit or screenshots

We will acknowledge reports within 3 business days and provide status updates at least weekly until the issue is resolved.

## Scope
- The desktop application code in this repository (`src/`, `src-tauri/`).
- The build scripts and docs.

Out of scope: proprietary Cosmos services, self-hosted forks, or any infrastructure we don’t manage.

## Disclosure process
1. Triage and reproduce the issue.
2. Prepare a patch + regression tests.
3. Coordinate a release date with the reporter.
4. Publish an advisory and tag a release.
5. Credit the reporter if they consent.

Thanks for helping make Cosmos OSS safer.
