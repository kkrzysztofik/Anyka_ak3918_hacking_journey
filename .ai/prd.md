# Product Requirements Document (PRD) - Anyka AK3918 Camera Firmware

## 1. Product Overview

The Anyka AK3918 Camera Firmware project delivers an open, ONVIF 24.12 compliant firmware for Anyka AK3918-based IP cameras. The product converts consumer-grade hardware into professional surveillance devices by replacing vendor-locked firmware with a standards-based solution that integrates seamlessly with enterprise security ecosystems. The firmware ships with an SD-card development workflow, enabling safe deployment and rapid iteration, and includes real-time streaming, PTZ control, imaging adjustments, and network discovery features. The roadmap follows phased milestones (feature-complete, testing, certification) with buffer time, leverages a gSOAP-based stack, and depends on the bundled Anyka toolchain.

## 2. User Problem

Security system integrators, IoT developers, researchers, open source enthusiasts, and educational institutions struggle with proprietary firmware that limits interoperability, customization, transparency, and security. Existing Anyka-based cameras lack ONVIF compliance, offer minimal documentation, expose unknown vulnerabilities, and require costly hardware upgrades to achieve professional functionality. Users need an auditable, standards-compliant firmware that enables integration with ONVIF tooling, provides robust control interfaces, adheres to security best practices, and maintains compatibility with existing camera hardware without invasive flashing procedures.

## 3. Functional Requirements

FR-01: Provide complete ONVIF 24.12 Device, Media, PTZ, and Imaging service coverage with Profile S compliance and Profile T capabilities matching hardware limits.
FR-02: Implement RTSP streaming with H.264 720p video, configurable bitrate, and streaming stability validated through VLC.
FR-03: Deliver full PTZ feature set including continuous move, presets, absolute/relative positioning, and status reporting.
FR-04: Expose imaging controls (brightness, contrast, saturation, sharpness) via ONVIF Imaging service with safe input validation.
FR-05: Support WS-Discovery to advertise services and respond to network discovery probes.
FR-06: Provide secure authentication, authorization hooks, and hardened input validation aligned with Security First principle, with final scope determined at MVP security checkpoint.
FR-07: Offer SD-card boot workflow that loads firmware without modifying onboard flash and includes rollback safeguards.
FR-08: Supply a web-based configuration and live-view interface covering device setup, stream preview, PTZ control, and imaging adjustments.
FR-09: Instrument runtime monitoring for camera health, network availability, RTSP session status, PTZ state, security events, and resource utilization.
FR-10: Maintain thorough documentation, including quickstart guides, ONVIF certification appendix, and release notes synchronized with @testspecs tracking.
FR-11: Enable developers to build using bundled Anyka toolchain and gSOAP dependencies, with reproducible cross-compilation environment.
FR-12: Pass mandatory unit tests, integration tests on physical cameras, and ONVIF conformance validation prior to release.
FR-13: Collect performance metrics (latency, bitrate, error rate, concurrent clients, CPU, memory) during dedicated measurement sprint post feature-lock.

## 4. Product Boundaries

In scope: ONVIF Device/Media/PTZ/Imaging services, SD-card based deployment, web configuration and live view, WS-Discovery, RTSP streaming, baseline monitoring dashboard, physical camera end-to-end testing, ONVIF certification appendix maintenance.
Out of scope for MVP: Diagnostics UI beyond core monitoring signals, expanded security features beyond checkpoint-approved scope, advanced analytics or AI features, non-Anyka hardware support, full licensing review (deferred per CONTRIBUTING.md).
Assumptions: Availability of Anyka AK3918 hardware for testing, stable SD-card interface, bundled toolchain and gSOAP present, network access for ONVIF discovery, developers familiar with ONVIF workflows.
Dependencies: Anyka SDK components, gSOAP stack, VLC for validation, physical lab environment for measurement sprint, @testspecs alignment for certification artifacts.

## 5. User Stories

US-001
Title: Discover Camera on Network
Description: As a security system integrator, I want the camera to announce itself via WS-Discovery so that my ONVIF management software can auto-detect it.
Acceptance Criteria: Device responds to Probe messages within 2 seconds; ONVIF client can retrieve device capabilities; discovery events logged with timestamp and requester IP.

US-002
Title: Configure Network Settings
Description: As an integrator, I want to configure network parameters through the web UI so that the camera fits my infrastructure.
Acceptance Criteria: Web UI allows setting IP/DNS/gateway; validation prevents invalid CIDR and gateway mismatch; configuration changes persist across reboots via SD-card overlay.

US-003
Title: Authenticate to Camera
Description: As an administrator, I need secure login to the web UI and ONVIF services so that unauthorized users cannot access camera controls.
Acceptance Criteria: Authentication uses credential store with hashing; failed attempts generate security logs; ONVIF requests without valid credentials are rejected with appropriate fault codes.

