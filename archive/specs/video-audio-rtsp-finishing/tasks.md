# Tasks Document

- [ ] 1. Implement media capability helper module
  - Files: cross-compile/onvif/src/services/media/onvif_media_capabilities.c, cross-compile/onvif/src/services/media/onvif_media_capabilities.h
  - Create a helper that refreshes profile/video/audio parameters from core config and platform queries, exposing transport metadata for Profile S/T responses.
  - Purpose: Replace hard-coded profile definitions with runtime data to satisfy capability accuracy requirements.
  - _Leverage: cross-compile/onvif/src/services/media/onvif_media.c, cross-compile/onvif/src/core/config/config.h, cross-compile/onvif/src/platform/platform.h_
  - _Requirements: Requirement 1, Requirement 2, Requirement 5_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded C developer focusing on ONVIF services | Task: Create onvif_media_capabilities.c/.h that load profile/video/audio settings from core config and platform queries, surface transport metadata, and expose accessors for the Media service | Restrictions: Do not break existing include ordering rules, reuse validation/error macros, keep data in existing profile structs | _Leverage: cross-compile/onvif/src/services/media/onvif_media.c, cross-compile/onvif/src/core/config/config.h, cross-compile/onvif/src/platform/platform.h_ | _Requirements: Requirement 1, Requirement 2, Requirement 5_ | Success: Helper compiles, passes lint/format checks, and unit tests can mock its platform interactions; remember to mark this task as in-progress by changing to [-] before coding and [x] once complete._

- [ ] 2. Wire capability helper into Media service callbacks
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Invoke the new helper to populate profiles, stream URIs, and capability responses while preserving existing dispatcher logic.
  - Purpose: Ensure ONVIF SOAP responses reflect accurate runtime capabilities and transport policies.
  - _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/utils/memory/smart_response_builder.h_
  - _Requirements: Requirement 1, Requirement 2_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media service specialist | Task: Update onvif_media.c to call the new capability helper for GetProfiles/GetStreamUri/etc., populating transport data and audio flags while retaining existing dispatcher plumbing | Restrictions: No duplicate profile storage, maintain error code constants, keep includes ordered | _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/utils/memory/smart_response_builder.h_ | _Requirements: Requirement 1, Requirement 2_ | Success: Media callbacks compile, unit tests cover new logic, SOAP responses return updated values; set this task to [-] while working and [x] after completion._

- [ ] 3. Implement shared stream router utility
  - Files: cross-compile/onvif/src/utils/stream/stream_router.c, cross-compile/onvif/src/utils/stream/stream_router.h
  - Manage encoder/audio handle reference counting so multiple consumers reuse Anyka hardware streams without spawning duplicate encoders.
  - Purpose: Guarantee encoder reuse for RTSP and future streaming consumers.
  - _Leverage: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/utils/network/network_utils.h_
  - _Requirements: Requirement 3, Requirement 5_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded systems engineer focused on media pipelines | Task: Build stream_router.c/.h that wraps platform_venc/aenc request/release APIs with mutex-protected ref counting per profile token | Restrictions: Follow utils module conventions, no global state without g_prefix, expose clear acquire/release API | _Leverage: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/utils/network/network_utils.h_ | _Requirements: Requirement 3, Requirement 5_ | Success: Utility compiles, provides thread-safe reuse, and logs reuse events; mark task [-] while developing and [x] when verified._

- [ ] 4. Integrate stream router into RTSP multistream handling
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_multistream.c, cross-compile/onvif/src/networking/rtsp/rtsp_multistream.h
  - Replace direct encoder/VI handle management with stream router API, ensure cleanup paths release references correctly, and remove per-session capture stop/start logic now handled by the router.
  - Purpose: Enable concurrent RTSP clients without duplicating encoder sessions.
  - _Leverage: cross-compile/onvif/src/utils/stream/stream_router.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.h_
  - _Requirements: Requirement 3, Requirement 5_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP server developer | Task: Update rtsp_multistream to acquire/release encoder handles through stream_router, handle teardown on disconnect, and maintain existing thread model | Restrictions: Preserve current synchronization primitives, no regression to RTP send loops, maintain logging style | _Leverage: cross-compile/onvif/src/utils/stream/stream_router.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.h_ | _Requirements: Requirement 3, Requirement 5_ | Success: Multiple SETUP/PLAY calls share encoders, teardown frees references, tests confirm no double-close; remember [-]/[x] status updates in tasks.md._

