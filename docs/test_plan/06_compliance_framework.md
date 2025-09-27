# ONVIF Compliance Validation Framework

## Overview

Comprehensive ONVIF Profile S and Profile T compliance validation framework ensuring actual specification compliance, not just test passing.

## ONVIF Profile Comparison

| Aspect | Profile S | Profile T |
|--------|-----------|-----------|
| **Mandatory Features** | 13 features | 22 features |
| **Technical Specification** | 1.02+ | 18.06+ |
| **Core Focus** | Basic streaming | Advanced streaming + metadata |
| **Key Requirements** | Device, Media services | + PTZ, Imaging, Events |

## Profile S Compliance Framework

### Mandatory Features Validation

Create `tests/compliance/test_profile_s_compliance.py`:

```python
"""ONVIF Profile S compliance validation tests."""

import pytest
from utils.onvif_validator import ONVIFProfileSValidator
from utils.compliance_checker import ComplianceResult

class TestProfileSCompliance:
    """Validate complete ONVIF Profile S compliance."""

    @pytest.mark.compliance
    @pytest.mark.profile_s_compliance
    @pytest.mark.critical
    def test_mandatory_device_service_compliance(self, device_client):
        """Test Profile S mandatory Device service compliance."""
        validator = ONVIFProfileSValidator(device_client)

        # Test 7.1: User Authentication
        auth_result = validator.validate_user_authentication()
        assert auth_result.is_compliant, f"User Authentication: {auth_result.issues}"

        # Test 7.2: Capabilities
        caps_result = validator.validate_capabilities()
        assert caps_result.is_compliant, f"Capabilities: {caps_result.issues}"

        # Test 7.3: Discovery (WS-Discovery)
        discovery_result = validator.validate_discovery()
        assert discovery_result.is_compliant, f"Discovery: {discovery_result.issues}"

        # Test 7.4: Network Configuration
        network_result = validator.validate_network_configuration()
        assert network_result.is_compliant, f"Network Config: {network_result.issues}"

        # Test 7.5: System (Date/Time)
        system_result = validator.validate_system_datetime()
        assert system_result.is_compliant, f"System DateTime: {system_result.issues}"

    @pytest.mark.compliance
    @pytest.mark.profile_s_compliance
    def test_mandatory_media_service_compliance(self, device_client):
        """Test Profile S mandatory Media service compliance."""
        validator = ONVIFProfileSValidator(device_client)

        # Test 7.7: Streaming
        streaming_result = validator.validate_streaming()
        assert streaming_result.is_compliant, f"Streaming: {streaming_result.issues}"

        # Test 7.8: Video Streaming
        video_result = validator.validate_video_streaming()
        assert video_result.is_compliant, f"Video Streaming: {video_result.issues}"

        # Test 7.9: Audio Streaming (if supported)
        audio_result = validator.validate_audio_streaming()
        if audio_result.applicable:
            assert audio_result.is_compliant, f"Audio Streaming: {audio_result.issues}"

        # Test 7.10: Metadata Streaming (if supported)
        metadata_result = validator.validate_metadata_streaming()
        if metadata_result.applicable:
            assert metadata_result.is_compliant, f"Metadata: {metadata_result.issues}"

    @pytest.mark.compliance
    @pytest.mark.profile_s_compliance
    def test_conditional_features_compliance(self, device_client):
        """Test Profile S conditional features if supported."""
        validator = ONVIFProfileSValidator(device_client)

        conditional_features = [
            ("PTZ", validator.validate_ptz_conditional),
            ("Event Handling", validator.validate_event_handling),
            ("Security", validator.validate_security_features),
            ("Relay Output", validator.validate_relay_output),
            ("Digital Input", validator.validate_digital_input),
        ]

        results = {}
        for feature_name, validation_func in conditional_features:
            result = validation_func()
            results[feature_name] = result

            if result.applicable and not result.is_compliant:
                pytest.fail(f"Conditional feature {feature_name} non-compliant: {result.issues}")

        # Log conditional feature support
        for feature, result in results.items():
            status = "Supported & Compliant" if result.applicable and result.is_compliant else \
                    "Supported but Non-Compliant" if result.applicable else "Not Supported"
            print(f"{feature}: {status}")
```

