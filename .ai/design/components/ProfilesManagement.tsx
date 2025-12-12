import { useState } from 'react';
import { 
  Image as ImageIcon, 
  Plus, 
  Edit2, 
  Trash2, 
  ChevronDown, 
  ChevronRight,
  Video,
  Mic,
  Radio,
  Settings as SettingsIcon,
  RefreshCw,
  Copy,
  Maximize2,
  Activity
} from 'lucide-react';
import ProfileFormModal from './ProfileFormModal';

interface VideoSource {
  token: string;
  framerate: number;
  resolution: string;
  brightness: number;
  colorSaturation: number;
  contrast: number;
  sharpness: number;
  exposure: string;
  focus: string;
  irCutFilter: string;
  backlightCompensation: string;
  whiteBalance: string;
  wideDynamicRange: string;
}

interface AudioSource {
  token: string;
  channels: number;
}

interface VideoEncoder {
  name: string;
  token: string;
  encoding: string;
  resolution: string;
  quality: number;
  frameRate: number;
  bitrate: number;
  sessionTimeout: string;
}

interface AudioEncoder {
  name: string;
  token: string;
  encoding: string;
  bitrate: number;
  sampleRate: number;
  sessionTimeout: string;
}

interface Multicast {
  ipAddress: string;
  port: number;
  ttl: number;
  autoStart: boolean;
}

interface PTZConfig {
  name: string;
  token: string;
  panTiltLimits: string;
  zoomLimits: string;
  defaultPtzSpeed: string;
  defaultPtzTimeout: string;
  defaultAbsolutePantTiltPositionSpace: string;
  defaultRelativePanTiltTranslationSpace: string;
  defaultContinuousPanTiltVelocitySpace: string;
  defaultContinuousZoomVelocitySpace: string;
  defaultRelativeZoomTranslationSpace: string;
  node: {
    name: string;
    token: string;
    homeSupported: boolean;
    maximumNumberOfPresets: number;
    auxiliaryCommands: string;
    supportedPtzSpaces: string;
  };
}

interface VideoAnalytics {
  name: string;
  token: string;
  useCount: number;
  modules: {
    cellMotion: string;
    tamperDetect: string;
    motionDetectorRule: string;
    tamperDetectorRule: string;
  };
}

interface Profile {
  id: string;
  name: string;
  token: string;
  fixed: boolean;
  videoSourceConfig: {
    name: string;
    token: string;
    useCount: number;
    bounds: string;
    videoSource: VideoSource;
  };
  audioSourceConfig: {
    name: string;
    token: string;
    useCount: number;
    audioSource: AudioSource;
  };
  videoEncoderConfig: VideoEncoder;
  audioEncoderConfig: AudioEncoder;
  multicast: Multicast;
  ptzConfig: PTZConfig;
  videoAnalytics: VideoAnalytics;
}

