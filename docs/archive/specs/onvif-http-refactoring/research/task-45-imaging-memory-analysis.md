# Task 45 – Imaging Service Memory Allocation Analysis

## Scope
- Source: `cross-compile/onvif/src/services/imaging/onvif_imaging.c` (revision 2025-09-28).
- Focus: inventory all heap/stack/global allocations and identify optimization opportunities prior to refactoring for smart response builders.

## Allocation Inventory
- **Global state (BSS)**
  - `g_imaging_auto_config`, `g_imaging_settings`: persistent structs for service defaults.
  - `g_imaging_vi_handle`: platform handle pointer retained across requests.
  - `g_imaging_mutex`, `g_imaging_initialized`: lifecycle synchronization flags.
  - `g_imaging_handler` (struct holding `service_handler_config_t` and `onvif_gsoap_context_t*`).
  - `g_handler_initialized`: lifecycle flag.
- **Service initialization (`onvif_imaging_service_init`)**
  - Allocates `onvif_gsoap_context_t` via `ONVIF_MALLOC(sizeof(onvif_gsoap_context_t))` → freed in `onvif_imaging_service_cleanup` after `onvif_gsoap_cleanup`.
  - Calls `memory_manager_check_leaks()` during cleanup, so leak surfacing depends on correct HTTP response teardown.
- **Per-request (GetImagingSettings)**
  - Stack buffers: `char video_source_token[32]`, `struct imaging_settings settings`.
  - Heap: `response->body = ONVIF_MALLOC(dynamic_size)` sized via `snprintf(NULL, 0, …)+1`.
  - No intermediate scratch buffers; relies on literal SOAP template string.
- **Per-request (SetImagingSettings)**
  - Stack: `struct imaging_settings settings` populated by gSOAP.
  - Heap: `response->body = ONVIF_MALLOC(strlen(template)+1)` followed by `strcpy`.
- **Other allocations**
  - No explicit heap usage in helper functions; VPSS helpers operate on stack values.
  - No use of response buffer macros (`IMAGING_RESPONSE_BUFFER_SIZE`, etc.); these constants now orphaned.

## Observed Patterns
- Response bodies use direct `ONVIF_MALLOC` with raw SOAP templates instead of smart response builders adopted in device/media services.
- Lifetime of `response->body` is managed by HTTP core; handlers themselves never free, consistent with existing convention.
- gSOAP context is per-service singleton; reuse is safe but currently manual. No pooling beyond that single instance.
- Token parsing uses fixed 32-byte stack buffer. Length constant matches `IMAGING_TOKEN_BUFFER_SIZE` (32) so stack allocation is adequate.

## Optimization Opportunities
1. **Adopt smart response builders** – replace manual string templates with the shared builder API to standardize allocation strategy and enable buffer pooling (aligns with Task 46).
2. **Eliminate unused buffer constants** – `IMAGING_RESPONSE_BUFFER_SIZE`, `IMAGING_SETTINGS_BUFFER_SIZE`, `IMAGING_OPTIONS_BUFFER_SIZE`, `IMAGING_VALUE_BUFFER_SIZE` appear unused; confirm before removal during implementation cleanup to reduce confusion.
3. **Swap `strcpy` for `memcpy`/builder usage** – once builders are used the raw `strcpy` goes away; meanwhile if kept, prefer safer copy utilities (though size is controlled) to maintain consistency.
4. **Centralize SOAP templates** – repeated literal strings duplicate namespace declarations; migrating to builder utilities allows reuse and reduces risk of mismatch with other services.
5. **Consider gSOAP context pooling review** – current single allocation is lightweight, but verify thread-safety of shared context once handlers become concurrent; builders can wrap gSOAP interactions similarly to other services.

## Next Steps
- Feed findings into Task 46 implementation plan (smart builder adoption).
- During optimization, ensure HTTP response teardown path frees `response->body` to avoid regressions once dynamic buffers become pooled.
- Update design/task notes with any additional constraints discovered during implementation.
