# Implementation Plan: ONVIF Service Gap Coverage

**Branch**: `002-let-s-cover` | **Date**: 2025-10-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-let-s-cover/spec.md`

## Summary

Implement missing ONVIF Device, Media, PTZ, and Imaging operations observed in captured traffic, ensuring each `Get*` endpoint returns live data and each `Set*` endpoint updates runtime state with persistence to the unified configuration system. Update unit and integration test suites to cover the new handlers while returning standards-compliant faults for features the hardware does not support (relay outputs, certificate lifecycle).

## Technical Context

<!--
  ACTION REQUIRED: Replace the content in this section with the technical details
  for the project. The structure here is presented in advisory capacity to guide
  the iteration process.
-->

**Language/Version**: C11 (arm-anykav200-linux-uclibcgnueabi toolchain)  
**Primary Dependencies**: gSOAP 2.8 generated stubs, project config/runtime libraries, platform abstraction (`platform_anyka.c`)  
**Storage**: Unified configuration system persisted to INI-style files on the device filesystem  
**Testing**: CMocka unit and integration suites executed via `make test`, `make test-unit`, and integration harness  
**Target Platform**: Embedded Linux on Anyka AK3918 (ARM)  
**Project Type**: Embedded service daemon (single repo under `cross-compile/onvif`)  
**Performance Goals**: Sub-second response for ONVIF operations; setters must persist within two seconds as defined in spec  
**Constraints**: Limited RAM/flash of AK3918 platform; must maintain memory safety and standards compliance per constitution  
**Scale/Scope**: Single camera device servicing multiple simultaneous ONVIF clients (1–10 concurrent)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Security First**: Plan must include input validation for all new handlers, safe memory usage, and logging of security-relevant operations. ✅ Addressed via requirements FR-013/FR-014 and test coverage tasks.
- **Standards Compliance**: Changes must align with ONVIF 24.12, HTTP/1.1, Profile S/T requirements. ✅ Implementations will reuse gSOAP bindings and follow captured traffic plus spec mandates.
- **Testing Requirements**: Must extend CMocka unit/integration suites with full coverage. ✅ User Story 3 and success criteria enforce automated testing deliverables.
- **Open Source Transparency & Documentation**: Need Doxygen updates and configuration docs reflecting new endpoints. ✅ Documentation tasks will be included in work breakdown.
- **Performance & Reliability**: Persistence deadlines (≤2s) and response times must be honored. ✅ Requirements specify timing and persistence behavior.
-> No constitutional violations identified; proceed to Phase 0.

*Post-Phase-1 Review*: Design artifacts (data model, contracts, quickstart) reinforce the above controls; no new constitutional risks detected.

## Project Structure

### Documentation (this feature)

```
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)
<!--
  ACTION REQUIRED: Replace the placeholder tree below with the concrete layout
  for this feature. Delete unused options and expand the chosen structure with
  real paths (e.g., apps/admin, packages/something). The delivered plan must
  not include Option labels.
-->

```
cross-compile/onvif/
├── src/
│   ├── core/
│   ├── networking/
│   ├── protocol/gsoap/
│   ├── services/
│   │   ├── device/
│   │   ├── media/
│   │   ├── ptz/
│   │   └── imaging/
│   ├── platform/
│   └── utils/
└── tests/
    ├── src/
    │   ├── unit/
    │   ├── integration/
    │   └── mocks/
    └── out/

cap/                         # Captured request/response samples
specs/002-let-s-cover/       # Feature documentation outputs
```

**Structure Decision**: Utilize existing `cross-compile/onvif` monorepo layout; services and protocol layers will be updated in-place with matching unit/integration tests under `tests/src`.

## Complexity Tracking

*Fill ONLY if Constitution Check has violations that must be justified*

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