export default function ProfilesManagement() {
  const [profiles, setProfiles] = useState<Profile[]>([
    {
      id: '1',
      name: 'mainStream',
      token: '000',
      fixed: true,
      videoSourceConfig: {
        name: 'V_SRC_CFG_000',
        token: 'V_SRC_000',
        useCount: 3,
        bounds: '(0, 0, 3072, 2048)',
        videoSource: {
          token: 'V_SRC_000',
          framerate: 25,
          resolution: '3072x2048',
          brightness: 50,
          colorSaturation: 50,
          contrast: 50,
          sharpness: 10,
          exposure: 'onvif:services:Exposure',
          focus: 'onvif:services:FocusConfiguration',
          irCutFilter: 'auto',
          backlightCompensation: 'level=10, mode=off',
          whiteBalance: 'CbGain=10, CrGain=10, mode=auto',
          wideDynamicRange: 'level=50, mode=auto',
        },
      },
      audioSourceConfig: {
        name: 'A_SRC_CFG_000',
        token: 'A_SRC_000',
        useCount: 2,
        audioSource: {
          token: 'AudioSourceToken',
          channels: 2,
        },
      },
      videoEncoderConfig: {
        name: 'V_ENC_000',
        token: '000',
        encoding: 'h264',
        resolution: '3072x2048',
        quality: 3,
        frameRate: 12,
        bitrate: 1536,
        sessionTimeout: 'PT10S',
      },
      audioEncoderConfig: {
        name: 'A_ENC_000',
        token: 'A_ENC_000',
        encoding: 'g711',
        bitrate: 128,
        sampleRate: 8,
        sessionTimeout: 'PT10S',
      },
      multicast: {
        ipAddress: '239.0.1.0',
        port: 32002,
        ttl: 2,
        autoStart: false,
      },
      ptzConfig: {
        name: 'PTZConfiguration',
        token: 'PTZConfigurationToken',
        panTiltLimits: '(-1, 1)',
        zoomLimits: '(0, 1)',
        defaultPtzSpeed: '1.0',
        defaultPtzTimeout: 'PT10S',
        defaultAbsolutePantTiltPositionSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace',
        defaultRelativePanTiltTranslationSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace',
        defaultContinuousPanTiltVelocitySpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace',
        defaultContinuousZoomVelocitySpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace',
        defaultRelativeZoomTranslationSpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
        node: {
          name: 'PTZNode',
          token: 'PTZNodeToken',
          homeSupported: true,
          maximumNumberOfPresets: 10,
          auxiliaryCommands: 'http://www.onvif.org/ver10/tptz/AuxiliaryCommands/Command1',
          supportedPtzSpaces: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
        },
      },
      videoAnalytics: {
        name: 'VideoAnalytics',
        token: 'VideoAnalyticsToken',
        useCount: 1,
        modules: {
          cellMotion: 'enabled',
          tamperDetect: 'enabled',
          motionDetectorRule: 'rule1',
          tamperDetectorRule: 'rule2',
        },
      },
    },
    {
      id: '2',
      name: 'subStream',
      token: '001',
      fixed: false,
      videoSourceConfig: {
        name: 'V_SRC_CFG_001',
        token: 'V_SRC_001',
        useCount: 2,
        bounds: '(0, 0, 1920, 1080)',
        videoSource: {
          token: 'V_SRC_001',
          framerate: 25,
          resolution: '1920x1080',
          brightness: 50,
          colorSaturation: 50,
          contrast: 50,
          sharpness: 10,
          exposure: 'onvif:services:Exposure',
          focus: 'onvif:services:FocusConfiguration',
          irCutFilter: 'auto',
          backlightCompensation: 'level=10, mode=off',
          whiteBalance: 'CbGain=10, CrGain=10, mode=auto',
          wideDynamicRange: 'level=50, mode=auto',
        },
      },
      audioSourceConfig: {
        name: 'A_SRC_CFG_001',
        token: 'A_SRC_001',
        useCount: 1,
        audioSource: {
          token: 'AudioSourceToken2',
          channels: 1,
        },
      },
      videoEncoderConfig: {
        name: 'V_ENC_001',
        token: '001',
        encoding: 'h264',
        resolution: '1920x1080',
        quality: 4,
        frameRate: 25,
        bitrate: 2048,
        sessionTimeout: 'PT10S',
      },
      audioEncoderConfig: {
        name: 'A_ENC_001',
        token: 'A_ENC_001',
        encoding: 'aac',
        bitrate: 128,
        sampleRate: 16,
        sessionTimeout: 'PT10S',
      },
      multicast: {
        ipAddress: '239.0.1.1',
        port: 32003,
        ttl: 2,
        autoStart: false,
      },
      ptzConfig: {
        name: 'PTZConfiguration',
        token: 'PTZConfigurationToken2',
        panTiltLimits: '(-1, 1)',
        zoomLimits: '(0, 1)',
        defaultPtzSpeed: '1.0',
        defaultPtzTimeout: 'PT10S',
        defaultAbsolutePantTiltPositionSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace',
        defaultRelativePanTiltTranslationSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace',
        defaultContinuousPanTiltVelocitySpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace',
        defaultContinuousZoomVelocitySpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace',
        defaultRelativeZoomTranslationSpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
        node: {
          name: 'PTZNode',
          token: 'PTZNodeToken',
          homeSupported: true,
          maximumNumberOfPresets: 10,
          auxiliaryCommands: 'http://www.onvif.org/ver10/tptz/AuxiliaryCommands/Command1',
          supportedPtzSpaces: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
        },
      },
      videoAnalytics: {
        name: 'VideoAnalytics',
        token: 'VideoAnalyticsToken',
        useCount: 1,
        modules: {
          cellMotion: 'enabled',
          tamperDetect: 'enabled',
          motionDetectorRule: 'rule1',
          tamperDetectorRule: 'rule2',
        },
      },
    },
  ]);

  const [expandedProfiles, setExpandedProfiles] = useState<Set<string>>(new Set(['1']));
  const [expandedSections, setExpandedSections] = useState<{ [key: string]: Set<string> }>({
    '1': new Set(['video', 'audio']),
  });

  const [showAddModal, setShowAddModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [selectedProfile, setSelectedProfile] = useState<Profile | null>(null);
  const [actionStatus, setActionStatus] = useState<'idle' | 'processing'>('idle');

  // Modal tab state
  const [activeTab, setActiveTab] = useState('basic');

  // Form states - Basic
  const [formName, setFormName] = useState('');
  const [formToken, setFormToken] = useState('');
  
  // Form states - Video Source
  const [formVideoResolution, setFormVideoResolution] = useState('1920x1080');
  const [formVideoFrameRate, setFormVideoFrameRate] = useState(25);
  const [formBrightness, setFormBrightness] = useState(50);
  const [formColorSaturation, setFormColorSaturation] = useState(50);
  const [formContrast, setFormContrast] = useState(50);
  const [formSharpness, setFormSharpness] = useState(10);
  const [formIrCutFilter, setFormIrCutFilter] = useState('auto');
  
  // Form states - Video Encoder
  const [formEncoding, setFormEncoding] = useState('h264');
  const [formEncoderResolution, setFormEncoderResolution] = useState('1920x1080');
  const [formQuality, setFormQuality] = useState(4);
  const [formEncoderFrameRate, setFormEncoderFrameRate] = useState(25);
  const [formBitrate, setFormBitrate] = useState(2048);
  
  // Form states - Audio
  const [formAudioEncoding, setFormAudioEncoding] = useState<'g711' | 'aac'>('aac');
  const [formAudioChannels, setFormAudioChannels] = useState(1);
  const [formAudioBitrate, setFormAudioBitrate] = useState(128);
  const [formSampleRate, setFormSampleRate] = useState(16);
  
  // Form states - Multicast
  const [formMulticastIP, setFormMulticastIP] = useState('239.0.1.2');
  const [formMulticastPort, setFormMulticastPort] = useState(32004);
  const [formMulticastTTL, setFormMulticastTTL] = useState(2);
  const [formMulticastAutoStart, setFormMulticastAutoStart] = useState(false);
  
  // Form states - PTZ
  const [formPtzSpeed, setFormPtzSpeed] = useState('1.0');
  const [formPtzTimeout, setFormPtzTimeout] = useState('PT10S');
  const [formMaxPresets, setFormMaxPresets] = useState(10);
  const [formHomeSupported, setFormHomeSupported] = useState(true);

  const resetForm = () => {
    setActiveTab('basic');
    setFormName('');
    setFormToken('');
    setFormVideoResolution('1920x1080');
    setFormVideoFrameRate(25);
    setFormBrightness(50);
    setFormColorSaturation(50);
    setFormContrast(50);
    setFormSharpness(10);
    setFormIrCutFilter('auto');
    setFormEncoding('h264');
    setFormEncoderResolution('1920x1080');
    setFormQuality(4);
    setFormEncoderFrameRate(25);
    setFormBitrate(2048);
    setFormAudioEncoding('aac');
    setFormAudioChannels(1);
    setFormAudioBitrate(128);
    setFormSampleRate(16);
    setFormMulticastIP('239.0.1.2');
    setFormMulticastPort(32004);
    setFormMulticastTTL(2);
    setFormMulticastAutoStart(false);
    setFormPtzSpeed('1.0');
    setFormPtzTimeout('PT10S');
    setFormMaxPresets(10);
    setFormHomeSupported(true);
  };

  const loadProfileToForm = (profile: Profile) => {
    setActiveTab('basic');
    setFormName(profile.name);
    setFormToken(profile.token);
    setFormVideoResolution(profile.videoSourceConfig.videoSource.resolution);
    setFormVideoFrameRate(profile.videoSourceConfig.videoSource.framerate);
    setFormBrightness(profile.videoSourceConfig.videoSource.brightness);
    setFormColorSaturation(profile.videoSourceConfig.videoSource.colorSaturation);
    setFormContrast(profile.videoSourceConfig.videoSource.contrast);
    setFormSharpness(profile.videoSourceConfig.videoSource.sharpness);
    setFormIrCutFilter(profile.videoSourceConfig.videoSource.irCutFilter);
    setFormEncoding(profile.videoEncoderConfig.encoding);
    setFormEncoderResolution(profile.videoEncoderConfig.resolution);
    setFormQuality(profile.videoEncoderConfig.quality);
    setFormEncoderFrameRate(profile.videoEncoderConfig.frameRate);
    setFormBitrate(profile.videoEncoderConfig.bitrate);
    setFormAudioEncoding(profile.audioEncoderConfig.encoding as 'g711' | 'aac');
    setFormAudioChannels(profile.audioSourceConfig.audioSource.channels);
    setFormAudioBitrate(profile.audioEncoderConfig.bitrate);
    setFormSampleRate(profile.audioEncoderConfig.sampleRate);
    setFormMulticastIP(profile.multicast.ipAddress);
    setFormMulticastPort(profile.multicast.port);
    setFormMulticastTTL(profile.multicast.ttl);
    setFormMulticastAutoStart(profile.multicast.autoStart);
    setFormPtzSpeed(profile.ptzConfig.defaultPtzSpeed);
    setFormPtzTimeout(profile.ptzConfig.defaultPtzTimeout);
    setFormMaxPresets(profile.ptzConfig.node.maximumNumberOfPresets);
    setFormHomeSupported(profile.ptzConfig.node.homeSupported);
  };

  const toggleProfile = (profileId: string) => {
    const newExpanded = new Set(expandedProfiles);
    if (newExpanded.has(profileId)) {
      newExpanded.delete(profileId);
    } else {
      newExpanded.add(profileId);
    }
    setExpandedProfiles(newExpanded);
  };

  const toggleSection = (profileId: string, section: string) => {
    const profileSections = expandedSections[profileId] || new Set();
    const newSections = new Set(profileSections);
    if (newSections.has(section)) {
      newSections.delete(section);
    } else {
      newSections.add(section);
    }
    setExpandedSections({
      ...expandedSections,
      [profileId]: newSections,
    });
  };

  const handleDeleteProfile = () => {
    if (!selectedProfile) return;
    setActionStatus('processing');
    setTimeout(() => {
      setProfiles(profiles.filter(p => p.id !== selectedProfile.id));
      setActionStatus('idle');
      setShowDeleteModal(false);
      setSelectedProfile(null);
    }, 1000);
  };

  const handleDuplicateProfile = (profile: Profile) => {
    const newProfile: Profile = {
      ...profile,
      id: Date.now().toString(),
      name: `${profile.name}_copy`,
      token: String(parseInt(profile.token) + 100).padStart(3, '0'),
      fixed: false,
    };
    setProfiles([...profiles, newProfile]);
  };

  const openDeleteModal = (profile: Profile) => {
    setSelectedProfile(profile);
    setShowDeleteModal(true);
  };

  const openEditModal = (profile: Profile) => {
    setSelectedProfile(profile);
    loadProfileToForm(profile);
    setShowEditModal(true);
  };

  const handleAddProfile = () => {
    setActionStatus('processing');
    setTimeout(() => {
      const newId = Date.now().toString();
      const newToken = formToken || String(profiles.length + 2).padStart(3, '0');
      
      const newProfile: Profile = {
        id: newId,
        name: formName,
        token: newToken,
        fixed: false,
        videoSourceConfig: {
          name: `V_SRC_CFG_${newToken}`,
          token: `V_SRC_${newToken}`,
          useCount: 1,
          bounds: `(0, 0, ${formVideoResolution})`,
          videoSource: {
            token: `V_SRC_${newToken}`,
            framerate: formVideoFrameRate,
            resolution: formVideoResolution,
            brightness: formBrightness,
            colorSaturation: formColorSaturation,
            contrast: formContrast,
            sharpness: formSharpness,
            exposure: 'onvif:services:Exposure',
            focus: 'onvif:services:FocusConfiguration',
            irCutFilter: formIrCutFilter,
            backlightCompensation: 'level=10, mode=off',
            whiteBalance: 'CbGain=10, CrGain=10, mode=auto',
            wideDynamicRange: 'level=50, mode=auto',
          },
        },
        audioSourceConfig: {
          name: `A_SRC_CFG_${newToken}`,
          token: `A_SRC_${newToken}`,
          useCount: 1,
          audioSource: {
            token: `AudioSourceToken_${newToken}`,
            channels: formAudioChannels,
          },
        },
        videoEncoderConfig: {
          name: `V_ENC_${newToken}`,
          token: newToken,
          encoding: formEncoding,
          resolution: formEncoderResolution,
          quality: formQuality,
          frameRate: formEncoderFrameRate,
          bitrate: formBitrate,
          sessionTimeout: 'PT10S',
        },
        audioEncoderConfig: {
          name: `A_ENC_${newToken}`,
          token: `A_ENC_${newToken}`,
          encoding: formAudioEncoding,
          bitrate: formAudioBitrate,
          sampleRate: formSampleRate,
          sessionTimeout: 'PT10S',
        },
        multicast: {
          ipAddress: formMulticastIP,
          port: formMulticastPort,
          ttl: formMulticastTTL,
          autoStart: formMulticastAutoStart,
        },
        ptzConfig: {
          name: 'PTZConfiguration',
          token: `PTZConfigurationToken_${newToken}`,
          panTiltLimits: '(-1, 1)',
          zoomLimits: '(0, 1)',
          defaultPtzSpeed: formPtzSpeed,
          defaultPtzTimeout: formPtzTimeout,
          defaultAbsolutePantTiltPositionSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace',
          defaultRelativePanTiltTranslationSpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace',
          defaultContinuousPanTiltVelocitySpace: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace',
          defaultContinuousZoomVelocitySpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace',
          defaultRelativeZoomTranslationSpace: 'http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
          node: {
            name: 'PTZNode',
            token: 'PTZNodeToken',
            homeSupported: formHomeSupported,
            maximumNumberOfPresets: formMaxPresets,
            auxiliaryCommands: 'http://www.onvif.org/ver10/tptz/AuxiliaryCommands/Command1',
            supportedPtzSpaces: 'http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace, http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace, http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace',
          },
        },
        videoAnalytics: {
          name: 'VideoAnalytics',
          token: 'VideoAnalyticsToken',
          useCount: 1,
          modules: {
            cellMotion: 'enabled',
            tamperDetect: 'enabled',
            motionDetectorRule: 'rule1',
            tamperDetectorRule: 'rule2',
          },
        },
      };

      setProfiles([...profiles, newProfile]);
      setActionStatus('idle');
      setShowAddModal(false);
      resetForm();
    }, 1000);
  };

  const handleEditProfile = () => {
    if (!selectedProfile) return;
    setActionStatus('processing');
    setTimeout(() => {
      setProfiles(profiles.map(p => p.id === selectedProfile.id ? {
        ...p,
        name: formName,
        token: formToken,
        videoSourceConfig: {
          ...p.videoSourceConfig,
          videoSource: {
            ...p.videoSourceConfig.videoSource,
            framerate: formVideoFrameRate,
            resolution: formVideoResolution,
            brightness: formBrightness,
            colorSaturation: formColorSaturation,
            contrast: formContrast,
            sharpness: formSharpness,
            irCutFilter: formIrCutFilter,
          },
        },
        audioSourceConfig: {
          ...p.audioSourceConfig,
          audioSource: {
            ...p.audioSourceConfig.audioSource,
            channels: formAudioChannels,
          },
        },
        videoEncoderConfig: {
          ...p.videoEncoderConfig,
          encoding: formEncoding,
          resolution: formEncoderResolution,
          quality: formQuality,
          frameRate: formEncoderFrameRate,
          bitrate: formBitrate,
        },
        audioEncoderConfig: {
          ...p.audioEncoderConfig,
          encoding: formAudioEncoding,
          bitrate: formAudioBitrate,
          sampleRate: formSampleRate,
        },
        multicast: {
          ...p.multicast,
          ipAddress: formMulticastIP,
          port: formMulticastPort,
          ttl: formMulticastTTL,
          autoStart: formMulticastAutoStart,
        },
        ptzConfig: {
          ...p.ptzConfig,
          defaultPtzSpeed: formPtzSpeed,
          defaultPtzTimeout: formPtzTimeout,
          node: {
            ...p.ptzConfig.node,
            homeSupported: formHomeSupported,
            maximumNumberOfPresets: formMaxPresets,
          },
        },
      } : p));
      setActionStatus('idle');
      setShowEditModal(false);
      setSelectedProfile(null);
      resetForm();
    }, 1000);
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[1200px] p-[16px] md:p-[32px] lg:p-[48px] pb-[80px] md:pb-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px] flex flex-col sm:flex-row items-start sm:items-center justify-between gap-[16px]">
          <div>
            <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Profiles</h1>
            <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
              Manage streaming profiles and encoder configurations
            </p>
          </div>
          <button
            onClick={() => setShowAddModal(true)}
            className="w-full sm:w-auto h-[44px] px-[24px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors flex items-center justify-center gap-[8px]"
          >
            <Plus className="size-5" />
            Add Profile
          </button>
        </div>

        {/* Profiles List */}
        <div className="space-y-[16px]">
          {profiles.map((profile) => {
            const isExpanded = expandedProfiles.has(profile.id);
            const sections = expandedSections[profile.id] || new Set();

            return (
              <div
                key={profile.id}
                className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] overflow-hidden"
              >
                {/* Profile Header */}
                <div className="p-[16px] md:p-[24px] flex flex-col md:flex-row items-start md:items-center justify-between gap-[12px] md:gap-0">
                  <div className="flex items-start md:items-center gap-[12px] md:gap-[16px] flex-1 w-full md:w-auto">
                    <button
                      onClick={() => toggleProfile(profile.id)}
                      className="size-[40px] bg-[rgba(220,38,38,0.1)] rounded-[8px] flex items-center justify-center hover:bg-[rgba(220,38,38,0.2)] transition-colors flex-shrink-0"
                    >
                      {isExpanded ? (
                        <ChevronDown className="size-5" color="#dc2626" />
                      ) : (
                        <ChevronRight className="size-5" color="#dc2626" />
                      )}
                    </button>
                    
                    <div className="flex-1 min-w-0">
                      <div className="flex flex-wrap items-center gap-[8px] md:gap-[12px] mb-[4px]">
                        <h3 className="text-white text-[16px] md:text-[18px]">{profile.name}</h3>
                        {profile.fixed && (
                          <span className="px-[6px] md:px-[8px] py-[2px] md:py-[4px] bg-[rgba(255,159,10,0.1)] text-[#ff9f0a] text-[11px] md:text-[12px] rounded-[4px] font-semibold">
                            FIXED
                          </span>
                        )}
                        <span className="text-[#6b6b6f] text-[12px] md:text-[14px]">Token: {profile.token}</span>
                      </div>
                      <p className="text-[#a1a1a6] text-[12px] md:text-[14px] break-words">
                        Video: {profile.videoEncoderConfig.resolution} @ {profile.videoEncoderConfig.frameRate}fps • 
                        Audio: {profile.audioEncoderConfig.encoding.toUpperCase()} • 
                        Encoding: {profile.videoEncoderConfig.encoding.toUpperCase()}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-[8px] w-full md:w-auto">
                    <button
                      onClick={() => openEditModal(profile)}
                      disabled={profile.fixed}
                      className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                    >
                      <Edit2 className="size-4" color="#a1a1a6" />
                    </button>
                    <button
                      onClick={() => handleDuplicateProfile(profile)}
                      className="flex-1 md:flex-none h-[36px] px-[12px] md:px-[16px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center md:justify-start gap-[8px] text-[#a1a1a6] text-[13px] md:text-[14px] hover:bg-[#2c2c2e] transition-colors"
                    >
                      <Copy className="size-4" />
                      <span className="hidden sm:inline">Duplicate</span>
                    </button>
                    <button
                      onClick={() => openDeleteModal(profile)}
                      disabled={profile.fixed}
                      className="size-[36px] rounded-[8px] bg-transparent border border-[#3a3a3c] flex items-center justify-center hover:bg-[rgba(220,38,38,0.1)] hover:border-[#dc2626] transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:border-[#3a3a3c]"
                    >
                      <Trash2 className="size-4" color="#dc2626" />
                    </button>
                  </div>
                </div>

                {/* Expanded Content */}
                {isExpanded && (
                  <div className="border-t border-[#3a3a3c] p-[16px] md:p-[24px] bg-[#0d0d0d]">
                    {/* Video Source Configuration */}
                    <div className="mb-[16px]">
                      <button
                        onClick={() => toggleSection(profile.id, 'video')}
                        className="w-full flex items-center justify-between p-[12px] md:p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[8px] md:gap-[12px] flex-1 min-w-0">
                          <Video className="size-4 md:size-5 flex-shrink-0" color="#dc2626" />
                          <div className="flex flex-col sm:flex-row sm:items-center gap-[4px] sm:gap-[8px] min-w-0">
                            <span className="text-white text-[14px] md:text-[15px] font-semibold">Video Source Configuration</span>
                            <span className="text-[#6b6b6f] text-[12px] md:text-[13px] truncate">{profile.videoSourceConfig.name}</span>
                          </div>
                        </div>
                        {sections.has('video') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('video') && (
                        <div className="mt-[12px] p-[16px] md:p-[20px] bg-[#1c1c1e] rounded-[8px] space-y-[12px]">
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-[12px] md:gap-[16px]">
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                              <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.token}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Frame Rate</label>
                              <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.framerate} fps</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Resolution</label>
                              <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.resolution}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Use Count</label>
                              <p className="text-white text-[14px]">{profile.videoSourceConfig.useCount}</p>
                            </div>
                          </div>

                          <div className="pt-[12px] border-t border-[#3a3a3c]">
                            <h4 className="text-white text-[14px] font-semibold mb-[12px]">Imaging Settings</h4>
                            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-[12px] md:gap-[16px]">
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Brightness</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.brightness}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Color Saturation</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.colorSaturation}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Contrast</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.contrast}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Sharpness</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.sharpness}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">IR Cut Filter</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.irCutFilter}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Backlight Comp.</label>
                                <p className="text-white text-[14px]">{profile.videoSourceConfig.videoSource.backlightCompensation}</p>
                              </div>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Audio Source Configuration */}
                    <div className="mb-[16px]">
                      <button
                        onClick={() => toggleSection(profile.id, 'audio')}
                        className="w-full flex items-center justify-between p-[12px] md:p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[8px] md:gap-[12px] flex-1 min-w-0">
                          <Mic className="size-4 md:size-5 flex-shrink-0" color="#dc2626" />
                          <div className="flex flex-col sm:flex-row sm:items-center gap-[4px] sm:gap-[8px] min-w-0">
                            <span className="text-white text-[14px] md:text-[15px] font-semibold">Audio Source Configuration</span>
                            <span className="text-[#6b6b6f] text-[12px] md:text-[13px] truncate">{profile.audioSourceConfig.name}</span>
                          </div>
                        </div>
                        {sections.has('audio') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('audio') && (
                        <div className="mt-[12px] p-[16px] md:p-[20px] bg-[#1c1c1e] rounded-[8px]">
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-[12px] md:gap-[16px]">
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                              <p className="text-white text-[14px]">{profile.audioSourceConfig.audioSource.token}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Channels</label>
                              <p className="text-white text-[14px]">{profile.audioSourceConfig.audioSource.channels}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Use Count</label>
                              <p className="text-white text-[14px]">{profile.audioSourceConfig.useCount}</p>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Video Encoder Configuration */}
                    <div className="mb-[16px]">
                      <button
                        onClick={() => toggleSection(profile.id, 'videoEncoder')}
                        className="w-full flex items-center justify-between p-[12px] md:p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[8px] md:gap-[12px] flex-1 min-w-0">
                          <SettingsIcon className="size-4 md:size-5 flex-shrink-0" color="#dc2626" />
                          <div className="flex flex-col sm:flex-row sm:items-center gap-[4px] sm:gap-[8px] min-w-0">
                            <span className="text-white text-[14px] md:text-[15px] font-semibold">Video Encoder Configuration</span>
                            <span className="text-[#6b6b6f] text-[12px] md:text-[13px] truncate">{profile.videoEncoderConfig.name}</span>
                          </div>
                        </div>
                        {sections.has('videoEncoder') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('videoEncoder') && (
                        <div className="mt-[12px] p-[16px] md:p-[20px] bg-[#1c1c1e] rounded-[8px]">
                          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-[12px] md:gap-[16px]">
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Encoding</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.encoding}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Resolution</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.resolution}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Quality</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.quality}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Frame Rate</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.frameRate} fps</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Bitrate</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.bitrate} kbps</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Session Timeout</label>
                              <p className="text-white text-[14px]">{profile.videoEncoderConfig.sessionTimeout}</p>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Audio Encoder & Multicast */}
                    <div className="mb-[16px]">
                      <button
                        onClick={() => toggleSection(profile.id, 'audioEncoder')}
                        className="w-full flex items-center justify-between p-[12px] md:p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[8px] md:gap-[12px] flex-1 min-w-0">
                          <Radio className="size-4 md:size-5 flex-shrink-0" color="#dc2626" />
                          <div className="flex flex-col sm:flex-row sm:items-center gap-[4px] sm:gap-[8px] min-w-0">
                            <span className="text-white text-[14px] md:text-[15px] font-semibold">Audio Encoder & Multicast</span>
                            <span className="text-[#6b6b6f] text-[12px] md:text-[13px] truncate">{profile.audioEncoderConfig.name}</span>
                          </div>
                        </div>
                        {sections.has('audioEncoder') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('audioEncoder') && (
                        <div className="mt-[12px] p-[16px] md:p-[20px] bg-[#1c1c1e] rounded-[8px] space-y-[16px]">
                          <div>
                            <h4 className="text-white text-[14px] font-semibold mb-[12px]">Audio Encoder</h4>
                            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-[12px] md:gap-[16px]">
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Encoding</label>
                                <p className="text-white text-[14px]">{profile.audioEncoderConfig.encoding}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Bitrate</label>
                                <p className="text-white text-[14px]">{profile.audioEncoderConfig.bitrate} kbps</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Sample Rate</label>
                                <p className="text-white text-[14px]">{profile.audioEncoderConfig.sampleRate} kHz</p>
                              </div>
                            </div>
                          </div>

                          <div className="pt-[16px] border-t border-[#3a3a3c]">
                            <h4 className="text-white text-[14px] font-semibold mb-[12px]">Multicast</h4>
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-[12px] md:gap-[16px]">
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">IP Address</label>
                                <p className="text-white text-[14px]">{profile.multicast.ipAddress}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Port</label>
                                <p className="text-white text-[14px]">{profile.multicast.port}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">TTL</label>
                                <p className="text-white text-[14px]">{profile.multicast.ttl}</p>
                              </div>
                              <div>
                                <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Auto Start</label>
                                <p className="text-white text-[14px]">{profile.multicast.autoStart ? 'True' : 'False'}</p>
                              </div>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* PTZ Configuration */}
                    <div className="mb-[16px]">
                      <button
                        onClick={() => toggleSection(profile.id, 'ptz')}
                        className="w-full flex items-center justify-between p-[12px] md:p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[8px] md:gap-[12px] flex-1 min-w-0">
                          <ImageIcon className="size-4 md:size-5 flex-shrink-0" color="#dc2626" />
                          <div className="flex flex-col sm:flex-row sm:items-center gap-[4px] sm:gap-[8px] min-w-0">
                            <span className="text-white text-[14px] md:text-[15px] font-semibold">PTZ Configuration</span>
                            <span className="text-[#6b6b6f] text-[12px] md:text-[13px] truncate">{profile.ptzConfig.name}</span>
                          </div>
                        </div>
                        {sections.has('ptz') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('ptz') && (
                        <div className="mt-[12px] p-[16px] md:p-[20px] bg-[#1c1c1e] rounded-[8px]">
                          <div className="grid grid-cols-1 sm:grid-cols-2 gap-[12px] md:gap-[16px]">
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.token}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Pan/Tilt Limits</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.panTiltLimits}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Zoom Limits</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.zoomLimits}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default PTZ Speed</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultPtzSpeed}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default PTZ Timeout</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultPtzTimeout}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default Absolute Pan/Tilt Position Space</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultAbsolutePantTiltPositionSpace}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default Relative Pan/Tilt Translation Space</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultRelativePanTiltTranslationSpace}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default Continuous Pan/Tilt Velocity Space</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultContinuousPanTiltVelocitySpace}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default Continuous Zoom Velocity Space</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultContinuousZoomVelocitySpace}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Default Relative Zoom Translation Space</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.defaultRelativeZoomTranslationSpace}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Node Name</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.name}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Node Token</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.token}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Home Supported</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.homeSupported ? 'True' : 'False'}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Maximum Number of Presets</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.maximumNumberOfPresets}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Auxiliary Commands</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.auxiliaryCommands}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Supported PTZ Spaces</label>
                              <p className="text-white text-[14px]">{profile.ptzConfig.node.supportedPtzSpaces}</p>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>

                    {/* Video Analytics Configuration */}
                    <div>
                      <button
                        onClick={() => toggleSection(profile.id, 'analytics')}
                        className="w-full flex items-center justify-between p-[16px] bg-[#1c1c1e] rounded-[8px] hover:bg-[#2c2c2e] transition-colors"
                      >
                        <div className="flex items-center gap-[12px]">
                          <Activity className="size-5" color="#dc2626" />
                          <span className="text-white text-[15px] font-semibold">Video Analytics Configuration</span>
                          <span className="text-[#6b6b6f] text-[13px]">{profile.videoAnalytics.name}</span>
                        </div>
                        {sections.has('analytics') ? (
                          <ChevronDown className="size-5" color="#a1a1a6" />
                        ) : (
                          <ChevronRight className="size-5" color="#a1a1a6" />
                        )}
                      </button>

                      {sections.has('analytics') && (
                        <div className="mt-[12px] p-[20px] bg-[#1c1c1e] rounded-[8px]">
                          <div className="grid grid-cols-2 gap-[16px]">
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.token}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Use Count</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.useCount}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Cell Motion Module</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.modules.cellMotion}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Tamper Detect Module</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.modules.tamperDetect}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Motion Detector Rule</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.modules.motionDetectorRule}</p>
                            </div>
                            <div>
                              <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Tamper Detector Rule</label>
                              <p className="text-white text-[14px]">{profile.videoAnalytics.modules.tamperDetectorRule}</p>
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {/* Delete Confirmation Modal */}
        {showDeleteModal && selectedProfile && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <Trash2 className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Delete Profile</h3>
              </div>

              <p className="text-[#a1a1a6] text-[15px] mb-[24px]">
                Are you sure you want to delete profile <strong className="text-white">"{selectedProfile.name}"</strong>? This action cannot be undone.
              </p>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleDeleteProfile}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Deleting...
                    </>
                  ) : (
                    'Delete Profile'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowDeleteModal(false);
                    setSelectedProfile(null);
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Profile Form Modals */}
        <ProfileFormModal
          isOpen={showAddModal}
          mode="add"
          onClose={() => {
            setShowAddModal(false);
            resetForm();
          }}
          onSubmit={handleAddProfile}
          isProcessing={actionStatus === 'processing'}
          formName={formName}
          setFormName={setFormName}
          formToken={formToken}
          setFormToken={setFormToken}
          formVideoResolution={formVideoResolution}
          setFormVideoResolution={setFormVideoResolution}
          formVideoFrameRate={formVideoFrameRate}
          setFormVideoFrameRate={setFormVideoFrameRate}
          formBrightness={formBrightness}
          setFormBrightness={setFormBrightness}
          formColorSaturation={formColorSaturation}
          setFormColorSaturation={setFormColorSaturation}
          formContrast={formContrast}
          setFormContrast={setFormContrast}
          formSharpness={formSharpness}
          setFormSharpness={setFormSharpness}
          formIrCutFilter={formIrCutFilter}
          setFormIrCutFilter={setFormIrCutFilter}
          formEncoding={formEncoding}
          setFormEncoding={setFormEncoding}
          formEncoderResolution={formEncoderResolution}
          setFormEncoderResolution={setFormEncoderResolution}
          formQuality={formQuality}
          setFormQuality={setFormQuality}
          formEncoderFrameRate={formEncoderFrameRate}
          setFormEncoderFrameRate={setFormEncoderFrameRate}
          formBitrate={formBitrate}
          setFormBitrate={setFormBitrate}
          formAudioEncoding={formAudioEncoding}
          setFormAudioEncoding={setFormAudioEncoding}
          formAudioChannels={formAudioChannels}
          setFormAudioChannels={setFormAudioChannels}
          formAudioBitrate={formAudioBitrate}
          setFormAudioBitrate={setFormAudioBitrate}
          formSampleRate={formSampleRate}
          setFormSampleRate={setFormSampleRate}
          formMulticastIP={formMulticastIP}
          setFormMulticastIP={setFormMulticastIP}
          formMulticastPort={formMulticastPort}
          setFormMulticastPort={setFormMulticastPort}
          formMulticastTTL={formMulticastTTL}
          setFormMulticastTTL={setFormMulticastTTL}
          formMulticastAutoStart={formMulticastAutoStart}
          setFormMulticastAutoStart={setFormMulticastAutoStart}
          formPtzSpeed={formPtzSpeed}
          setFormPtzSpeed={setFormPtzSpeed}
          formPtzTimeout={formPtzTimeout}
          setFormPtzTimeout={setFormPtzTimeout}
          formMaxPresets={formMaxPresets}
          setFormMaxPresets={setFormMaxPresets}
          formHomeSupported={formHomeSupported}
          setFormHomeSupported={setFormHomeSupported}
        />

        <ProfileFormModal
          isOpen={showEditModal}
          mode="edit"
          onClose={() => {
            setShowEditModal(false);
            setSelectedProfile(null);
            resetForm();
          }}
          onSubmit={handleEditProfile}
          isProcessing={actionStatus === 'processing'}
          formName={formName}
          setFormName={setFormName}
          formToken={formToken}
          setFormToken={setFormToken}
          formVideoResolution={formVideoResolution}
          setFormVideoResolution={setFormVideoResolution}
          formVideoFrameRate={formVideoFrameRate}
          setFormVideoFrameRate={setFormVideoFrameRate}
          formBrightness={formBrightness}
          setFormBrightness={setFormBrightness}
          formColorSaturation={formColorSaturation}
          setFormColorSaturation={setFormColorSaturation}
          formContrast={formContrast}
          setFormContrast={setFormContrast}
          formSharpness={formSharpness}
          setFormSharpness={setFormSharpness}
          formIrCutFilter={formIrCutFilter}
          setFormIrCutFilter={setFormIrCutFilter}
          formEncoding={formEncoding}
          setFormEncoding={setFormEncoding}
          formEncoderResolution={formEncoderResolution}
          setFormEncoderResolution={setFormEncoderResolution}
          formQuality={formQuality}
          setFormQuality={setFormQuality}
          formEncoderFrameRate={formEncoderFrameRate}
          setFormEncoderFrameRate={setFormEncoderFrameRate}
          formBitrate={formBitrate}
          setFormBitrate={setFormBitrate}
          formAudioEncoding={formAudioEncoding}
          setFormAudioEncoding={setFormAudioEncoding}
          formAudioChannels={formAudioChannels}
          setFormAudioChannels={setFormAudioChannels}
          formAudioBitrate={formAudioBitrate}
          setFormAudioBitrate={setFormAudioBitrate}
          formSampleRate={formSampleRate}
          setFormSampleRate={setFormSampleRate}
          formMulticastIP={formMulticastIP}
          setFormMulticastIP={setFormMulticastIP}
          formMulticastPort={formMulticastPort}
          setFormMulticastPort={setFormMulticastPort}
          formMulticastTTL={formMulticastTTL}
          setFormMulticastTTL={setFormMulticastTTL}
          formMulticastAutoStart={formMulticastAutoStart}
          setFormMulticastAutoStart={setFormMulticastAutoStart}
          formPtzSpeed={formPtzSpeed}
          setFormPtzSpeed={setFormPtzSpeed}
          formPtzTimeout={formPtzTimeout}
          setFormPtzTimeout={setFormPtzTimeout}
          formMaxPresets={formMaxPresets}
          setFormMaxPresets={setFormMaxPresets}
          formHomeSupported={formHomeSupported}
          setFormHomeSupported={setFormHomeSupported}
        />

        {/* Old Add Profile Modal - TO BE REMOVED */}
        {false && showAddModal && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <Plus className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Add Profile</h3>
              </div>

              <form className="space-y-[16px]">
                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Name</label>
                    <input
                      type="text"
                      value={formName}
                      onChange={(e) => setFormName(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                    <input
                      type="text"
                      value={formToken}
                      onChange={(e) => setFormToken(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Resolution</label>
                    <input
                      type="text"
                      value={formResolution}
                      onChange={(e) => setFormResolution(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Frame Rate</label>
                    <input
                      type="number"
                      value={formFrameRate}
                      onChange={(e) => setFormFrameRate(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Encoding</label>
                    <select
                      value={formEncoding}
                      onChange={(e) => setFormEncoding(e.target.value as 'h264')}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    >
                      <option value="h264">H.264</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Quality</label>
                    <input
                      type="number"
                      value={formQuality}
                      onChange={(e) => setFormQuality(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Bitrate</label>
                    <input
                      type="number"
                      value={formBitrate}
                      onChange={(e) => setFormBitrate(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Audio Encoding</label>
                    <select
                      value={formAudioEncoding}
                      onChange={(e) => setFormAudioEncoding(e.target.value as 'g711' | 'aac')}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    >
                      <option value="g711">G.711</option>
                      <option value="aac">AAC</option>
                    </select>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Audio Channels</label>
                    <input
                      type="number"
                      value={formAudioChannels}
                      onChange={(e) => setFormAudioChannels(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Multicast IP</label>
                    <input
                      type="text"
                      value={formMulticastIP}
                      onChange={(e) => setFormMulticastIP(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Multicast Port</label>
                    <input
                      type="number"
                      value={formMulticastPort}
                      onChange={(e) => setFormMulticastPort(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>
              </form>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleAddProfile}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Adding...
                    </>
                  ) : (
                    'Add Profile'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowAddModal(false);
                    resetForm();
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Old Edit Profile Modal - TO BE REMOVED */}
        {false && showEditModal && selectedProfile && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <Edit2 className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Edit Profile</h3>
              </div>

              <form className="space-y-[16px]">
                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Name</label>
                    <input
                      type="text"
                      value={formName}
                      onChange={(e) => setFormName(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Token</label>
                    <input
                      type="text"
                      value={formToken}
                      onChange={(e) => setFormToken(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Resolution</label>
                    <input
                      type="text"
                      value={formResolution}
                      onChange={(e) => setFormResolution(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Frame Rate</label>
                    <input
                      type="number"
                      value={formFrameRate}
                      onChange={(e) => setFormFrameRate(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Encoding</label>
                    <select
                      value={formEncoding}
                      onChange={(e) => setFormEncoding(e.target.value as 'h264')}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    >
                      <option value="h264">H.264</option>
                    </select>
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Quality</label>
                    <input
                      type="number"
                      value={formQuality}
                      onChange={(e) => setFormQuality(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Bitrate</label>
                    <input
                      type="number"
                      value={formBitrate}
                      onChange={(e) => setFormBitrate(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Audio Encoding</label>
                    <select
                      value={formAudioEncoding}
                      onChange={(e) => setFormAudioEncoding(e.target.value as 'g711' | 'aac')}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    >
                      <option value="g711">G.711</option>
                      <option value="aac">AAC</option>
                    </select>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Audio Channels</label>
                    <input
                      type="number"
                      value={formAudioChannels}
                      onChange={(e) => setFormAudioChannels(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Multicast IP</label>
                    <input
                      type="text"
                      value={formMulticastIP}
                      onChange={(e) => setFormMulticastIP(e.target.value)}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[4px]">Multicast Port</label>
                    <input
                      type="number"
                      value={formMulticastPort}
                      onChange={(e) => setFormMulticastPort(Number(e.target.value))}
                      className="w-full h-[40px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                </div>
              </form>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleEditProfile}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Editing...
                    </>
                  ) : (
                    'Edit Profile'
                  )}
                </button>
                <button
                  onClick={() => {
                    setShowEditModal(false);
                    setSelectedProfile(null);
                    resetForm();
                  }}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}