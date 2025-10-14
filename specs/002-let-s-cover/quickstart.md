# Quickstart: Implementing ONVIF Service Gap Coverage

## 1. Prepare Environment
1. Checkout branch `002-let-s-cover`.
2. Ensure gSOAP codegen artifacts are up to date (`make -C cross-compile/onvif generated`).
3. Install CMocka dependencies if not present (`./cross-compile/onvif/tests/install_dependencies.sh`).

## 2. Implement Service Handlers
1. Update Device, Media, PTZ, Imaging handlers under `cross-compile/onvif/src/services/**` to cover missing `Get*` and `Set*` operations.
2. Utilize `config_runtime` APIs for reads/writes; add helpers where gaps exist (e.g., network introspection, profile persistence).
3. Return `ONVIF_ERROR_NOT_SUPPORTED` for relay and certificate operations.

## 3. Update Protocol Layer
1. Extend gSOAP request parsing/response generation files in `src/protocol/gsoap/` for new operations.
2. Maintain include ordering and documentation headers per Development Standards.

## 4. Expand Tests
1. Add CMocka unit tests in `tests/src/unit/services/*` covering success/failure paths for each new handler.
2. Add integration tests in `tests/src/integration` that replay representative SOAP envelopes from `cap/` captures.
3. Run `make test`, `make test-unit`, and confirm logs are clean.

## 5. Validate Quality Gates
1. Run `./cross-compile/onvif/scripts/lint_code.sh --check` and `./cross-compile/onvif/scripts/format_code.sh --check`.
2. Regenerate docs: `make -C cross-compile/onvif docs`.
3. Confirm automated tests pass in CI configuration.

## 6. Document Changes
1. Update Doxygen comments for new APIs.
2. Document configuration schema additions or behavioral changes in appropriate markdown/README files.
