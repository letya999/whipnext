# Threat Mitigation Map

| Threat | Risk | Preventive control | Detective control | Corrective control |
|---|---:|---|---|---|
| Secret committed to Git | Critical | forbidden-file checks, pre-commit, Gitleaks | local secret scan | rotate and filter history |
| Agent reads `.env` | High | AGENTS.md denylist and hooks | scan logs and context | rotate exposed secret |
| MCP tool poisoning | High | allowlist and no plaintext tokens | MCP config scanner | remove server and rotate tokens |
| Prompt injection in issue/PR | High | GitHub read guard | prompt-injection scan | discard context and re-run safely |
| Hallucinated dependency | High | dependency policy | package audit | remove package and rotate tokens |