### Profile T Compliance Framework

Create `tests/compliance/test_profile_t_compliance.py`:

```python
"""ONVIF Profile T compliance validation tests."""

import pytest
from utils.onvif_validator import ONVIFProfileTValidator

class TestProfileTCompliance:
    """Validate complete ONVIF Profile T compliance."""

    @pytest.mark.compliance
    @pytest.mark.profile_t_compliance
    @pytest.mark.critical
    def test_enhanced_video_streaming(self, device_client):
        """Test Profile T enhanced video streaming requirements."""
        validator = ONVIFProfileTValidator(device_client)

        # H.264 streaming mandatory in Profile T
        h264_result = validator.validate_h264_streaming()
        assert h264_result.is_compliant, f"H.264 Streaming: {h264_result.issues}"

        # Advanced video profiles
        profile_result = validator.validate_video_profiles()
        assert profile_result.is_compliant, f"Video Profiles: {profile_result.issues}"

        # Video encoder configuration
        encoder_result = validator.validate_video_encoder_config()
        assert encoder_result.is_compliant, f"Video Encoder: {encoder_result.issues}"

    @pytest.mark.compliance
    @pytest.mark.profile_t_compliance
    def test_mandatory_ptz_compliance(self, device_client):
        """Test Profile T mandatory PTZ requirements."""
        validator = ONVIFProfileTValidator(device_client)

        # PTZ is mandatory in Profile T
        ptz_result = validator.validate_ptz_mandatory()
        assert ptz_result.is_compliant, f"PTZ Mandatory: {ptz_result.issues}"

        # PTZ configuration
        ptz_config_result = validator.validate_ptz_configuration()
        assert ptz_config_result.is_compliant, f"PTZ Config: {ptz_config_result.issues}"

    @pytest.mark.compliance
    @pytest.mark.profile_t_compliance
    def test_imaging_service_compliance(self, device_client):
        """Test Profile T imaging service requirements."""
        validator = ONVIFProfileTValidator(device_client)

        # Imaging service mandatory in Profile T
        imaging_result = validator.validate_imaging_service()
        assert imaging_result.is_compliant, f"Imaging Service: {imaging_result.issues}"

        # Image settings
        settings_result = validator.validate_imaging_settings()
        assert settings_result.is_compliant, f"Imaging Settings: {settings_result.issues}"

    @pytest.mark.compliance
    @pytest.mark.profile_t_compliance
    def test_advanced_features_compliance(self, device_client):
        """Test Profile T advanced features."""
        validator = ONVIFProfileTValidator(device_client)

        advanced_features = [
            ("Motion Detection", validator.validate_motion_detection),
            ("OSD Configuration", validator.validate_osd_configuration),
            ("Tampering Detection", validator.validate_tampering_detection),
            ("Enhanced Metadata", validator.validate_enhanced_metadata),
        ]

        for feature_name, validation_func in advanced_features:
            result = validation_func()
            assert result.is_compliant, f"{feature_name}: {result.issues}"
```

## Compliance Validation Utilities

### ONVIF Specification Validator

Create `utils/onvif_validator.py`:

