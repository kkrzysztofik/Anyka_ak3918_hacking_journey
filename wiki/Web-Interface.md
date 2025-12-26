# Web Interface

The project includes two web interfaces with clear separation and easy switching between them. The web interfaces communicate with the ONVIF server (Rust implementation) via HTTP/SOAP requests.

## Web Interface Structure

```text
www/cgi-bin/
├── header                    # Common header for all interfaces
├── footer                    # Common footer for all interfaces
├── webui_onvif              # Main ONVIF interface (recommended)
├── onvif_imaging            # ONVIF imaging controls
├── onvif_presets            # ONVIF PTZ preset management
└── legacy/                  # Legacy web UI implementation
    ├── webui                # Original web interface
    ├── events               # Event management
    ├── login                # Login page
    ├── login_validate.sh    # Login validation
    ├── settings             # Settings page
    ├── settings_submit.sh   # Settings submission
    ├── system               # System information
    ├── video                # Video playback
    ├── del_video.sh         # Video deletion
    └── pwd_change           # Password change
```

## Interface Comparison

### ONVIF Interface (Recommended)

- **Location**: `/cgi-bin/webui_onvif`
- **Features**:
  - ONVIF protocol compliance
  - Advanced PTZ controls
  - Imaging parameter adjustment
  - PTZ preset management
  - Real-time service status
  - Better integration with surveillance software

### Legacy Interface

- **Location**: `/cgi-bin/legacy/webui`
- **Features**:
  - Original ptz_daemon integration
  - libre_anyka_app integration
  - Basic PTZ controls
  - Event management
  - Settings configuration

## Navigation

### Main Entry Point

- **URL**: `http://[CAMERA_IP]/`
- **Behavior**: Redirects to ONVIF interface by default
- **Options**: Provides links to both interfaces

### ONVIF Interface Navigation

- Home → Imaging → Presets → Settings → System → Events
- Includes link to Legacy Interface

### Legacy Interface Navigation

- Home → Settings → System → Events
- Includes link to ONVIF Interface

## Access URLs

### ONVIF Interface

- Main: `http://[CAMERA_IP]/cgi-bin/webui_onvif`
- Imaging: `http://[CAMERA_IP]/cgi-bin/onvif_imaging`
- Presets: `http://[CAMERA_IP]/cgi-bin/onvif_presets`

### Legacy Interface URLs

- Main: `http://[CAMERA_IP]/cgi-bin/legacy/webui`
- Events: `http://[CAMERA_IP]/cgi-bin/legacy/events`
- Settings: `http://[CAMERA_IP]/cgi-bin/legacy/settings`
- System: `http://[CAMERA_IP]/cgi-bin/legacy/system`

## See Also

- [[ONVIF-Rust-Implementation]] - ONVIF server that the web interface communicates with
- [[Development-Guide]] - Web interface development instructions
- [[Legacy-Applications]] - Legacy web interface details
