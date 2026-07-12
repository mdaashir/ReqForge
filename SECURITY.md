# Security Policy for ReqForge

## Reporting a Vulnerability

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Send an email to **security@reqforge.io** with details about the vulnerability.
3. You should receive a response within 48 hours.
4. If the issue is confirmed, a patch will be prepared and released within 14 days.
5. The vulnerability will be publicly disclosed after the fix is released.

## Scope

The following are considered in-scope:

- Remote code execution
- Privilege escalation
- Data leakage of user credentials or tokens
- Cross-site scripting (XSS) via rendered response bodies
- Denial of service via crafted import files

The following are out-of-scope:

- Self-XSS
- Missing HTTP security headers on self-hosted instances
- Rate limiting issues on the cloud sync server
- Social engineering of the ReqForge team

## Response Targets

| Severity | First response | Fix released |
|----------|---------------|--------------|
| Critical | 12 hours      | 3 days       |
| High     | 24 hours      | 7 days       |
| Medium   | 48 hours      | 14 days      |
| Low      | 72 hours      | 30 days      |

## Responsible Disclosure

We kindly ask researchers to give us a reasonable time to fix the issue before disclosing it publicly. We will credit you in the release notes unless you prefer to remain anonymous.
