import { RefreshCw, Plus, Edit2, X } from 'lucide-react';
import { useState } from 'react';

interface ProfileFormModalProps {
  isOpen: boolean;
  mode: 'add' | 'edit';
  onClose: () => void;
  onSubmit: () => void;
  isProcessing: boolean;
  
  // Form states
  formName: string;
  setFormName: (value: string) => void;
  formToken: string;
  setFormToken: (value: string) => void;
  
  formVideoResolution: string;
  setFormVideoResolution: (value: string) => void;
  formVideoFrameRate: number;
  setFormVideoFrameRate: (value: number) => void;
  formBrightness: number;
  setFormBrightness: (value: number) => void;
  formColorSaturation: number;
  setFormColorSaturation: (value: number) => void;
  formContrast: number;
  setFormContrast: (value: number) => void;
  formSharpness: number;
  setFormSharpness: (value: number) => void;
  formIrCutFilter: string;
  setFormIrCutFilter: (value: string) => void;
  
  formEncoding: string;
  setFormEncoding: (value: string) => void;
  formEncoderResolution: string;
  setFormEncoderResolution: (value: string) => void;
  formQuality: number;
  setFormQuality: (value: number) => void;
  formEncoderFrameRate: number;
  setFormEncoderFrameRate: (value: number) => void;
  formBitrate: number;
  setFormBitrate: (value: number) => void;
  
  formAudioEncoding: 'g711' | 'aac';
  setFormAudioEncoding: (value: 'g711' | 'aac') => void;
  formAudioChannels: number;
  setFormAudioChannels: (value: number) => void;
  formAudioBitrate: number;
  setFormAudioBitrate: (value: number) => void;
  formSampleRate: number;
  setFormSampleRate: (value: number) => void;
  
  formMulticastIP: string;
  setFormMulticastIP: (value: string) => void;
  formMulticastPort: number;
  setFormMulticastPort: (value: number) => void;
  formMulticastTTL: number;
  setFormMulticastTTL: (value: number) => void;
  formMulticastAutoStart: boolean;
  setFormMulticastAutoStart: (value: boolean) => void;
  
  formPtzSpeed: string;
  setFormPtzSpeed: (value: string) => void;
  formPtzTimeout: string;
  setFormPtzTimeout: (value: string) => void;
  formMaxPresets: number;
  setFormMaxPresets: (value: number) => void;
  formHomeSupported: boolean;
  setFormHomeSupported: (value: boolean) => void;
}