US-004
Title: View Live Stream
Description: As an integrator, I want to view the camera’s live RTSP stream at 720p so that I can verify installation quality.
Acceptance Criteria: RTSP URL returned by ONVIF Media service streams H.264 at configured bitrate; VLC playback remains stable for 30 minutes without frame drops exceeding 1%.

US-005
Title: Control PTZ Movements
Description: As an operator, I want to send PTZ commands through ONVIF so that I can position the camera accurately.
Acceptance Criteria: ContinuousMove, RelativeMove, AbsoluteMove, SetPreset, and GotoPreset operations succeed with device feedback; invalid inputs return ONVIF error codes without moving hardware; movement logs record start/stop timestamps and target coordinates.

US-006
Title: Adjust Imaging Settings
Description: As an operator, I want to adjust brightness, contrast, and saturation over ONVIF Imaging so that the video quality suits lighting conditions.
Acceptance Criteria: Imaging service exposes supported ranges from device capabilities; adjustments take effect within 500 ms; out-of-range inputs return ONVIF invalid parameter errors; settings persist after reboot.

US-007
Title: Monitor Camera Health
Description: As a support engineer, I need a dashboard with camera status and resource metrics so that I can detect issues proactively.
Acceptance Criteria: Dashboard shows connection status, CPU, memory, stream bitrate, PTZ state, and authentication events updated at least every 10 seconds; export of logs and configuration available in CSV/JSON; read-only access mode provided for stakeholders.

US-008
Title: Deploy Firmware Safely
Description: As a developer, I want to boot custom firmware from an SD card without flashing the camera so that I can test changes safely.
Acceptance Criteria: SD-card payload initiates firmware without modifying internal flash; rollback to stock firmware achieved by removing SD card; deployment logs copy to SD-card for troubleshooting; configuration quickstart instructions included.

US-009
Title: Run Automated Tests
Description: As a release manager, I need automated unit and integration tests to run as part of CI so that regressions are detected before release.
Acceptance Criteria: make test passes with 100% success; physical camera end-to-end tests execute nightly with report artifacts; certification appendix updated automatically when test specs change.

US-010
Title: Measure Performance Metrics
Description: As a performance engineer, I want to record latency, bitrate, error rate, connected clients, and resource usage during load so that I can validate performance goals.
Acceptance Criteria: Measurement sprint tooling gathers metrics under defined workloads; results stored with timestamp and firmware version; alerts raised if latency exceeds 1 second or error rate >0.1%; findings feed into release readiness review.

US-011
Title: Maintain Documentation
Description: As a contributor, I need up-to-date onboarding materials so that I can develop features quickly.
Acceptance Criteria: Quickstart guide updated each release with SD-card workflow and toolchain setup; ONVIF certification appendix matches @testspecs; change log records new features and breaking changes.

US-012
Title: Record Security Events
Description: As a security analyst, I want to review authentication attempts and suspicious activity so that I can detect intrusions.
Acceptance Criteria: All login attempts logged with timestamp, IP, and outcome; repeated failures trigger alert flag in dashboard; logs exportable for SIEM ingestion; log retention configurable via web UI.

US-013
Title: Validate ONVIF Compliance
Description: As a compliance officer, I want the firmware to pass ONVIF conformance tests so that we can deploy in regulated environments.
Acceptance Criteria: Firmware passes official ONVIF test suite for Device, Media, PTZ, Imaging services; certification appendix updated with test results and deviations; blocking issues resolved before release candidate sign-off.

US-014
Title: Handle Network Loss Gracefully
Description: As an operator, I want the camera to recover from temporary network outages without manual intervention.
Acceptance Criteria: Firmware retries service announcements after connectivity loss; RTSP sessions resume within 5 seconds after network restoration; dashboard indicates recovery events; no data corruption occurs during outage.

## 6. Success Metrics

SM-01 ONVIF Compliance: 100% pass rate on ONVIF 24.12 Profile S test suite with documented Profile T capabilities.
SM-02 Security: Zero critical vulnerabilities identified in penetration testing prior to release; security event coverage logged for all authentication attempts.
SM-03 Performance: Sub-second (<1s) response time for ONVIF commands under nominal load; sustained 720p stream with bitrate variance under 5%.
SM-04 Reliability: 99.9% uptime measured over 30-day soak tests with automated recovery from network interruptions.
SM-05 Adoption: Growth in active community contributors and integrators measured by monthly commits, forum activity, and firmware downloads.
SM-06 Documentation: Quickstart guide and certification appendix updated every release with ≤2 outstanding documentation issues at release review.
