import { useState, useEffect } from 'react';
import { Clock, Globe, Settings as SettingsIcon, CheckCircle2, RefreshCw, Info, Server } from 'lucide-react';

export default function TimeSettings() {
  const [currentTime, setCurrentTime] = useState(new Date());
  const [timeZone, setTimeZone] = useState('Europe/Paris');
  const [daylightSaving, setDaylightSaving] = useState(true);
  const [timeSync, setTimeSync] = useState('ntp');
  const [ntpServer1, setNtpServer1] = useState('pool.ntp.org');
  const [ntpServer2, setNtpServer2] = useState('time.nist.gov');
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle');

  // Update current time every second
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const formatTime = (date: Date) => {
    const hours = date.getHours().toString().padStart(2, '0');
    const minutes = date.getMinutes().toString().padStart(2, '0');
    const seconds = date.getSeconds().toString().padStart(2, '0');
    return `${hours}:${minutes}:${seconds}`;
  };

  const formatDate = (date: Date) => {
    const day = date.getDate().toString().padStart(2, '0');
    const month = (date.getMonth() + 1).toString().padStart(2, '0');
    const year = date.getFullYear();
    return `${day}.${month}.${year}`;
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
    setTimeZone('Europe/Paris');
    setDaylightSaving(true);
    setTimeSync('ntp');
    setNtpServer1('pool.ntp.org');
    setNtpServer2('time.nist.gov');
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[900px] p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Time Settings</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Configure time zone, NTP synchronization, and daylight saving time
          </p>
        </div>

        {/* Current Time Display */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[32px]">
          <div className="flex items-center gap-[16px]">
            <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
              <Clock className="size-6" color="#dc2626" />
            </div>
            <div className="flex-1">
              <h3 className="text-[#a1a1a6] text-[14px] mb-[8px]">Current time</h3>
              <div className="flex items-center gap-[16px]">
                <p className="text-white text-[24px]">{formatTime(currentTime)}</p>
                <p className="text-[#a1a1a6] text-[18px]">{formatDate(currentTime)}</p>
                <span className="text-[#dc2626] text-[14px] px-[12px] py-[4px] bg-[rgba(220,38,38,0.1)] rounded-[6px]">
                  UTC
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* Time Zone Configuration */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Time Zone</h3>
          
          <div className="mb-[24px]">
            <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
              Time zone
            </label>
            <div className="flex gap-[12px]">
              <div className="flex-1 relative">
                <select
                  value={timeZone}
                  onChange={(e) => setTimeZone(e.target.value)}
                  className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[40px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors appearance-none cursor-pointer"
                >
                  <option value="UTC">UTC - Coordinated Universal Time</option>
                  <optgroup label="Europe">
                    <option value="Europe/London">London (GMT/BST)</option>
                    <option value="Europe/Paris">Paris (CET/CEST)</option>
                    <option value="Europe/Berlin">Berlin (CET/CEST)</option>
                    <option value="Europe/Rome">Rome (CET/CEST)</option>
                    <option value="Europe/Madrid">Madrid (CET/CEST)</option>
                    <option value="Europe/Amsterdam">Amsterdam (CET/CEST)</option>
                    <option value="Europe/Brussels">Brussels (CET/CEST)</option>
                    <option value="Europe/Vienna">Vienna (CET/CEST)</option>
                    <option value="Europe/Warsaw">Warsaw (CET/CEST)</option>
                    <option value="Europe/Moscow">Moscow (MSK)</option>
                  </optgroup>
                  <optgroup label="Americas">
                    <option value="America/New_York">New York (EST/EDT)</option>
                    <option value="America/Chicago">Chicago (CST/CDT)</option>
                    <option value="America/Denver">Denver (MST/MDT)</option>
                    <option value="America/Los_Angeles">Los Angeles (PST/PDT)</option>
                    <option value="America/Toronto">Toronto (EST/EDT)</option>
                    <option value="America/Vancouver">Vancouver (PST/PDT)</option>
                    <option value="America/Mexico_City">Mexico City (CST/CDT)</option>
                    <option value="America/Sao_Paulo">São Paulo (BRT/BRST)</option>
                  </optgroup>
                  <optgroup label="Asia">
                    <option value="Asia/Dubai">Dubai (GST)</option>
                    <option value="Asia/Shanghai">Shanghai (CST)</option>
                    <option value="Asia/Hong_Kong">Hong Kong (HKT)</option>
                    <option value="Asia/Tokyo">Tokyo (JST)</option>
                    <option value="Asia/Seoul">Seoul (KST)</option>
                    <option value="Asia/Singapore">Singapore (SGT)</option>
                    <option value="Asia/Bangkok">Bangkok (ICT)</option>
                    <option value="Asia/Kolkata">Mumbai/Kolkata (IST)</option>
                  </optgroup>
                  <optgroup label="Pacific">
                    <option value="Australia/Sydney">Sydney (AEDT/AEST)</option>
                    <option value="Australia/Melbourne">Melbourne (AEDT/AEST)</option>
                    <option value="Pacific/Auckland">Auckland (NZDT/NZST)</option>
                  </optgroup>
                </select>
                <div className="absolute right-[16px] top-1/2 translate-y-[-50%] pointer-events-none">
                  <svg className="size-4" fill="none" viewBox="0 0 24 24" stroke="#a1a1a6" strokeWidth="2">
                    <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
              </div>
              <button className="size-[44px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors">
                <SettingsIcon className="size-5" color="#a1a1a6" />
              </button>
            </div>
          </div>

          {/* Daylight Saving Checkbox */}
          <div className="flex items-center gap-[12px]">
            <input
              type="checkbox"
              id="daylight-saving"
              checked={daylightSaving}
              onChange={(e) => setDaylightSaving(e.target.checked)}
              className="size-[18px] bg-transparent border-2 border-[#3a3a3c] rounded-[4px] cursor-pointer accent-[#dc2626]"
            />
            <label htmlFor="daylight-saving" className="text-[#a1a1a6] text-[14px] cursor-pointer">
              Automatically adjust for daylight saving time changes.
            </label>
          </div>
        </div>

        {/* Time Synchronization */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <h3 className="text-white text-[18px] mb-[24px]">Time Synchronization</h3>
          
          <div className="space-y-[16px]">
            {/* NTP Server Option */}
            <div 
              onClick={() => setTimeSync('ntp')}
              className={`flex items-center justify-between p-[16px] border rounded-[8px] cursor-pointer transition-colors ${
                timeSync === 'ntp' 
                  ? 'border-[#dc2626] bg-[rgba(220,38,38,0.05)]' 
                  : 'border-[#3a3a3c] hover:bg-[#2c2c2e]'
              }`}
            >
              <div className="flex items-center gap-[16px]">
                <div className={`size-[40px] rounded-[10px] flex items-center justify-center ${
                  timeSync === 'ntp' ? 'bg-[rgba(220,38,38,0.1)]' : 'bg-[#2c2c2e]'
                }`}>
                  <Server className="size-5" color={timeSync === 'ntp' ? '#dc2626' : '#a1a1a6'} />
                </div>
                <div>
                  <h4 className={`text-[15px] mb-[2px] ${timeSync === 'ntp' ? 'text-white' : 'text-[#a1a1a6]'}`}>
                    Synchronize with NTP server
                  </h4>
                  <p className="text-[#a1a1a6] text-[13px]">
                    Automatically sync time from network time servers
                  </p>
                </div>
              </div>
              <div className={`size-[20px] rounded-full border-2 flex items-center justify-center ${
                timeSync === 'ntp' ? 'border-[#dc2626]' : 'border-[#3a3a3c]'
              }`}>
                {timeSync === 'ntp' && (
                  <div className="size-[10px] rounded-full bg-[#dc2626]" />
                )}
              </div>
            </div>

            {/* Synchronize with Computer Option */}
            <div 
              onClick={() => setTimeSync('computer')}
              className={`flex items-center justify-between p-[16px] border rounded-[8px] cursor-pointer transition-colors ${
                timeSync === 'computer' 
                  ? 'border-[#dc2626] bg-[rgba(220,38,38,0.05)]' 
                  : 'border-[#3a3a3c] hover:bg-[#2c2c2e]'
              }`}
            >
              <div className="flex items-center gap-[16px]">
                <div className={`size-[40px] rounded-[10px] flex items-center justify-center ${
                  timeSync === 'computer' ? 'bg-[rgba(220,38,38,0.1)]' : 'bg-[#2c2c2e]'
                }`}>
                  <Globe className="size-5" color={timeSync === 'computer' ? '#dc2626' : '#a1a1a6'} />
                </div>
                <div>
                  <h4 className={`text-[15px] mb-[2px] ${timeSync === 'computer' ? 'text-white' : 'text-[#a1a1a6]'}`}>
                    Synchronize with computer
                  </h4>
                  <p className="text-[#a1a1a6] text-[13px]">
                    Use the time from your connected computer
                  </p>
                </div>
              </div>
              <div className={`size-[20px] rounded-full border-2 flex items-center justify-center ${
                timeSync === 'computer' ? 'border-[#dc2626]' : 'border-[#3a3a3c]'
              }`}>
                {timeSync === 'computer' && (
                  <div className="size-[10px] rounded-full bg-[#dc2626]" />
                )}
              </div>
            </div>

            {/* Set Manually Option */}
            <div 
              onClick={() => setTimeSync('manual')}
              className={`flex items-center justify-between p-[16px] border rounded-[8px] cursor-pointer transition-colors ${
                timeSync === 'manual' 
                  ? 'border-[#dc2626] bg-[rgba(220,38,38,0.05)]' 
                  : 'border-[#3a3a3c] hover:bg-[#2c2c2e]'
              }`}
            >
              <div className="flex items-center gap-[16px]">
                <div className={`size-[40px] rounded-[10px] flex items-center justify-center ${
                  timeSync === 'manual' ? 'bg-[rgba(220,38,38,0.1)]' : 'bg-[#2c2c2e]'
                }`}>
                  <Clock className="size-5" color={timeSync === 'manual' ? '#dc2626' : '#a1a1a6'} />
                </div>
                <div>
                  <h4 className={`text-[15px] mb-[2px] ${timeSync === 'manual' ? 'text-white' : 'text-[#a1a1a6]'}`}>
                    Set manually
                  </h4>
                  <p className="text-[#a1a1a6] text-[13px]">
                    Manually configure date and time
                  </p>
                </div>
              </div>
              <div className={`size-[20px] rounded-full border-2 flex items-center justify-center ${
                timeSync === 'manual' ? 'border-[#dc2626]' : 'border-[#3a3a3c]'
              }`}>
                {timeSync === 'manual' && (
                  <div className="size-[10px] rounded-full bg-[#dc2626]" />
                )}
              </div>
            </div>
          </div>
        </div>

        {/* NTP Server Configuration */}
        {timeSync === 'ntp' && (
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
            <h3 className="text-white text-[18px] mb-[24px]">NTP Server Configuration</h3>
            
            <div className="space-y-[24px]">
              {/* NTP Server 1 */}
              <div>
                <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                  Primary NTP Server
                </label>
                <input
                  type="text"
                  value={ntpServer1}
                  onChange={(e) => setNtpServer1(e.target.value)}
                  className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                  placeholder="pool.ntp.org"
                />
              </div>

              {/* NTP Server 2 */}
              <div>
                <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                  Secondary NTP Server
                </label>
                <input
                  type="text"
                  value={ntpServer2}
                  onChange={(e) => setNtpServer2(e.target.value)}
                  className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                  placeholder="time.nist.gov"
                />
              </div>
            </div>

            {/* NTP Info */}
            <div className="mt-[24px] p-[16px] bg-[rgba(0,122,255,0.05)] border border-[rgba(0,122,255,0.2)] rounded-[8px]">
              <div className="flex gap-[12px]">
                <Info className="size-5 text-[#007AFF] flex-shrink-0 mt-[2px]" />
                <div>
                  <p className="text-[#007AFF] text-[14px] mb-[4px]">NTP Server Information</p>
                  <p className="text-[#a1a1a6] text-[13px]">
                    Common NTP servers: pool.ntp.org, time.nist.gov, time.google.com, time.windows.com
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Manual Time Setting (when manual mode is selected) */}
        {timeSync === 'manual' && (
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
            <h3 className="text-white text-[18px] mb-[24px]">Manual Time Configuration</h3>
            
            <div className="grid grid-cols-2 gap-[24px]">
              <div>
                <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                  Date
                </label>
                <input
                  type="date"
                  className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                />
              </div>
              <div>
                <label className="block text-[#a1a1a6] text-[14px] mb-[8px]">
                  Time
                </label>
                <input
                  type="time"
                  className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] focus:outline-none focus:border-[#dc2626] transition-colors"
                />
              </div>
            </div>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex items-center gap-[16px]">
          <button
            onClick={handleSave}
            disabled={saveStatus === 'saving'}
            className="h-[44px] px-[32px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-[8px]"
          >
            {saveStatus === 'saving' && <RefreshCw className="size-4 animate-spin" />}
            {saveStatus === 'success' && <CheckCircle2 className="size-4" />}
            {saveStatus === 'saving' ? 'Applying...' : saveStatus === 'success' ? 'Applied!' : 'Apply'}
          </button>
          
          <button
            onClick={handleReset}
            className="h-[44px] px-[32px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#1c1c1e] hover:text-white transition-colors"
          >
            Cancel
          </button>
        </div>

        {/* Timezone Info */}
        <div className="mt-[24px] p-[16px] bg-[rgba(220,38,38,0.05)] border border-[rgba(220,38,38,0.2)] rounded-[8px]">
          <div className="flex gap-[12px]">
            <Globe className="size-5 text-[#dc2626] flex-shrink-0 mt-[2px]" />
            <div>
              <p className="text-[#dc2626] text-[14px] mb-[4px]">Time Zone Format</p>
              <p className="text-[#a1a1a6] text-[13px]">
                Time zones use the POSIX TZ format. For example: CET-1CEST means Central European Time with daylight saving time adjustments.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}