export default function ProfileFormModal(props: ProfileFormModalProps) {
  const [activeTab, setActiveTab] = useState('basic');

  if (!props.isOpen) return null;

  const tabs = [
    { id: 'basic', label: 'Basic Info' },
    { id: 'videoSource', label: 'Video Source' },
    { id: 'videoEncoder', label: 'Video Encoder' },
    { id: 'audio', label: 'Audio' },
    { id: 'multicast', label: 'Multicast' },
    { id: 'ptz', label: 'PTZ' },
  ];

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-[24px]">
      <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] max-w-[800px] w-full max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="p-[32px] pb-[24px] border-b border-[#3a3a3c]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                {props.mode === 'add' ? (
                  <Plus className="size-6" color="#dc2626" />
                ) : (
                  <Edit2 className="size-6" color="#dc2626" />
                )}
              </div>
              <h3 className="text-white text-[20px]">
                {props.mode === 'add' ? 'Add Profile' : 'Edit Profile'}
              </h3>
            </div>
            <button
              onClick={props.onClose}
              disabled={props.isProcessing}
              className="size-[32px] rounded-[8px] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors disabled:opacity-50"
            >
              <X className="size-5" color="#a1a1a6" />
            </button>
          </div>

          {/* Tabs */}
          <div className="flex gap-[8px] mt-[24px] overflow-x-auto pb-[2px]">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`px-[16px] py-[8px] rounded-[6px] text-[14px] font-semibold transition-colors whitespace-nowrap ${
                  activeTab === tab.id
                    ? 'bg-[#dc2626] text-white'
                    : 'bg-transparent text-[#a1a1a6] hover:bg-[#2c2c2e]'
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-[32px]">
          {/* Basic Info Tab */}
          {activeTab === 'basic' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Profile Name *</label>
                  <input
                    type="text"
                    value={props.formName}
                    onChange={(e) => props.setFormName(e.target.value)}
                    placeholder="e.g., subStream"
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Token</label>
                  <input
                    type="text"
                    value={props.formToken}
                    onChange={(e) => props.setFormToken(e.target.value)}
                    placeholder="Auto-generated if empty"
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
              </div>
              <div className="p-[16px] bg-[rgba(220,38,38,0.05)] border border-[rgba(220,38,38,0.2)] rounded-[8px]">
                <p className="text-[#dc2626] text-[13px]">
                  * Required field. Token will be auto-generated if not provided.
                </p>
              </div>
            </div>
          )}

          {/* Video Source Tab */}
          {activeTab === 'videoSource' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Resolution</label>
                  <select
                    value={props.formVideoResolution}
                    onChange={(e) => props.setFormVideoResolution(e.target.value)}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="3072x2048">3072x2048 (6MP)</option>
                    <option value="2592x1944">2592x1944 (5MP)</option>
                    <option value="1920x1080">1920x1080 (Full HD)</option>
                    <option value="1280x720">1280x720 (HD)</option>
                    <option value="704x576">704x576 (D1)</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Frame Rate (fps)</label>
                  <input
                    type="number"
                    min="1"
                    max="60"
                    value={props.formVideoFrameRate}
                    onChange={(e) => props.setFormVideoFrameRate(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
              </div>

              <div className="pt-[16px] border-t border-[#3a3a3c]">
                <h4 className="text-white text-[15px] font-semibold mb-[16px]">Imaging Settings</h4>
                <div className="grid grid-cols-2 gap-[16px]">
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Brightness (0-100)</label>
                    <input
                      type="number"
                      min="0"
                      max="100"
                      value={props.formBrightness}
                      onChange={(e) => props.setFormBrightness(Number(e.target.value))}
                      className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Color Saturation (0-100)</label>
                    <input
                      type="number"
                      min="0"
                      max="100"
                      value={props.formColorSaturation}
                      onChange={(e) => props.setFormColorSaturation(Number(e.target.value))}
                      className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Contrast (0-100)</label>
                    <input
                      type="number"
                      min="0"
                      max="100"
                      value={props.formContrast}
                      onChange={(e) => props.setFormContrast(Number(e.target.value))}
                      className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Sharpness (0-100)</label>
                    <input
                      type="number"
                      min="0"
                      max="100"
                      value={props.formSharpness}
                      onChange={(e) => props.setFormSharpness(Number(e.target.value))}
                      className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    />
                  </div>
                  <div>
                    <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">IR Cut Filter</label>
                    <select
                      value={props.formIrCutFilter}
                      onChange={(e) => props.setFormIrCutFilter(e.target.value)}
                      className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                    >
                      <option value="auto">Auto</option>
                      <option value="on">On</option>
                      <option value="off">Off</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Video Encoder Tab */}
          {activeTab === 'videoEncoder' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Encoding</label>
                  <select
                    value={props.formEncoding}
                    onChange={(e) => props.setFormEncoding(e.target.value)}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="h264">H.264</option>
                    <option value="h265">H.265</option>
                    <option value="mjpeg">MJPEG</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Resolution</label>
                  <select
                    value={props.formEncoderResolution}
                    onChange={(e) => props.setFormEncoderResolution(e.target.value)}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="3072x2048">3072x2048 (6MP)</option>
                    <option value="2592x1944">2592x1944 (5MP)</option>
                    <option value="1920x1080">1920x1080 (Full HD)</option>
                    <option value="1280x720">1280x720 (HD)</option>
                    <option value="704x576">704x576 (D1)</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Quality (1-5)</label>
                  <input
                    type="number"
                    min="1"
                    max="5"
                    value={props.formQuality}
                    onChange={(e) => props.setFormQuality(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Frame Rate (fps)</label>
                  <input
                    type="number"
                    min="1"
                    max="60"
                    value={props.formEncoderFrameRate}
                    onChange={(e) => props.setFormEncoderFrameRate(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Bitrate (kbps)</label>
                  <input
                    type="number"
                    min="64"
                    max="16384"
                    value={props.formBitrate}
                    onChange={(e) => props.setFormBitrate(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
              </div>
            </div>
          )}

          {/* Audio Tab */}
          {activeTab === 'audio' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Audio Encoding</label>
                  <select
                    value={props.formAudioEncoding}
                    onChange={(e) => props.setFormAudioEncoding(e.target.value as 'g711' | 'aac')}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="g711">G.711</option>
                    <option value="aac">AAC</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Channels</label>
                  <select
                    value={props.formAudioChannels}
                    onChange={(e) => props.setFormAudioChannels(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value={1}>Mono (1)</option>
                    <option value={2}>Stereo (2)</option>
                  </select>
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Bitrate (kbps)</label>
                  <input
                    type="number"
                    min="32"
                    max="320"
                    value={props.formAudioBitrate}
                    onChange={(e) => props.setFormAudioBitrate(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Sample Rate (kHz)</label>
                  <select
                    value={props.formSampleRate}
                    onChange={(e) => props.setFormSampleRate(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value={8}>8 kHz</option>
                    <option value={16}>16 kHz</option>
                    <option value={32}>32 kHz</option>
                    <option value={44}>44.1 kHz</option>
                    <option value={48}>48 kHz</option>
                  </select>
                </div>
              </div>
            </div>
          )}

          {/* Multicast Tab */}
          {activeTab === 'multicast' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Multicast IP Address</label>
                  <input
                    type="text"
                    value={props.formMulticastIP}
                    onChange={(e) => props.setFormMulticastIP(e.target.value)}
                    placeholder="e.g., 239.0.1.2"
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Port</label>
                  <input
                    type="number"
                    min="1024"
                    max="65535"
                    value={props.formMulticastPort}
                    onChange={(e) => props.setFormMulticastPort(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">TTL (Time To Live)</label>
                  <input
                    type="number"
                    min="1"
                    max="255"
                    value={props.formMulticastTTL}
                    onChange={(e) => props.setFormMulticastTTL(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Auto Start</label>
                  <select
                    value={props.formMulticastAutoStart ? 'true' : 'false'}
                    onChange={(e) => props.setFormMulticastAutoStart(e.target.value === 'true')}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="false">Disabled</option>
                    <option value="true">Enabled</option>
                  </select>
                </div>
              </div>
            </div>
          )}

          {/* PTZ Tab */}
          {activeTab === 'ptz' && (
            <div className="space-y-[20px]">
              <div className="grid grid-cols-2 gap-[16px]">
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Default PTZ Speed</label>
                  <input
                    type="text"
                    value={props.formPtzSpeed}
                    onChange={(e) => props.setFormPtzSpeed(e.target.value)}
                    placeholder="e.g., 1.0"
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Default PTZ Timeout</label>
                  <input
                    type="text"
                    value={props.formPtzTimeout}
                    onChange={(e) => props.setFormPtzTimeout(e.target.value)}
                    placeholder="e.g., PT10S"
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Maximum Number of Presets</label>
                  <input
                    type="number"
                    min="0"
                    max="255"
                    value={props.formMaxPresets}
                    onChange={(e) => props.setFormMaxPresets(Number(e.target.value))}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  />
                </div>
                <div>
                  <label className="block text-[#6b6b6f] text-[13px] mb-[8px]">Home Position Supported</label>
                  <select
                    value={props.formHomeSupported ? 'true' : 'false'}
                    onChange={(e) => props.setFormHomeSupported(e.target.value === 'true')}
                    className="w-full h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[12px] text-white text-[14px] focus:outline-none focus:border-[#dc2626]"
                  >
                    <option value="true">Yes</option>
                    <option value="false">No</option>
                  </select>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-[32px] pt-[24px] border-t border-[#3a3a3c] flex gap-[16px]">
          <button
            onClick={props.onSubmit}
            disabled={props.isProcessing || !props.formName}
            className="flex-1 h-[48px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
          >
            {props.isProcessing ? (
              <>
                <RefreshCw className="size-4 animate-spin" />
                {props.mode === 'add' ? 'Adding...' : 'Saving...'}
              </>
            ) : (
              props.mode === 'add' ? 'Add Profile' : 'Save Changes'
            )}
          </button>
          <button
            onClick={props.onClose}
            disabled={props.isProcessing}
            className="flex-1 h-[48px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