- [ ] 5. Introduce RTSP pipeline controller for transport validation
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_pipeline_controller.c, cross-compile/onvif/src/networking/rtsp/rtsp_pipeline_controller.h, cross-compile/onvif/src/networking/rtsp/rtsp_server.c
  - Add a controller that mediates DESCRIBE/SETUP using capability data, rejects RTPS/RTSPS, and configures session timeouts from Media metadata.
  - Purpose: Centralize transport policy enforcement and session preparation.
  - _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.c, cross-compile/onvif/src/networking/rtsp/rtsp_types.h_
  - _Requirements: Requirement 1, Requirement 3_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP control-plane engineer | Task: Create rtsp_pipeline_controller module that reads media transport info, enforces unsupported transport errors, and wires into rtsp_server DESCRIBE/SETUP flow | Restrictions: Reuse existing parsing helpers, do not duplicate auth handling, maintain return code conventions | _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.c, cross-compile/onvif/src/networking/rtsp/rtsp_types.h_ | _Requirements: Requirement 1, Requirement 3_ | Success: Controller integrated, RTPS/RTSPS requests return RTSP_UNSUPPORTED_TRANSPORT, timeouts align with Media helper; update task status to [-]/[x] appropriately._

- [ ] 6. Enhance SDP generation for dual A/V profiles
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_sdp.c, cross-compile/onvif/src/networking/rtsp/rtsp_describe.c (or equivalent handler), cross-compile/onvif/src/networking/rtsp/rtsp_sdp.h
  - Populate SDP with capability-driven video/audio attributes, Anyka-aligned fmtp strings, and correct control URIs sourced from the capability helper.
  - Purpose: Provide ONVIF Profile S/T compliant SDP responses that match stream availability.
  - _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/networking/rtsp/rtsp_pipeline_controller.h_
  - _Requirements: Requirement 1, Requirement 2_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: SDP/RTSP specialist | Task: Update SDP construction to use capability metadata, include audio tracks when enabled, and align fmtp/control fields with Anyka reference behaviour | Restrictions: Maintain existing allocator usage, avoid hard-coded literals where helper data exists, respect string buffer limits | _Leverage: cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/networking/rtsp/rtsp_pipeline_controller.h_ | _Requirements: Requirement 1, Requirement 2_ | Success: SDP output validated by unit tests, audio inclusion toggles correctly, lint/format pass; remember task status updates._

- [ ] 7. Expand unit tests for media, RTSP, and stream router modules
  - Files: cross-compile/onvif/tests/src/unit/services/media/test_onvif_media_callbacks.c, cross-compile/onvif/tests/src/unit/networking/rtsp/test_rtsp_pipeline_controller.c, cross-compile/onvif/tests/src/unit/utils/stream/test_stream_router.c (new)
  - Cover capability refresh logic, transport rejection paths, stream router reference counting, and SDP composition.
  - Purpose: Provide regression coverage for new logic per testing requirements.
  - _Leverage: cross-compile/onvif/tests/src/mocks/platform_mock.c, cross-compile/onvif/tests/src/mocks/network_mock.c, cross-compile/onvif/tests/src/mocks/mock_service_dispatcher.c_
  - _Requirements: Requirement 1, Requirement 2, Requirement 3, Requirement 5, Requirement 4_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C unit test engineer using CMocka | Task: Write/extend unit tests to assert capability values, transport rejection, stream router reuse, and SDP correctness | Restrictions: Use existing mocks/utilities, keep tests deterministic, ensure coverage of error paths | _Leverage: cross-compile/onvif/tests/src/mocks/platform_mock.c, cross-compile/onvif/tests/src/mocks/network_mock.c, cross-compile/onvif/tests/src/mocks/mock_service_dispatcher.c_ | _Requirements: Requirement 1, Requirement 2, Requirement 3, Requirement 5, Requirement 4_ | Success: `make test` passes with new suites, coverage reports show exercised paths; adjust task status [-]/[x] as you work._

- [ ] 8. Update integration scripts and documentation for streaming workflow
  - Files: SD_card_contents/anyka_hack/scripts/start_rtsp_tests.sh (new or updated), docs/agents/streaming_validation.md (new), README streaming section updates
  - Provide automation to run DESCRIBE/SETUP/PLAY checks, document configuration toggles, and highlight unsupported RTPS behaviour.
  - Purpose: Ensure developers/operators can validate the finished pipeline and understand usage constraints.
  - _Leverage: SD_card_contents/anyka_hack/scripts/, README.md, cross-compile/onvif/docs/_
  - _Requirements: Requirement 3, Requirement 4, Requirement 5, Non-Functional Reliability, Non-Functional Usability_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded integration engineer | Task: Extend SD-card scripts and documentation to verify RTSP handshake, explain audio/video toggles, and note unsupported transports | Restrictions: Keep scripts POSIX-compliant, avoid duplicating existing docs, ensure instructions match current config defaults | _Leverage: SD_card_contents/anyka_hack/scripts/, README.md, cross-compile/onvif/docs/_ | _Requirements: Requirement 3, Requirement 4, Requirement 5, Non-Functional Reliability, Non-Functional Usability_ | Success: Script runs in SD-card environment, documentation clarified, lint/format for docs passes; update task state to [-]/[x] accordingly._

