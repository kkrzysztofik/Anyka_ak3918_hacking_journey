---
applyTo: "**/*.rs,**/auth/**,**/security/**"
description: "Security best practices for embedded ONVIF implementation"
---

# Security Guidelines

## Input Validation

- Validate all user inputs using `validator` crate
- Bound string lengths appropriately
- Check for XML XXE and XML bomb attacks
- Validate paths against directory traversal
- Never trust input from network sources

## Authentication

- Use timing-safe comparison for credentials (`constant_time_eq`)
- Hash passwords with Argon2
- Implement nonce freshness for session management
- Rate limit authentication endpoints
- Log authentication failures

## Memory Safety

- Minimize `unsafe` code blocks
- Document all `unsafe` with `// SAFETY:` comments
- Use proper synchronization (Arc, Mutex, RwLock)
- Clean up resources in error paths
- Respect 24MB memory constraint

## Network Security

- Don't leak information in error messages
- Use HTTPS/TLS in production
- Implement rate limiting
- Never hardcode secrets
- Validate all SOAP/XML payloads

## Cryptography

- Use well-vetted cryptographic libraries
- Never implement custom crypto
- Use secure random number generation
- Rotate keys appropriately

## Error Handling

- Don't expose internal details in user-facing errors
- Log detailed errors server-side only
- Return generic error messages to clients
- Use proper ONVIF SOAP faults

## Dependency Security

- Run `cargo audit` regularly
- Keep dependencies updated
- Review dependencies before adding
- Minimize dependency count
