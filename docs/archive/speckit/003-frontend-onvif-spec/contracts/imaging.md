# API Contract: Imaging Service

**Service Endpoint**: `POST /onvif/imaging_service`

All operations use SOAP 1.2 with WS-UsernameToken authentication.

---

## GetImagingSettings

Retrieves current imaging parameters.

**ONVIF Action**: `http://www.onvif.org/ver20/imaging/wsdl/GetImagingSettings`

### Request

```xml
<timg:GetImagingSettings>
  <timg:VideoSourceToken>string</timg:VideoSourceToken>
</timg:GetImagingSettings>
```

### Response

```xml
<timg:GetImagingSettingsResponse>
  <timg:ImagingSettings>
    <tt:Brightness>float</tt:Brightness>
    <tt:Contrast>float</tt:Contrast>
    <tt:ColorSaturation>float</tt:ColorSaturation>
    <tt:Sharpness>float</tt:Sharpness>
  </timg:ImagingSettings>
</timg:GetImagingSettingsResponse>
```

### Frontend Mapping

```typescript
interface ImagingSettings {
  brightness: number;   // 0.0 - 1.0
  contrast: number;     // 0.0 - 1.0
  saturation: number;   // 0.0 - 1.0
  sharpness: number;    // 0.0 - 1.0
}
```

---

## SetImagingSettings

Updates imaging parameters.

**ONVIF Action**: `http://www.onvif.org/ver20/imaging/wsdl/SetImagingSettings`

### Request

```xml
<timg:SetImagingSettings>
  <timg:VideoSourceToken>string</timg:VideoSourceToken>
  <timg:ImagingSettings>
    <tt:Brightness>float</tt:Brightness>
    <tt:Contrast>float</tt:Contrast>
    <tt:ColorSaturation>float</tt:ColorSaturation>
    <tt:Sharpness>float</tt:Sharpness>
  </timg:ImagingSettings>
</timg:SetImagingSettings>
```

### Response

```xml
<timg:SetImagingSettingsResponse />
```

### Validation

Values must be within ranges returned by GetOptions.

---

## GetOptions

Retrieves valid ranges for imaging settings.

**ONVIF Action**: `http://www.onvif.org/ver20/imaging/wsdl/GetOptions`

### Request

```xml
<timg:GetOptions>
  <timg:VideoSourceToken>string</timg:VideoSourceToken>
</timg:GetOptions>
```

### Response

```xml
<timg:GetOptionsResponse>
  <timg:ImagingOptions>
    <tt:Brightness>
      <tt:Min>float</tt:Min>
      <tt:Max>float</tt:Max>
    </tt:Brightness>
    <tt:Contrast>
      <tt:Min>float</tt:Min>
      <tt:Max>float</tt:Max>
    </tt:Contrast>
    <tt:ColorSaturation>
      <tt:Min>float</tt:Min>
      <tt:Max>float</tt:Max>
    </tt:ColorSaturation>
    <tt:Sharpness>
      <tt:Min>float</tt:Min>
      <tt:Max>float</tt:Max>
    </tt:Sharpness>
  </timg:ImagingOptions>
</timg:GetOptionsResponse>
```

### Frontend Mapping

```typescript
interface ImagingOptions {
  brightness: { min: number; max: number };
  contrast: { min: number; max: number };
  saturation: { min: number; max: number };
  sharpness: { min: number; max: number };
}
```

---

## Frontend Service Implementation

```typescript
// services/imagingService.ts

const DEFAULT_VIDEO_SOURCE = 'VideoSourceToken';

export const imagingService = {
  async getSettings(): Promise<ImagingSettings> {
    const response = await onvifClient.sendSOAPRequest(
      'imaging_service',
      'GetImagingSettings',
      {
        'timg:GetImagingSettings': {
          'timg:VideoSourceToken': DEFAULT_VIDEO_SOURCE
        }
      }
    );
    return parseImagingSettings(response);
  },

  async setSettings(settings: ImagingSettings): Promise<void> {
    await onvifClient.sendSOAPRequest(
      'imaging_service',
      'SetImagingSettings',
      {
        'timg:SetImagingSettings': {
          'timg:VideoSourceToken': DEFAULT_VIDEO_SOURCE,
          'timg:ImagingSettings': {
            'tt:Brightness': settings.brightness,
            'tt:Contrast': settings.contrast,
            'tt:ColorSaturation': settings.saturation,
            'tt:Sharpness': settings.sharpness
          }
        }
      }
    );
  },

  async getOptions(): Promise<ImagingOptions> {
    const response = await onvifClient.sendSOAPRequest(
      'imaging_service',
      'GetOptions',
      {
        'timg:GetOptions': {
          'timg:VideoSourceToken': DEFAULT_VIDEO_SOURCE
        }
      }
    );
    return parseImagingOptions(response);
  }
};
```

---

## UI Component Integration

```typescript
// pages/settings/ImagingPage.tsx

const ImagingPage = () => {
  const [settings, setSettings] = useState<ImagingSettings | null>(null);
  const [options, setOptions] = useState<ImagingOptions | null>(null);

  useEffect(() => {
    Promise.all([
      imagingService.getSettings(),
      imagingService.getOptions()
    ]).then(([s, o]) => {
      setSettings(s);
      setOptions(o);
    });
  }, []);

  const handleSliderChange = (name: keyof ImagingSettings, value: number) => {
    setSettings(prev => ({ ...prev!, [name]: value }));
  };

  const handleSave = async () => {
    if (!settings) return;
    await imagingService.setSettings(settings);
    showToast({ type: 'success', message: 'Imaging settings saved' });
  };

  return (
    <SettingsCard title="Imaging Settings">
      {options && settings && (
        <>
          <Slider
            label="Brightness"
            value={settings.brightness}
            min={options.brightness.min}
            max={options.brightness.max}
            onChange={(v) => handleSliderChange('brightness', v)}
          />
          {/* Similar for contrast, saturation, sharpness */}
        </>
      )}
      <Button onClick={handleSave}>Save Changes</Button>
    </SettingsCard>
  );
};
```
