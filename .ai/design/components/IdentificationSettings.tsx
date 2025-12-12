import { useState } from 'react';
import { Smartphone, Info, CheckCircle2, RefreshCw, AlertCircle } from 'lucide-react';

export default function IdentificationSettings() {
  const [name, setName] = useState('NVT');
  const [location, setLocation] = useState('country/china');
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle');

  // Read-only device information
  const deviceInfo = {
    manufacturer: 'H264',
    model: 'XM535 X6E-WEQ_8M',
    hardware: '00001',
    firmware: 'V5.05.R02.00A073.10010.346532.S.ONVIF 21.06',
    deviceId: '940b5413c9d02f27ne4',
    ipAddress: '192.168.1.10',
    macAddress: '00-12-34-70-DF-D2',
    onvifVersion: '21.01'
  };

  const handleSave = () => {
    setSaveStatus('saving');
    // Simulate API call
    setTimeout(() => {
      setSaveStatus('success');
      setTimeout(() => setSaveStatus('idle'), 2000);
    }, 1000);
  };

  const handleReset = () => {
    setName('NVT');
    setLocation('country/china');
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[900px] p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Identification Settings</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Configure device identification and location information
          </p>
        </div>

        {/* Device Status Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[32px]">
          <div className="flex items-center gap-[16px]">
            <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
              <Smartphone className="size-6" color="#dc2626" />
            </div>
            <div className="flex-1">
              <h3 className="text-white text-[16px] mb-[4px]">{name}</h3>
              <p className="text-[#a1a1a6] text-[14px]">{deviceInfo.model}</p>
            </div>
            <div className="flex items-center gap-[8px] px-[16px] py-[8px] bg-[rgba(52,199,89,0.1)] rounded-[8px]">
              <div className="size-[8px] bg-[#34c759] rounded-full animate-pulse" />
              <p className="text-[#34c759] text-[14px]">Online</p>
            </div>
          </div>
        </div>

        {/* Editable Configuration */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Device Configuration</h3>
          
          <div className="grid grid-cols-2 gap-[24px]">
            {/* Name */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                placeholder="Device name"
              />
            </div>

            {/* Location */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Location
              </label>
              <input
                type="text"
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                placeholder="Device location"
              />
            </div>
          </div>
        </div>

        {/* Hardware Information */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Hardware Information</h3>
          
          <div className="grid grid-cols-2 gap-[24px]">
            {/* Manufacturer */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Manufacturer
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.manufacturer}</p>
              </div>
            </div>

            {/* Model */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Model
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.model}</p>
              </div>
            </div>

            {/* Hardware Version */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Hardware
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.hardware}</p>
              </div>
            </div>

            {/* Firmware Version */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Firmware
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.firmware}</p>
              </div>
            </div>
          </div>
        </div>

        {/* Network & System Information */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Network & System Information</h3>
          
          <div className="grid grid-cols-2 gap-[24px]">
            {/* Device ID */}
            <div className="col-span-2">
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Device ID
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.deviceId}</p>
              </div>
            </div>

            {/* IP Address */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                IP Address
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.ipAddress}</p>
              </div>
            </div>

            {/* MAC Address */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                MAC Address
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.macAddress}</p>
              </div>
            </div>

            {/* ONVIF Version */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                ONVIF Version
              </label>
              <div className="h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
                <p className="text-[#6b6b6f] text-[15px]">{deviceInfo.onvifVersion}</p>
              </div>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-[16px]">
          <button
            onClick={handleSave}
            disabled={saveStatus === 'saving'}
            className="h-[44px] px-[32px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-[8px]"
          >
            {saveStatus === 'saving' && <RefreshCw className="size-4 animate-spin" />}
            {saveStatus === 'success' && <CheckCircle2 className="size-4" />}
            {saveStatus === 'saving' ? 'Saving...' : saveStatus === 'success' ? 'Saved!' : 'Save Changes'}
          </button>
          
          <button
            onClick={handleReset}
            className="h-[44px] px-[32px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#1c1c1e] hover:text-white transition-colors"
          >
            Reset to Default
          </button>
        </div>

        {/* Help Text */}
        <div className="mt-[24px] p-[16px] bg-[rgba(220,38,38,0.05)] border border-[rgba(220,38,38,0.2)] rounded-[8px]">
          <div className="flex gap-[12px]">
            <Info className="size-5 text-[#dc2626] flex-shrink-0 mt-[2px]" />
            <div>
              <p className="text-[#dc2626] text-[14px] mb-[4px]">Device Information</p>
              <p className="text-[#a1a1a6] text-[13px]">
                Hardware, firmware, and system information are read-only and determined by the device. Only Name and Location can be customized.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}