```python
"""ONVIF specification compliance validation utilities."""

import xml.etree.ElementTree as ET
from dataclasses import dataclass
from typing import List, Dict, Any, Optional
from enum import Enum

class ComplianceLevel(Enum):
    """ONVIF compliance levels."""
    MANDATORY = "mandatory"
    CONDITIONAL = "conditional"
    OPTIONAL = "optional"

@dataclass
class ComplianceResult:
    """Result of compliance validation."""
    feature_name: str
    is_compliant: bool
    applicable: bool = True
    level: ComplianceLevel = ComplianceLevel.MANDATORY
    issues: List[str] = None
    warnings: List[str] = None
    recommendations: List[str] = None

    def __post_init__(self):
        if self.issues is None:
            self.issues = []
        if self.warnings is None:
            self.warnings = []
        if self.recommendations is None:
            self.recommendations = []

class ONVIFSpecValidator:
    """Base ONVIF specification validator."""

    def __init__(self, device_client):
        self.device_client = device_client
        self.device_capabilities = None
        self._load_device_capabilities()

    def _load_device_capabilities(self):
        """Load device capabilities for validation."""
        try:
            response = self.device_client.get_capabilities()
            self.device_capabilities = self.device_client.parse_capabilities(response)
        except Exception as e:
            raise ValueError(f"Could not load device capabilities: {e}")

    def validate_soap_response(self, response_xml: str, expected_operation: str) -> ComplianceResult:
        """Validate SOAP response format compliance."""
        result = ComplianceResult("SOAP Response Format", True)

        try:
            root = ET.fromstring(response_xml)

            # Check SOAP envelope
            envelope = root.find('.//{http://www.w3.org/2003/05/soap-envelope}Envelope')
            if envelope is None:
                envelope = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Envelope')

            if envelope is None:
                result.is_compliant = False
                result.issues.append("Missing SOAP Envelope")
                return result

            # Check SOAP body
            body = root.find('.//{http://www.w3.org/2003/05/soap-envelope}Body')
            if body is None:
                body = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Body')

            if body is None:
                result.is_compliant = False
                result.issues.append("Missing SOAP Body")
                return result

            # Check for expected operation response
            expected_response = f"{expected_operation}Response"
            if expected_response not in response_xml:
                result.warnings.append(f"Expected {expected_response} not found")

        except ET.ParseError as e:
            result.is_compliant = False
            result.issues.append(f"Invalid XML format: {e}")

        return result

    def validate_required_fields(self, data: Dict[str, Any],
                                required_fields: List[str]) -> ComplianceResult:
        """Validate required fields are present."""
        result = ComplianceResult("Required Fields", True)

        for field in required_fields:
            if field not in data:
                result.is_compliant = False
                result.issues.append(f"Missing required field: {field}")
            elif not data[field] or str(data[field]).strip() == "":
                result.is_compliant = False
                result.issues.append(f"Empty required field: {field}")

        return result

class ONVIFProfileSValidator(ONVIFSpecValidator):
    """ONVIF Profile S specific compliance validator."""

    def validate_user_authentication(self) -> ComplianceResult:
        """Validate Profile S Section 7.1: User Authentication."""
        result = ComplianceResult("User Authentication", True, level=ComplianceLevel.MANDATORY)

        try:
            # Test with valid credentials
            response = self.device_client.get_device_information()
            if response.status_code != 200:
                result.is_compliant = False
                result.issues.append("Authentication with valid credentials failed")

            # Test with invalid credentials (should fail)
            old_auth = self.device_client.auth
            self.device_client.auth = ('invalid', 'invalid')

            try:
                response = self.device_client.get_device_information()
                if response.status_code == 200:
                    result.warnings.append("Authentication with invalid credentials succeeded - security concern")
            except Exception:
                pass  # Expected to fail

            # Restore valid auth
            self.device_client.auth = old_auth

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"Authentication validation error: {e}")

        return result

    def validate_capabilities(self) -> ComplianceResult:
        """Validate Profile S Section 7.2: Capabilities."""
        result = ComplianceResult("Capabilities", True, level=ComplianceLevel.MANDATORY)

        if not self.device_capabilities:
            result.is_compliant = False
            result.issues.append("Could not retrieve device capabilities")
            return result

        # Check required capability categories
        required_categories = ['Device', 'Media']
        for category in required_categories:
            if category not in self.device_capabilities:
                result.is_compliant = False
                result.issues.append(f"Missing required capability category: {category}")

        # Validate Device capabilities
        if 'Device' in self.device_capabilities:
            device_caps = self.device_capabilities['Device']
            if 'XAddr' not in device_caps:
                result.is_compliant = False
                result.issues.append("Device capabilities missing XAddr")

        # Validate Media capabilities
        if 'Media' in self.device_capabilities:
            media_caps = self.device_capabilities['Media']
            if 'XAddr' not in media_caps:
                result.is_compliant = False
                result.issues.append("Media capabilities missing XAddr")

        return result

    def validate_discovery(self) -> ComplianceResult:
        """Validate Profile S Section 7.3: Discovery (WS-Discovery)."""
        result = ComplianceResult("WS-Discovery", True, level=ComplianceLevel.MANDATORY)

        try:
            from utils.ws_discovery import WSDiscoveryClient

            discovery_client = WSDiscoveryClient()
            discovered_devices = discovery_client.discover(timeout=10)

            # Check if our device is discoverable
            device_found = False
            for device in discovered_devices:
                if self.device_client.device_url in device.get('XAddrs', []):
                    device_found = True
                    break

            if not device_found:
                result.is_compliant = False
                result.issues.append("Device not discoverable via WS-Discovery")

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"WS-Discovery validation error: {e}")

        return result

    def validate_streaming(self) -> ComplianceResult:
        """Validate Profile S streaming capabilities."""
        result = ComplianceResult("Video Streaming", True, level=ComplianceLevel.MANDATORY)

        try:
            # Get media profiles
            response = self.device_client.get_profiles()
            profiles = self.device_client.parse_profiles(response)

            if not profiles:
                result.is_compliant = False
                result.issues.append("No media profiles available")
                return result

            # Test stream URI generation
            for profile in profiles[:1]:  # Test first profile
                stream_uri_response = self.device_client.get_stream_uri(
                    profile['token'], 'RTP-Unicast'
                )

                if stream_uri_response.status_code != 200:
                    result.is_compliant = False
                    result.issues.append(f"Failed to get stream URI for profile {profile['token']}")

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"Streaming validation error: {e}")

        return result

class ONVIFProfileTValidator(ONVIFSpecValidator):
    """ONVIF Profile T specific compliance validator."""

    def validate_h264_streaming(self) -> ComplianceResult:
        """Validate Profile T H.264 streaming requirement."""
        result = ComplianceResult("H.264 Streaming", True, level=ComplianceLevel.MANDATORY)

        try:
            # Get video encoder configurations
            response = self.device_client.get_video_encoder_configurations()
            configurations = self.device_client.parse_video_encoder_configurations(response)

            h264_supported = False
            for config in configurations:
                if config.get('Encoding') == 'H264':
                    h264_supported = True
                    break

            if not h264_supported:
                result.is_compliant = False
                result.issues.append("H.264 encoding not supported (mandatory for Profile T)")

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"H.264 validation error: {e}")

        return result

    def validate_ptz_mandatory(self) -> ComplianceResult:
        """Validate Profile T mandatory PTZ support."""
        result = ComplianceResult("PTZ Support", True, level=ComplianceLevel.MANDATORY)

        # Check PTZ capabilities
        if 'PTZ' not in self.device_capabilities:
            result.is_compliant = False
            result.issues.append("PTZ capabilities not available (mandatory for Profile T)")
            return result

        try:
            # Test basic PTZ operations
            ptz_caps = self.device_capabilities['PTZ']
            if 'XAddr' not in ptz_caps:
                result.is_compliant = False
                result.issues.append("PTZ service XAddr missing")

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"PTZ validation error: {e}")

        return result

    def validate_imaging_service(self) -> ComplianceResult:
        """Validate Profile T imaging service requirement."""
        result = ComplianceResult("Imaging Service", True, level=ComplianceLevel.MANDATORY)

        # Check imaging capabilities
        if 'Imaging' not in self.device_capabilities:
            result.is_compliant = False
            result.issues.append("Imaging capabilities not available (mandatory for Profile T)")
            return result

        try:
            imaging_caps = self.device_capabilities['Imaging']
            if 'XAddr' not in imaging_caps:
                result.is_compliant = False
                result.issues.append("Imaging service XAddr missing")

        except Exception as e:
            result.is_compliant = False
            result.issues.append(f"Imaging service validation error: {e}")

        return result
```

