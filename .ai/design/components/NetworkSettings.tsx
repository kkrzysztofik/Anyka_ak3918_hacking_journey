import { useState } from 'react';
import { Wifi, Globe, RefreshCw, CheckCircle2, AlertCircle } from 'lucide-react';

export default function NetworkSettings() {
  const [ipAddress, setIpAddress] = useState('192.168.1.100');
  const [subnetMask, setSubnetMask] = useState('255.255.255.0');
  const [gateway, setGateway] = useState('192.168.1.1');
  const [primaryDns, setPrimaryDns] = useState('8.8.8.8');
  const [secondaryDns, setSecondaryDns] = useState('8.8.4.4');
  const [hostname, setHostname] = useState('camera-device-01');
  const [httpPort, setHttpPort] = useState('80');
  const [httpsPort, setHttpsPort] = useState('443');
  const [rtspPort, setRtspPort] = useState('554');
  const [dhcpEnabled, setDhcpEnabled] = useState(false);
  const [onvifDiscovery, setOnvifDiscovery] = useState(true);
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle');

  const handleSave = () => {
    setSaveStatus('saving');
    // Simulate API call
    setTimeout(() => {
      setSaveStatus('success');
      setTimeout(() => setSaveStatus('idle'), 2000);
    }, 1000);
  };

  const handleReset = () => {
    setIpAddress('192.168.1.100');
    setSubnetMask('255.255.255.0');
    setGateway('192.168.1.1');
    setPrimaryDns('8.8.8.8');
    setSecondaryDns('8.8.4.4');
    setHostname('camera-device-01');
    setHttpPort('80');
    setHttpsPort('443');
    setRtspPort('554');
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[900px] p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Network Settings</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Configure network connectivity and DNS settings
          </p>
        </div>

        {/* Connection Status Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px] mb-[24px] md:mb-[32px]">
          <div className="flex flex-col sm:flex-row items-start sm:items-center gap-[12px] md:gap-[16px] mb-[16px]">
            <div className="size-[44px] md:size-[48px] bg-[rgba(52,199,89,0.1)] rounded-[12px] flex items-center justify-center flex-shrink-0">
              <Wifi className="size-5 md:size-6" color="#34C759" />
            </div>
            <div className="flex-1">
              <h3 className="text-white text-[15px] md:text-[16px] mb-[4px]">WiFi Connection</h3>
              <p className="text-[#34c759] text-[13px] md:text-[14px] mb-[4px]">Connected to "Office_Network_5G"</p>
              <div className="flex items-center gap-[4px]">
                <div className="flex gap-[2px]">
                  <div className="w-[3px] h-[8px] bg-[#34c759] rounded-[1px]"></div>
                  <div className="w-[3px] h-[12px] bg-[#34c759] rounded-[1px]"></div>
                  <div className="w-[3px] h-[16px] bg-[#34c759] rounded-[1px]"></div>
                  <div className="w-[3px] h-[20px] bg-[#34c759] rounded-[1px]"></div>
                </div>
                <p className="text-[#a1a1a6] text-[11px] md:text-[12px] ml-[4px]">Excellent Signal (92%)</p>
              </div>
            </div>
            <div className="text-left sm:text-right w-full sm:w-auto">
              <p className="text-[#a1a1a6] text-[11px] md:text-[12px] mb-[4px]">Uptime</p>
              <p className="text-white text-[13px] md:text-[14px]">24h 15m</p>
            </div>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-[12px] md:gap-[16px] pt-[16px] border-t border-[#3a3a3c]">
            <div>
              <p className="text-[#a1a1a6] text-[11px] md:text-[12px] mb-[4px]">MAC Address</p>
              <p className="text-white text-[12px] md:text-[14px]">00:1A:2B:3C:4D:5E</p>
            </div>
            <div>
              <p className="text-[#a1a1a6] text-[11px] md:text-[12px] mb-[4px]">Speed</p>
              <p className="text-white text-[12px] md:text-[14px]">866 Mbps</p>
            </div>
            <div>
              <p className="text-[#a1a1a6] text-[11px] md:text-[12px] mb-[4px]">Channel</p>
              <p className="text-white text-[12px] md:text-[14px]">149</p>
            </div>
            <div>
              <p className="text-[#a1a1a6] text-[11px] md:text-[12px] mb-[4px]">Security</p>
              <p className="text-white text-[12px] md:text-[14px]">WPA2-PSK</p>
            </div>
          </div>
        </div>

        {/* DHCP Toggle */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[40px] bg-[rgba(0,122,255,0.1)] rounded-[10px] flex items-center justify-center">
                <Globe className="size-5" color="#007AFF" />
              </div>
              <div>
                <h3 className="text-white text-[16px] mb-[4px]">DHCP Configuration</h3>
                <p className="text-[#a1a1a6] text-[13px]">
                  {dhcpEnabled ? 'Automatically obtain IP address' : 'Use static IP address'}
                </p>
              </div>
            </div>
            <button
              onClick={() => setDhcpEnabled(!dhcpEnabled)}
              className={`relative w-[52px] h-[32px] rounded-[16px] transition-colors ${
                dhcpEnabled ? 'bg-[#34c759]' : 'bg-[#3a3a3c]'
              }`}
            >
              <div
                className={`absolute top-[2px] size-[28px] bg-white rounded-full transition-transform ${
                  dhcpEnabled ? 'translate-x-[22px]' : 'translate-x-[2px]'
                }`}
              />
            </button>
          </div>
        </div>

        {/* ONVIF Discovery Toggle */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[40px] bg-[rgba(255,59,48,0.1)] rounded-[10px] flex items-center justify-center">
                <svg className="size-5" fill="none" viewBox="0 0 24 24" stroke="#ff3b30" strokeWidth="2">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
              </div>
              <div>
                <h3 className="text-white text-[16px] mb-[4px]">ONVIF Discovery</h3>
                <p className="text-[#a1a1a6] text-[13px]">
                  {onvifDiscovery ? 'Device is discoverable on network' : 'Device is hidden from network discovery'}
                </p>
              </div>
            </div>
            <button
              onClick={() => setOnvifDiscovery(!onvifDiscovery)}
              className={`relative w-[52px] h-[32px] rounded-[16px] transition-colors ${
                onvifDiscovery ? 'bg-[#34c759]' : 'bg-[#3a3a3c]'
              }`}
            >
              <div
                className={`absolute top-[2px] size-[28px] bg-white rounded-full transition-transform ${
                  onvifDiscovery ? 'translate-x-[22px]' : 'translate-x-[2px]'
                }`}
              />
            </button>
          </div>
        </div>

        {/* IP Configuration Form */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">IP Configuration</h3>
          
          <div className="grid grid-cols-2 gap-[24px]">
            {/* Hostname */}
            <div className="col-span-2">
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Hostname
              </label>
              <input
                type="text"
                value={hostname}
                onChange={(e) => setHostname(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="camera-device-01"
              />
            </div>

            {/* IP Address */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                IP Address
              </label>
              <input
                type="text"
                value={ipAddress}
                onChange={(e) => setIpAddress(e.target.value)}
                disabled={dhcpEnabled}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                placeholder="192.168.1.100"
              />
            </div>

            {/* Subnet Mask */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Subnet Mask
              </label>
              <input
                type="text"
                value={subnetMask}
                onChange={(e) => setSubnetMask(e.target.value)}
                disabled={dhcpEnabled}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                placeholder="255.255.255.0"
              />
            </div>

            {/* Gateway */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Default Gateway
              </label>
              <input
                type="text"
                value={gateway}
                onChange={(e) => setGateway(e.target.value)}
                disabled={dhcpEnabled}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                placeholder="192.168.1.1"
              />
            </div>

            {/* Primary DNS */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Primary DNS
              </label>
              <input
                type="text"
                value={primaryDns}
                onChange={(e) => setPrimaryDns(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="8.8.8.8"
              />
            </div>

            {/* Secondary DNS */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                Secondary DNS
              </label>
              <input
                type="text"
                value={secondaryDns}
                onChange={(e) => setSecondaryDns(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="8.8.4.4"
              />
            </div>
          </div>
        </div>

        {/* Port Configuration */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Port Configuration</h3>
          
          <div className="grid grid-cols-3 gap-[24px]">
            {/* HTTP Port */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                HTTP Port
              </label>
              <input
                type="text"
                value={httpPort}
                onChange={(e) => setHttpPort(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="80"
              />
            </div>

            {/* HTTPS Port */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                HTTPS Port
              </label>
              <input
                type="text"
                value={httpsPort}
                onChange={(e) => setHttpsPort(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="443"
              />
            </div>

            {/* RTSP Port */}
            <div>
              <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                RTSP Port
              </label>
              <input
                type="text"
                value={rtspPort}
                onChange={(e) => setRtspPort(e.target.value)}
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#ff3b30] transition-colors"
                placeholder="554"
              />
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-[16px]">
          <button
            onClick={handleSave}
            disabled={saveStatus === 'saving'}
            className="h-[44px] px-[32px] bg-[#ff3b30] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ff4d42] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-[8px]"
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
        <div className="mt-[24px] p-[16px] bg-[rgba(0,122,255,0.05)] border border-[rgba(0,122,255,0.2)] rounded-[8px]">
          <div className="flex gap-[12px]">
            <AlertCircle className="size-5 text-[#007AFF] flex-shrink-0 mt-[2px]" />
            <div>
              <p className="text-[#007AFF] text-[14px] mb-[4px]">Network Configuration Note</p>
              <p className="text-[#a1a1a6] text-[13px]">
                Changing network settings may temporarily interrupt connectivity. Ensure you have physical access to the device before making changes to avoid losing access.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}