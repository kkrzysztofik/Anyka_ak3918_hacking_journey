# Research Summary: ONVIF Service Gap Coverage

## Decision: Source of Truth for GET Endpoints
- **Rationale**: Reusing the unified configuration runtime and platform adapters guarantees responses stay synchronized with device state while honoring existing validation logic.
- **Alternatives considered**:
  - Hard-coded placeholder data — rejected because it violates FR-015 and undermines interoperability.
  - Direct parsing of raw INI files in each handler — rejected to avoid duplicating logic already consolidated in `config_runtime`.

## Decision: Persistence Strategy for SET Endpoints
- **Rationale**: Leveraging `config_runtime` setters and persistence queue ensures changes apply immediately, coalesce writes, and survive reboot within the two-second requirement.
- **Alternatives considered**:
  - Writing per-handler persistence logic — rejected due to high risk of divergence and missing coalescing safeguards.
  - Deferring persistence to a background cron service — rejected because it would not meet the durability SLA in FR-012.

## Decision: Handling Unsupported Hardware Features (Relay Outputs, Certificates)
- **Rationale**: Returning ONVIF-compliant `ONVIF_ERROR_NOT_SUPPORTED` faults advertises true hardware capabilities while keeping services standards-compliant.
- **Alternatives considered**:
  - Silently ignoring requests — rejected for poor client feedback and potential certification failures.
  - Implementing dummy software-backed relays/cert stores — rejected to avoid misleading integrators and introducing security liabilities.

## Decision: Testing Approach for New Handlers
- **Rationale**: Expand existing CMocka unit suites for validation/error cases and integration suites that replay captured SOAP envelopes to guarantee behavioral parity.
- **Alternatives considered**:
  - Manual QA only — rejected due to lack of automation and conflict with constitutional testing mandates.
  - Creating new end-to-end harness in this effort — deferred to future work per spec scope (unit + integration focus now).