### Compliance Reporting

Create `utils/compliance_reporter.py`:

```python
"""ONVIF compliance reporting utilities."""

import json
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any
from utils.onvif_validator import ComplianceResult, ComplianceLevel

class ComplianceReporter:
    """Generate comprehensive compliance reports."""

    def __init__(self, output_dir: str = "reports/compliance"):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)

    def generate_profile_s_report(self, results: List[ComplianceResult]) -> Path:
        """Generate Profile S compliance report."""
        report_data = self._create_base_report("ONVIF Profile S", results)

        report_file = self.output_dir / f"profile_s_compliance_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        with open(report_file, 'w') as f:
            json.dump(report_data, f, indent=2)

        # Generate HTML report
        html_report = self._generate_html_report(report_data)
        html_file = report_file.with_suffix('.html')

        with open(html_file, 'w') as f:
            f.write(html_report)

        return report_file

    def generate_profile_t_report(self, results: List[ComplianceResult]) -> Path:
        """Generate Profile T compliance report."""
        report_data = self._create_base_report("ONVIF Profile T", results)

        report_file = self.output_dir / f"profile_t_compliance_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        with open(report_file, 'w') as f:
            json.dump(report_data, f, indent=2)

        # Generate HTML report
        html_report = self._generate_html_report(report_data)
        html_file = report_file.with_suffix('.html')

        with open(html_file, 'w') as f:
            f.write(html_report)

        return report_file

    def _create_base_report(self, profile_name: str, results: List[ComplianceResult]) -> Dict[str, Any]:
        """Create base report structure."""
        mandatory_results = [r for r in results if r.level == ComplianceLevel.MANDATORY]
        conditional_results = [r for r in results if r.level == ComplianceLevel.CONDITIONAL]
        optional_results = [r for r in results if r.level == ComplianceLevel.OPTIONAL]

        mandatory_passed = sum(1 for r in mandatory_results if r.is_compliant)
        conditional_passed = sum(1 for r in conditional_results if r.is_compliant and r.applicable)
        conditional_applicable = sum(1 for r in conditional_results if r.applicable)

        overall_compliance = (len(mandatory_results) == mandatory_passed and
                            (conditional_applicable == 0 or conditional_passed == conditional_applicable))

        return {
            'profile': profile_name,
            'generated_at': datetime.now().isoformat(),
            'overall_compliance': overall_compliance,
            'summary': {
                'mandatory_features': {
                    'total': len(mandatory_results),
                    'passed': mandatory_passed,
                    'failed': len(mandatory_results) - mandatory_passed,
                    'compliance_rate': (mandatory_passed / len(mandatory_results)) * 100 if mandatory_results else 0
                },
                'conditional_features': {
                    'total': len(conditional_results),
                    'applicable': conditional_applicable,
                    'passed': conditional_passed,
                    'compliance_rate': (conditional_passed / conditional_applicable) * 100 if conditional_applicable else 0
                },
                'optional_features': {
                    'total': len(optional_results),
                    'passed': sum(1 for r in optional_results if r.is_compliant)
                }
            },
            'detailed_results': [
                {
                    'feature': result.feature_name,
                    'level': result.level.value,
                    'compliant': result.is_compliant,
                    'applicable': result.applicable,
                    'issues': result.issues,
                    'warnings': result.warnings,
                    'recommendations': result.recommendations
                } for result in results
            ]
        }

    def _generate_html_report(self, report_data: Dict[str, Any]) -> str:
        """Generate HTML compliance report."""
        compliance_status = "✅ COMPLIANT" if report_data['overall_compliance'] else "❌ NON-COMPLIANT"
        status_color = "green" if report_data['overall_compliance'] else "red"

        html = f"""
<!DOCTYPE html>
<html>
<head>
    <title>{report_data['profile']} Compliance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f5f5f5; padding: 20px; border-radius: 5px; }}
        .status {{ font-size: 24px; font-weight: bold; color: {status_color}; }}
        .summary {{ margin: 20px 0; }}
        .feature {{ margin: 10px 0; padding: 10px; border: 1px solid #ddd; border-radius: 3px; }}
        .compliant {{ background-color: #d4edda; }}
        .non-compliant {{ background-color: #f8d7da; }}
        .not-applicable {{ background-color: #e2e3e5; }}
        .issues {{ color: #721c24; }}
        .warnings {{ color: #856404; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{report_data['profile']} Compliance Report</h1>
        <div class="status">{compliance_status}</div>
        <p>Generated: {report_data['generated_at']}</p>
    </div>

    <div class="summary">
        <h2>Summary</h2>
        <p><strong>Mandatory Features:</strong> {report_data['summary']['mandatory_features']['passed']}/{report_data['summary']['mandatory_features']['total']} passed ({report_data['summary']['mandatory_features']['compliance_rate']:.1f}%)</p>
        <p><strong>Conditional Features:</strong> {report_data['summary']['conditional_features']['passed']}/{report_data['summary']['conditional_features']['applicable']} passed ({report_data['summary']['conditional_features']['compliance_rate']:.1f}%)</p>
    </div>

    <div class="details">
        <h2>Detailed Results</h2>
"""

        for feature in report_data['detailed_results']:
            if not feature['applicable']:
                css_class = "not-applicable"
                status_text = "Not Applicable"
            elif feature['compliant']:
                css_class = "compliant"
                status_text = "✅ Compliant"
            else:
                css_class = "non-compliant"
                status_text = "❌ Non-Compliant"

            html += f"""
        <div class="feature {css_class}">
            <h3>{feature['feature']} ({feature['level'].upper()}) - {status_text}</h3>
"""

            if feature['issues']:
                html += f"""
            <div class="issues">
                <strong>Issues:</strong>
                <ul>{"".join(f"<li>{issue}</li>" for issue in feature['issues'])}</ul>
            </div>
"""

            if feature['warnings']:
                html += f"""
            <div class="warnings">
                <strong>Warnings:</strong>
                <ul>{"".join(f"<li>{warning}</li>" for warning in feature['warnings'])}</ul>
            </div>
"""

            html += "        </div>\n"

        html += """
    </div>
</body>
</html>"""

        return html
```