- [ ] 9. Normalize RTSP error handling and return codes
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_rtp.c, cross-compile/onvif/src/networking/rtsp/rtsp_session.c, cross-compile/onvif/src/networking/rtsp/rtsp_types.h
  - Replace magic number returns with project error constants, route encoder setup failures through unified helper, and reuse session timeout utilities.
  - Purpose: Align RTSP server error paths with project coding standards and simplify future transport controller integration.
  - _Leverage: cross-compile/onvif/src/networking/rtsp/rtsp_types.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.c, cross-compile/onvif/src/utils/error/error_handling.h_
  - _Requirements: Requirement 1, Requirement 3, Requirement 5_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP server maintainer | Task: Update rtsp_rtp/rtsp_session code to return RTSP error constants instead of raw integers, share timeout logic, and prepare for pipeline controller hooks | Restrictions: Maintain existing threading primitives, keep logging via service logging helpers, avoid duplicating platform calls | _Leverage: cross-compile/onvif/src/networking/rtsp/rtsp_types.h, cross-compile/onvif/src/networking/rtsp/rtsp_session.c, cross-compile/onvif/src/utils/error/error_handling.h_ | _Requirements: Requirement 1, Requirement 3, Requirement 5_ | Success: All RTSP functions use constants, compile cleanly, and unit tests adjusted; remember [-]/[x] task status updates._

- [ ] 10. Refactor platform encoder setup utilities and expose stream descriptors
  - Files: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/platform/platform_anyka.h (if required)
  - Extract shared config clamping/mapping helpers, wrap vendor enums, and expose reusable stream descriptor/creation helpers used by Media and RTSP layers.
  - Purpose: Reduce duplication between platform calls and prepare cleaner interfaces for capability helper and stream router.
  - _Leverage: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/platform/platform.h_
  - _Requirements: Requirement 1, Requirement 5_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Platform abstraction engineer | Task: Introduce static helpers that clamp video/audio configs, wrap vendor enums, and provide a public `platform_venc_apply_stream_config` used by Media and RTSP callers | Restrictions: Preserve g_-prefixed globals, keep include ordering, ensure new helpers documented with Doxygen | _Leverage: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/platform/platform.h_ | _Requirements: Requirement 1, Requirement 5_ | Success: Platform layer compiles, lint/format pass, RTSP/Media callers updated to reuse helpers; mark [-]/[x] appropriately._

- [ ] 11. Synchronize audio configuration with capability data
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_rtp.c, cross-compile/onvif/src/services/media/onvif_media_capabilities.h, cross-compile/onvif/src/services/media/onvif_media_capabilities.c
  - Feed audio sample rate/codec selections from the capability helper into RTSP encoder setup and SDP generation to remove hard-coded defaults.
  - Purpose: Keep audio pipeline, SDP, and ONVIF responses consistent without duplicate constants.
  - _Leverage: cross-compile/onvif/src/networking/rtsp/rtsp_rtp.c, cross-compile/onvif/src/networking/rtsp/rtsp_sdp.c, cross-compile/onvif/src/services/media/onvif_media_capabilities.h_
  - _Requirements: Requirement 1, Requirement 2_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media pipeline integrator | Task: Update audio encoder setup and SDP code to consume capability helper outputs, eliminating duplicated AAC/G.711 literals | Restrictions: Follow include ordering, maintain existing logging tone, ensure null-handling is robust | _Leverage: cross-compile/onvif/src/networking/rtsp/rtsp_rtp.c, cross-compile/onvif/src/networking/rtsp/rtsp_sdp.c, cross-compile/onvif/src/services/media/onvif_media_capabilities.h_ | _Requirements: Requirement 1, Requirement 2_ | Success: Audio setup pulls values from helper, SDP matches runtime config, unit tests updated; set tasks to [-]/[x] as progress is made._

- [ ] 12. Centralize randomness and logging utilities for RTSP sessions
  - Files: cross-compile/onvif/src/networking/rtsp/rtsp_rtp.c, cross-compile/onvif/src/utils/security/random_utils.h, cross-compile/onvif/src/utils/logging/service_logging.h
  - Replace per-session `srand/rand` seeding with project random utility and align RTSP logging with shared service logging levels to avoid repetitive debug spam.
  - Purpose: Improve determinism for tests and align logging with service logging guidelines.
  - _Leverage: cross-compile/onvif/src/utils/security/random_utils.h, cross-compile/onvif/src/utils/logging/service_logging.h_
  - _Requirements: Requirement 3, Non-Functional Reliability_
  - _Prompt: Implement the task for spec video-audio-rtsp-finishing, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded reliability engineer | Task: Swap direct srand/rand usage for the random utility, adjust RTSP log levels to the shared logging helpers, and ensure no verbose spam remains | Restrictions: Do not introduce new globals, maintain thread safety, keep logging ASCII only | _Leverage: cross-compile/onvif/src/utils/security/random_utils.h, cross-compile/onvif/src/utils/logging/service_logging.h_ | _Requirements: Requirement 3, Non-Functional Reliability_ | Success: Randomness and logging changes compile, unit tests deterministic, lint clean; remember [-]/[x] updates._
