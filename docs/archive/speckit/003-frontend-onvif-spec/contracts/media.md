# API Contract: Media Service (Profiles)

**Service Endpoint**: `POST /onvif/media_service`

All operations use SOAP 1.2 with WS-UsernameToken authentication.

---

## GetProfiles

Retrieves list of media profiles.

**ONVIF Action**: `http://www.onvif.org/ver10/media/wsdl/GetProfiles`

### GetProfiles Request

```xml
<trt:GetProfiles />
```

### GetProfiles Response

```xml
<trt:GetProfilesResponse>
  <trt:Profiles token="string" fixed="boolean">
    <tt:Name>string</tt:Name>
    <tt:VideoSourceConfiguration token="string">
      <tt:Name>string</tt:Name>
      <tt:SourceToken>string</tt:SourceToken>
    </tt:VideoSourceConfiguration>
    <tt:VideoEncoderConfiguration token="string">
      <tt:Name>string</tt:Name>
      <tt:Encoding>H264|JPEG|H265</tt:Encoding>
      <tt:Resolution>
        <tt:Width>int</tt:Width>
        <tt:Height>int</tt:Height>
      </tt:Resolution>
      <tt:RateControl>
        <tt:FrameRateLimit>int</tt:FrameRateLimit>
        <tt:BitrateLimit>int</tt:BitrateLimit>
      </tt:RateControl>
    </tt:VideoEncoderConfiguration>
  </trt:Profiles>
  <!-- Multiple Profiles elements -->
</trt:GetProfilesResponse>
```

### Frontend Mapping

```typescript
interface Profile {
  token: string;
  name: string;
  fixed: boolean;
  videoEncoder?: {
    encoding: 'H264' | 'H265' | 'JPEG';
    resolution: { width: number; height: number };
    frameRate: number;
    bitrate: number;
  };
}
```

---

## CreateProfile

Creates a new media profile.

**ONVIF Action**: `http://www.onvif.org/ver10/media/wsdl/CreateProfile`

### CreateProfile Request

```xml
<trt:CreateProfile>
  <trt:Name>string</trt:Name>
  <trt:Token>string</trt:Token><!-- optional -->
</trt:CreateProfile>
```

### CreateProfile Response

```xml
<trt:CreateProfileResponse>
  <trt:Profile token="string" fixed="false">
    <tt:Name>string</tt:Name>
  </trt:Profile>
</trt:CreateProfileResponse>
```

---

## DeleteProfile

Deletes a media profile.

**ONVIF Action**: `http://www.onvif.org/ver10/media/wsdl/DeleteProfile`

### DeleteProfile Request

```xml
<trt:DeleteProfile>
  <trt:ProfileToken>string</trt:ProfileToken>
</trt:DeleteProfile>
```

### DeleteProfile Response

```xml
<trt:DeleteProfileResponse />
```

### Constraints

- Cannot delete fixed profiles (system-defined)
- At least one profile must remain

---

## GetVideoEncoderConfigurations

Retrieves available video encoder configurations.

**ONVIF Action**: `http://www.onvif.org/ver10/media/wsdl/GetVideoEncoderConfigurations`

### GetVideoEncoderConfigurations Request

```xml
<trt:GetVideoEncoderConfigurations />
```

### GetVideoEncoderConfigurations Response

```xml
<trt:GetVideoEncoderConfigurationsResponse>
  <trt:Configurations token="string">
    <tt:Name>string</tt:Name>
    <tt:Encoding>H264|JPEG|H265</tt:Encoding>
    <tt:Resolution>
      <tt:Width>int</tt:Width>
      <tt:Height>int</tt:Height>
    </tt:Resolution>
    <tt:Quality>float</tt:Quality>
    <tt:RateControl>
      <tt:FrameRateLimit>int</tt:FrameRateLimit>
      <tt:BitrateLimit>int</tt:BitrateLimit>
    </tt:RateControl>
  </trt:Configurations>
</trt:GetVideoEncoderConfigurationsResponse>
```

---

## Frontend Service Implementation

```typescript
// services/profileService.ts

export const profileService = {
  async getProfiles(): Promise<Profile[]> {
    const response = await onvifClient.sendSOAPRequest(
      'media_service',
      'GetProfiles',
      { 'trt:GetProfiles': {} }
    );
    return parseProfiles(response);
  },

  async createProfile(name: string): Promise<Profile> {
    const response = await onvifClient.sendSOAPRequest(
      'media_service',
      'CreateProfile',
      {
        'trt:CreateProfile': {
          'trt:Name': name
        }
      }
    );
    return parseProfile(response);
  },

  async deleteProfile(token: string): Promise<void> {
    await onvifClient.sendSOAPRequest(
      'media_service',
      'DeleteProfile',
      {
        'trt:DeleteProfile': {
          'trt:ProfileToken': token
        }
      }
    );
  },

  async getVideoEncoderConfigs(): Promise<VideoEncoderConfig[]> {
    const response = await onvifClient.sendSOAPRequest(
      'media_service',
      'GetVideoEncoderConfigurations',
      { 'trt:GetVideoEncoderConfigurations': {} }
    );
    return parseVideoEncoderConfigs(response);
  }
};
```

---

## UI Component Integration

```typescript
// pages/settings/ProfilesPage.tsx

const ProfilesPage = () => {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);

  useEffect(() => {
    loadProfiles();
  }, []);

  const loadProfiles = async () => {
    const data = await profileService.getProfiles();
    setProfiles(data);
  };

  const handleCreate = async (name: string) => {
    await profileService.createProfile(name);
    await loadProfiles();
    setShowCreateModal(false);
    showToast({ type: 'success', message: 'Profile created' });
  };

  const handleDelete = async (token: string) => {
    await profileService.deleteProfile(token);
    await loadProfiles();
    showToast({ type: 'success', message: 'Profile deleted' });
  };

  return (
    <SettingsCard title="Profiles">
      <ProfileList
        profiles={profiles}
        onDelete={handleDelete}
      />
      <Button onClick={() => setShowCreateModal(true)}>
        Add Profile
      </Button>
      {showCreateModal && (
        <CreateProfileModal
          onSubmit={handleCreate}
          onClose={() => setShowCreateModal(false)}
        />
      )}
    </SettingsCard>
  );
};
```