## Implementation Checklist

- [ ] Create Profile S compliance validator
- [ ] Create Profile T compliance validator
- [ ] Implement compliance reporting system
- [ ] Create compliance test suites
- [ ] Add validation scripts
- [ ] Generate compliance reports
- [ ] Verify actual ONVIF specification compliance

## Next Steps

After implementing compliance framework, all modules are complete. Return to [00_overview.md](00_overview.md) to review the complete implementation plan.

## Related Documentation

- **Previous**: [05_performance_tests.md](05_performance_tests.md) - Performance testing framework
- **Issues Addressed**: [01_current_issues.md](01_current_issues.md#critical-issues) - Missing ONVIF compliance validation
- **Templates**:
  - [templates/test_template.py](templates/test_template.py) - Compliance test patterns
  - [templates/fixture_template.py](templates/fixture_template.py) - Compliance fixtures
- **Directory Structure**: [02_directory_structure.md](02_directory_structure.md#phase-1-create-new-structure) - `tests/compliance/` location
- **Quality Standards**: [03_code_quality.md](03_code_quality.md) - Compliance test quality requirements
- **Overview**: [00_overview.md](00_overview.md#success-criteria) - Compliance goals and validation

## Implementation Complete

This completes the modular test plan implementation. All modules work together to provide:

1. **Clear Issue Identification** ([01_current_issues.md](01_current_issues.md))
2. **Structured Organization** ([02_directory_structure.md](02_directory_structure.md))
3. **Quality Standards** ([03_code_quality.md](03_code_quality.md))
4. **Test Refactoring** ([04_device_tests.md](04_device_tests.md))
5. **Performance Framework** ([05_performance_tests.md](05_performance_tests.md))
6. **Compliance Validation** (this document)
7. **Working Templates** ([templates/](templates/))

Each module is self-contained and can be processed by agents within context limits while maintaining full cross-reference capabilities.