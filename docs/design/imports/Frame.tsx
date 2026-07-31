import svgPaths from "./svg-tjyz9d9lof";
import { useState } from 'react';
import { Settings, Network, Video, Users, Clock, Wrench, Shield, Image, User, Menu, X } from 'lucide-react';
import NetworkSettings from '../components/NetworkSettings';
import IdentificationSettings from '../components/IdentificationSettings';
import TimeSettings from '../components/TimeSettings';
import MaintenanceSettings from '../components/MaintenanceSettings';
import UserManagement from '../components/UserManagement';
import ProfilesManagement from '../components/ProfilesManagement';
import ImagingSettings from '../components/ImagingSettings';
import LiveVideoPreview from '../components/LiveVideoPreview';
import DiagnosticsStatistics from '../components/DiagnosticsStatistics';

function Background() {
  return (
    <div className="absolute inset-0 bg-[#0d0d0d]" data-name="Background" />
  );
}

function SettingsItem({ icon: Icon, title, description, isActive, onClick }: {
  icon: any;
  title: string;
  description: string;
  isActive: boolean;
  onClick: () => void;
}) {
  return (
    <div 
      className={`relative h-[82px] left-[32px] right-[32px] rounded-[8px] cursor-pointer ${
        isActive ? 'bg-[rgba(255,59,48,0.1)] border-[#ff3b30] border-[0px_0px_0px_3px] border-solid' : ''
      }`}
      onClick={onClick}
    >
      <div className="absolute h-[23.999px] left-[16px] top-1/2 translate-y-[-50%] w-[19.53px]">
        <Icon className="size-6" color={isActive ? 'white' : '#A1A1A6'} />
      </div>
      <div className={`absolute flex flex-col font-['Inter:Semi_Bold',sans-serif] font-semibold h-[50px] justify-center leading-[normal] left-[51.54px] not-italic text-[0px] ${
        isActive ? 'text-white' : 'text-[#a1a1a6]'
      } top-[41px] translate-y-[-50%]`}>
        <p className="mb-0 text-[16px]">{title}</p>
        <p className="font-['Inter:Regular',sans-serif] font-normal mb-0 text-[#a1a1a6] text-[12.8px]">{description.split('\n')[0]}</p>
        {description.split('\n')[1] && (
          <p className="font-['Inter:Regular',sans-serif] font-normal text-[#a1a1a6] text-[12.8px]">{description.split('\n')[1]}</p>
        )}
      </div>
    </div>
  );
}

function BackgroundVerticalBorder({ activeSection, onSectionChange }: { 
  activeSection: string; 
  onSectionChange: (section: string) => void;
}) {
  const sections = [
    { id: 'identification', icon: User, title: 'Identification', description: 'Device name and\nidentification info.' },
    { id: 'network', icon: Network, title: 'Network', description: 'Configure IP, DNS, and\nports.' },
    { id: 'time', icon: Clock, title: 'Time Settings', description: 'Set timezone, NTP\nservers.' },
    { id: 'maintenance', icon: Wrench, title: 'Maintenance', description: 'Updates, backups,\nand logs.' },
    { id: 'video', icon: Video, title: 'Imaging Settings', description: 'Camera image controls\nand exposure.' },
    { id: 'users', icon: Users, title: 'User Management', description: 'Manage accounts and\npermissions.' },
    { id: 'profiles', icon: Image, title: 'Profiles', description: 'Configuration profiles\nand presets.' },
  ];

  return (
    <div className="absolute bg-[#1c1c1e] border-[#3a3a3c] border-[0px_1px_0px_0px] border-solid bottom-0 left-[76.84px] top-0 w-[280px] overflow-y-auto" data-name="Background+VerticalBorder">
      <div className="absolute flex flex-col font-['Inter:Semi_Bold',sans-serif] font-semibold h-[17px] justify-center leading-[0] left-[32px] not-italic text-[#a1a1a6] text-[14.4px] top-[40.5px] tracking-[1px] translate-y-[-50%] w-[80.454px]">
        <p className="leading-[normal]">SETTINGS</p>
      </div>
      <div className="mt-[81px] pb-[32px]">
        {sections.map((section, index) => (
          <div key={section.id} style={{ marginTop: index > 0 ? '16px' : '0' }}>
            <SettingsItem 
              icon={section.icon}
              title={section.title}
              description={section.description}
              isActive={activeSection === section.id}
              onClick={() => onSectionChange(section.id)}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

function Container2() {
  return (
    <div className="absolute h-[18px] left-[16px] overflow-auto right-[16px] top-[12px]" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[16px] text-white top-[8.5px] translate-y-[-50%] w-[102.53px]">
        <p className="leading-[normal]">192.168.1.100</p>
      </div>
    </div>
  );
}

function Input() {
  return (
    <div className="absolute border border-[#3a3a3c] border-solid h-[44px] left-0 overflow-clip right-0 rounded-[8px] top-[25px]" data-name="Input">
      <Container2 />
    </div>
  );
}

function Container3() {
  return (
    <div className="absolute h-[18px] left-[16px] overflow-auto right-[16px] top-[12px]" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[15px] text-white top-[8.5px] translate-y-[-50%] w-[102.53px]">
        <p className="leading-[normal]">255.255.255.0</p>
      </div>
    </div>
  );
}

function Input1() {
  return (
    <div className="absolute border border-[#3a3a3c] border-solid h-[44px] left-0 overflow-clip right-0 rounded-[8px] top-[118px]" data-name="Input">
      <Container3 />
    </div>
  );
}

function Container4() {
  return (
    <div className="absolute h-[18px] left-[16px] overflow-auto right-[16px] top-[12px]" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[16px] text-white top-[8.5px] translate-y-[-50%] w-[84.73px]">
        <p className="leading-[normal]">192.168.1.1</p>
      </div>
    </div>
  );
}

function Input2() {
  return (
    <div className="absolute border border-[#3a3a3c] border-solid h-[44px] left-0 overflow-clip right-0 rounded-[8px] top-[211px]" data-name="Input">
      <Container4 />
    </div>
  );
}

function Container5() {
  return (
    <div className="absolute h-[18px] left-[16px] overflow-auto right-[16px] top-[12px]" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[15.5px] text-white top-[8.5px] translate-y-[-50%] w-[49.14px]">
        <p className="leading-[normal]">8.8.8.8</p>
      </div>
    </div>
  );
}

function Input3() {
  return (
    <div className="absolute border border-[#3a3a3c] border-solid h-[44px] left-0 overflow-clip right-0 rounded-[8px] top-[304px]" data-name="Input">
      <Container5 />
    </div>
  );
}

function Container6() {
  return (
    <div className="absolute h-[18px] left-[16px] overflow-auto right-[16px] top-[12px]" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[15.1px] text-white top-[8.5px] translate-y-[-50%] w-[49.14px]">
        <p className="leading-[normal]">8.8.4.4</p>
      </div>
    </div>
  );
}

function Input4() {
  return (
    <div className="absolute border border-[#3a3a3c] border-solid h-[44px] left-0 overflow-clip right-0 rounded-[8px] top-[397px]" data-name="Input">
      <Container6 />
    </div>
  );
}

function Button() {
  return (
    <div className="absolute bg-[#ff3b30] h-[42px] left-0 rounded-[8px] top-[465px] w-[133.38px]" data-name="Button">
      <div className="absolute flex flex-col font-['Inter:Semi_Bold',sans-serif] font-semibold h-[17px] justify-center leading-[0] left-[calc(50%+0.1px)] not-italic text-[15.9px] text-center text-white top-[calc(50%-0.5px)] translate-x-[-50%] translate-y-[-50%] w-[37.58px]">
        <p className="leading-[normal]">Save</p>
      </div>
    </div>
  );
}

function Form() {
  return (
    <div className="absolute h-[507px] left-[48px] right-[915.16px] top-[101px]" data-name="Form">
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[#a1a1a6] text-[14.4px] top-[8.5px] translate-y-[-50%] w-[73.733px]">
        <p className="leading-[normal]">IP Address</p>
      </div>
      <Input />
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[#a1a1a6] text-[14.4px] top-[101.5px] translate-y-[-50%] w-[89.117px]">
        <p className="leading-[normal]">Subnet Mask</p>
      </div>
      <Input1 />
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[#a1a1a6] text-[14.4px] top-[194.5px] translate-y-[-50%] w-[59.996px]">
        <p className="leading-[normal]">Gateway</p>
      </div>
      <Input2 />
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[#a1a1a6] text-[14.4px] top-[287.5px] translate-y-[-50%] w-[87.505px]">
        <p className="leading-[normal]">Primary DNS</p>
      </div>
      <Input3 />
      <div className="absolute flex flex-col font-['Inter:Regular',sans-serif] font-normal h-[17px] justify-center leading-[0] left-0 not-italic text-[#a1a1a6] text-[14.4px] top-[380.5px] translate-y-[-50%] w-[108.477px]">
        <p className="leading-[normal]">Secondary DNS</p>
      </div>
      <Input4 />
      <Button />
    </div>
  );
}

function Container7() {
  return (
    <div className="absolute inset-[0_0_0_356.84px] overflow-auto" data-name="Container">
      <div className="absolute flex flex-col font-['Inter:Semi_Bold',sans-serif] font-semibold h-[29px] justify-center leading-[0] left-[48px] not-italic text-[24px] text-white top-[62.5px] translate-y-[-50%] w-[201.749px]">
        <p className="leading-[normal]">Network Settings</p>
      </div>
      <Form />
    </div>
  );
}

function Nav({ activeNav, onNavChange }: { 
  activeNav: string; 
  onNavChange: (nav: string) => void;
}) {
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const [aboutModalOpen, setAboutModalOpen] = useState(false);
  const [changePasswordModalOpen, setChangePasswordModalOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [passwordSuccess, setPasswordSuccess] = useState(false);

  const handleSignOut = () => {
    window.location.reload();
  };

  const handleChangePassword = () => {
    setPasswordError('');
    setPasswordSuccess(false);

    if (!currentPassword) {
      setPasswordError('Current password is required');
      return;
    }
    if (!newPassword) {
      setPasswordError('New password is required');
      return;
    }
    if (newPassword.length < 6) {
      setPasswordError('Password must be at least 6 characters long');
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordError('New passwords do not match');
      return;
    }
    
    // Simulate password change success
    setPasswordSuccess(true);
    setTimeout(() => {
      setChangePasswordModalOpen(false);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      setPasswordError('');
      setPasswordSuccess(false);
    }, 1500);
  };

  return (
    <>
      <div className="absolute bg-[#121212] border-[#3a3a3c] border-[0px_1px_0px_0px] border-solid bottom-0 left-0 top-0 w-[76.84px]" data-name="Nav">
        {/* Navigation Links */}
        <div className="absolute top-[32px] left-0 right-0 flex flex-col items-center gap-[16px]">
          {/* Live Video Preview */}
          <button
            onClick={() => onNavChange('live')}
            className={`size-[48px] rounded-[12px] flex items-center justify-center transition-colors ${
              activeNav === 'live' ? 'border-[#ff3b30] border-[3px] border-solid' : ''
            }`}
          >
            <Video className="size-6" color={activeNav === 'live' ? '#FF3B30' : '#A1A1A6'} />
          </button>

          {/* Settings */}
          <button
            onClick={() => onNavChange('settings')}
            className={`size-[48px] rounded-[12px] flex items-center justify-center transition-colors ${
              activeNav === 'settings' ? 'border-[#ff3b30] border-[3px] border-solid' : ''
            }`}
          >
            <Settings className="size-6" color={activeNav === 'settings' ? '#FF3B30' : '#A1A1A6'} />
          </button>

          {/* Diagnostics/Statistics */}
          <button
            onClick={() => onNavChange('diagnostics')}
            className={`size-[48px] rounded-[12px] flex items-center justify-center transition-colors ${
              activeNav === 'diagnostics' ? 'border-[#ff3b30] border-[3px] border-solid' : ''
            }`}
          >
            <svg className="size-6" fill="none" viewBox="0 0 24 24" stroke={activeNav === 'diagnostics' ? '#FF3B30' : '#A1A1A6'} strokeWidth="2">
              <path strokeLinecap="round" strokeLinejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </button>
        </div>

        {/* User Profile Popup */}
        <div className="absolute bottom-[24px] left-[12px] right-[12px]">
          <button
            onClick={() => setUserMenuOpen(!userMenuOpen)}
            className="w-full size-[52px] rounded-[12px] bg-[#1c1c1e] border border-[#3a3a3c] flex items-center justify-center hover:bg-[#2c2c2e] transition-colors relative"
          >
            <div className="size-[32px] rounded-full bg-[#ff3b30] flex items-center justify-center">
              <span className="text-white font-semibold">A</span>
            </div>
          </button>

          {/* Popup Menu */}
          {userMenuOpen && (
            <div className="absolute bottom-[60px] left-0 w-[200px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] shadow-lg py-[8px] z-50">
              <div className="px-[16px] py-[12px] border-b border-[#3a3a3c]">
                <p className="text-white text-[14px] mb-[4px]">Admin User</p>
                <p className="text-[#a1a1a6] text-[12px]">admin@device.local</p>
              </div>
              <button 
                onClick={() => {
                  setChangePasswordModalOpen(true);
                  setUserMenuOpen(false);
                }}
                className="w-full px-[16px] py-[10px] text-left text-[#a1a1a6] text-[14px] hover:bg-[#2c2c2e] transition-colors"
              >
                Change Password
              </button>
              <button 
                onClick={() => {
                  setAboutModalOpen(true);
                  setUserMenuOpen(false);
                }}
                className="w-full px-[16px] py-[10px] text-left text-[#a1a1a6] text-[14px] hover:bg-[#2c2c2e] transition-colors"
              >
                About
              </button>
              <div className="border-t border-[#3a3a3c] mt-[4px] pt-[4px]">
                <button 
                  onClick={handleSignOut}
                  className="w-full px-[16px] py-[10px] text-left text-[#ff3b30] text-[14px] hover:bg-[#2c2c2e] transition-colors"
                >
                  Sign Out
                </button>
              </div>
            </div>
          )}
        </div>

        {/* About Modal */}
        {aboutModalOpen && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-[16px]">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] w-full max-w-[480px] max-h-[90vh] overflow-y-auto">
              {/* Build Time Header */}
              <div className="px-[20px] md:px-[24px] py-[12px] border-b border-[#3a3a3c] text-right">
                <span className="text-[#a1a1a6] text-[11px] md:text-[12px]">Build time: Dec 11 2024 - 10:24:15</span>
              </div>

              {/* Modal Content */}
              <div className="px-[20px] md:px-[24px] py-[20px] md:py-[24px]">
                {/* Logo and Version */}
                <div className="flex items-center gap-[12px] md:gap-[16px] mb-[16px] md:mb-[20px]">
                  <div className="size-[56px] md:size-[64px] rounded-[12px] bg-[#dc2626] flex items-center justify-center flex-shrink-0">
                    <Video className="size-7 md:size-8 text-white" />
                  </div>
                  <div>
                    <h2 className="text-white text-[18px] md:text-[20px] font-semibold mb-[4px]">IP Camera Pro 4K</h2>
                    <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">Firmware v2.4.1 (64-bit)</p>
                  </div>
                </div>

                {/* Home/Website */}
                <div className="mb-[16px] md:mb-[20px]">
                  <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Home: </span>
                  <a href="#" className="text-[#dc2626] text-[13px] md:text-[14px] hover:underline break-all">https://device.local/</a>
                </div>

                {/* License Section */}
                <div className="mb-[16px] md:mb-[20px]">
                  <h3 className="text-white text-[14px] font-semibold text-center mb-[12px]">Proprietary License</h3>
                  <div className="border border-[#3a3a3c] rounded-[8px] p-[12px] md:p-[16px] bg-[#0d0d0d] max-h-[200px] overflow-y-auto">
                    <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mb-[12px]">
                      This device and its software are provided under a proprietary license. 
                      Unauthorized copying, modification, or distribution is strictly prohibited.
                    </p>
                    <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mb-[12px]">
                      This device is provided "AS IS", <span className="text-white font-semibold">WITHOUT ANY WARRANTY</span>; 
                      without even the implied warranty of <span className="text-white font-semibold">MERCHANTABILITY</span> or{' '}
                      <span className="text-white font-semibold">FITNESS FOR A PARTICULAR PURPOSE</span>. See the End User 
                      License Agreement for more details.
                    </p>
                    <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6]">
                      © 2024 Device Management System. All rights reserved.
                    </p>
                    <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mt-[12px]">
                      Serial Number: SN-2024-12345<br />
                      MAC Address: 00:11:22:33:44:55
                    </p>
                  </div>
                </div>

                {/* OK Button */}
                <div className="flex justify-center">
                  <button 
                    onClick={() => setAboutModalOpen(false)}
                    className="px-[32px] py-[8px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[6px] transition-colors text-[14px] min-w-[100px]"
                  >
                    OK
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Change Password Modal */}
        {changePasswordModalOpen && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-[16px]">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] w-full max-w-[480px]">
              {/* Modal Content */}
              <div className="px-[20px] md:px-[24px] py-[20px] md:py-[24px]">
                <h2 className="text-white text-[18px] md:text-[20px] font-semibold mb-[16px]">Change Password</h2>

                {/* Current Password */}
                <div className="mb-[16px]">
                  <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">Current Password</label>
                  <input
                    type="password"
                    value={currentPassword}
                    onChange={(e) => setCurrentPassword(e.target.value)}
                    className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                  />
                </div>

                {/* New Password */}
                <div className="mb-[16px]">
                  <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">New Password</label>
                  <input
                    type="password"
                    value={newPassword}
                    onChange={(e) => setNewPassword(e.target.value)}
                    className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                  />
                </div>

                {/* Confirm Password */}
                <div className="mb-[16px]">
                  <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">Confirm Password</label>
                  <input
                    type="password"
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                  />
                </div>

                {/* Error Message */}
                {passwordError && (
                  <div className="text-[#ff3b30] text-[13px] md:text-[14px] mb-[16px]">
                    {passwordError}
                  </div>
                )}

                {/* Success Message */}
                {passwordSuccess && (
                  <div className="text-[#4ade80] text-[13px] md:text-[14px] mb-[16px]">
                    Password changed successfully!
                  </div>
                )}

                {/* Buttons */}
                <div className="flex flex-col sm:flex-row justify-end gap-[12px] sm:gap-0">
                  <button
                    onClick={() => setChangePasswordModalOpen(false)}
                    className="px-[32px] py-[8px] bg-[#3a3a3c] hover:bg-[#5a5a5c] text-[#a1a1a6] rounded-[6px] transition-colors text-[14px] min-w-[100px] sm:mr-[16px]"
                  >
                    Cancel
                  </button>
                  <button
                    onClick={handleChangePassword}
                    className="px-[32px] py-[8px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[6px] transition-colors text-[14px] min-w-[100px]"
                  >
                    Change Password
                  </button>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </>
  );
}

function Container9() {
  const [activeSection, setActiveSection] = useState('network');

  return (
    <div className="absolute h-[1200px] left-1/2 overflow-clip top-[calc(50%+0.5px)] translate-x-[-50%] translate-y-[-50%] w-[1920px]" data-name="Container">
      <BackgroundVerticalBorder activeSection={activeSection} onSectionChange={setActiveSection} />
      <Container7 />
      <Nav />
    </div>
  );
}

export default function Frame() {
  const [activeView, setActiveView] = useState('live'); // 'live', 'settings', 'diagnostics'
  const [activeSection, setActiveSection] = useState('network');
  const [settingsSidebarOpen, setSettingsSidebarOpen] = useState(false);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const [aboutModalOpen, setAboutModalOpen] = useState(false);
  const [changePasswordModalOpen, setChangePasswordModalOpen] = useState(false);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordError, setPasswordError] = useState('');
  const [passwordSuccess, setPasswordSuccess] = useState(false);

  const handleSignOut = () => {
    window.location.reload();
  };

  const handleChangePassword = () => {
    setPasswordError('');
    setPasswordSuccess(false);

    if (!currentPassword) {
      setPasswordError('Current password is required');
      return;
    }
    if (!newPassword) {
      setPasswordError('New password is required');
      return;
    }
    if (newPassword.length < 6) {
      setPasswordError('Password must be at least 6 characters long');
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordError('New passwords do not match');
      return;
    }
    
    // Simulate password change success
    setPasswordSuccess(true);
    setTimeout(() => {
      setChangePasswordModalOpen(false);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      setPasswordError('');
      setPasswordSuccess(false);
    }, 1500);
  };

  return (
    <div className="relative size-full overflow-hidden" data-name="Frame">
      <Background />
      
      {/* Desktop Navigation - Hidden on mobile */}
      <div className="hidden lg:block">
        <Nav activeNav={activeView} onNavChange={setActiveView} />
      </div>

      {/* Mobile Top Bar */}
      <div className="lg:hidden fixed top-0 left-0 right-0 h-[60px] bg-[#121212] border-b border-[#3a3a3c] z-40 flex items-center justify-between px-[16px]">
        <button
          onClick={() => setMobileNavOpen(!mobileNavOpen)}
          className="size-[40px] rounded-[8px] bg-[#1c1c1e] border border-[#3a3a3c] flex items-center justify-center"
        >
          <Menu className="size-5 text-white" />
        </button>
        <h1 className="text-white text-[16px] font-semibold">
          {activeView === 'live' && 'Live Video'}
          {activeView === 'settings' && 'Settings'}
          {activeView === 'diagnostics' && 'Diagnostics'}
        </h1>
        <button
          onClick={() => setUserMenuOpen(!userMenuOpen)}
          className="size-[40px] rounded-full bg-[#ff3b30] flex items-center justify-center relative"
        >
          <span className="text-white text-[14px] font-semibold">A</span>
        </button>

        {/* Mobile User Menu Dropdown */}
        {userMenuOpen && (
          <>
            <div 
              className="fixed inset-0 z-40"
              onClick={() => setUserMenuOpen(false)}
            />
            <div className="absolute top-[52px] right-[16px] w-[200px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] shadow-lg py-[8px] z-50">
              <div className="px-[16px] py-[12px] border-b border-[#3a3a3c]">
                <p className="text-white text-[14px] mb-[4px]">Admin User</p>
                <p className="text-[#a1a1a6] text-[12px]">admin@device.local</p>
              </div>
              <button 
                onClick={() => {
                  setChangePasswordModalOpen(true);
                  setUserMenuOpen(false);
                }}
                className="w-full px-[16px] py-[10px] text-left text-[#a1a1a6] text-[14px] hover:bg-[#2c2c2e] transition-colors"
              >
                Change Password
              </button>
              <button 
                onClick={() => {
                  setAboutModalOpen(true);
                  setUserMenuOpen(false);
                }}
                className="w-full px-[16px] py-[10px] text-left text-[#a1a1a6] text-[14px] hover:bg-[#2c2c2e] transition-colors"
              >
                About
              </button>
              <div className="border-t border-[#3a3a3c] mt-[4px] pt-[4px]">
                <button 
                  onClick={handleSignOut}
                  className="w-full px-[16px] py-[10px] text-left text-[#ff3b30] text-[14px] hover:bg-[#2c2c2e] transition-colors"
                >
                  Sign Out
                </button>
              </div>
            </div>
          </>
        )}
      </div>

      {/* Mobile Navigation Drawer */}
      {mobileNavOpen && (
        <>
          <div 
            className="lg:hidden fixed inset-0 bg-black/50 z-50"
            onClick={() => setMobileNavOpen(false)}
          />
          <div className="lg:hidden fixed top-0 left-0 bottom-0 w-[280px] bg-[#121212] border-r border-[#3a3a3c] z-50">
            <div className="flex items-center justify-between p-[16px] border-b border-[#3a3a3c]">
              <h2 className="text-white text-[18px] font-semibold">Menu</h2>
              <button
                onClick={() => setMobileNavOpen(false)}
                className="size-[32px] rounded-[8px] bg-[#1c1c1e] flex items-center justify-center"
              >
                <X className="size-5 text-white" />
              </button>
            </div>
            <div className="p-[16px] space-y-[8px]">
              <button
                onClick={() => {
                  setActiveView('live');
                  setMobileNavOpen(false);
                }}
                className={`w-full flex items-center gap-[12px] px-[16px] py-[12px] rounded-[8px] transition-colors ${
                  activeView === 'live' ? 'bg-[#dc2626] text-white' : 'text-[#a1a1a6] hover:bg-[#1c1c1e]'
                }`}
              >
                <Video className="size-5" />
                <span className="text-[15px]">Live Video Preview</span>
              </button>
              <button
                onClick={() => {
                  setActiveView('settings');
                  setMobileNavOpen(false);
                }}
                className={`w-full flex items-center gap-[12px] px-[16px] py-[12px] rounded-[8px] transition-colors ${
                  activeView === 'settings' ? 'bg-[#dc2626] text-white' : 'text-[#a1a1a6] hover:bg-[#1c1c1e]'
                }`}
              >
                <Settings className="size-5" />
                <span className="text-[15px]">Settings</span>
              </button>
              <button
                onClick={() => {
                  setActiveView('diagnostics');
                  setMobileNavOpen(false);
                }}
                className={`w-full flex items-center gap-[12px] px-[16px] py-[12px] rounded-[8px] transition-colors ${
                  activeView === 'diagnostics' ? 'bg-[#dc2626] text-white' : 'text-[#a1a1a6] hover:bg-[#1c1c1e]'
                }`}
              >
                <svg className="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                  <path strokeLinecap="round" strokeLinejoin="round" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                </svg>
                <span className="text-[15px]">Diagnostics & Statistics</span>
              </button>
            </div>
          </div>
        </>
      )}

      {/* Settings Sidebar - Responsive */}
      {activeView === 'settings' && (
        <>
          {/* Desktop Settings Sidebar */}
          <div className="hidden lg:block">
            <BackgroundVerticalBorder 
              activeSection={activeSection}
              onSectionChange={setActiveSection}
            />
          </div>

          {/* Mobile Settings Selector */}
          <div className="lg:hidden fixed top-[60px] left-0 right-0 bg-[#1c1c1e] border-b border-[#3a3a3c] z-30 p-[16px]">
            <button
              onClick={() => setSettingsSidebarOpen(!settingsSidebarOpen)}
              className="w-full px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] flex items-center justify-between"
            >
              <span>
                {activeSection === 'network' && 'Network'}
                {activeSection === 'identification' && 'Identification'}
                {activeSection === 'time' && 'Time Settings'}
                {activeSection === 'maintenance' && 'Maintenance'}
                {activeSection === 'video' && 'Imaging Settings'}
                {activeSection === 'users' && 'User Management'}
                {activeSection === 'profiles' && 'Profiles'}
              </span>
              <svg className="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
              </svg>
            </button>
          </div>

          {/* Mobile Settings Dropdown */}
          {settingsSidebarOpen && (
            <>
              <div 
                className="lg:hidden fixed inset-0 bg-black/50 z-40 top-[120px]"
                onClick={() => setSettingsSidebarOpen(false)}
              />
              <div className="lg:hidden fixed top-[120px] left-[16px] right-[16px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[8px] z-50 max-h-[400px] overflow-y-auto">
                {[
                  { id: 'identification', icon: User, title: 'Identification' },
                  { id: 'network', icon: Network, title: 'Network' },
                  { id: 'time', icon: Clock, title: 'Time Settings' },
                  { id: 'maintenance', icon: Wrench, title: 'Maintenance' },
                  { id: 'video', icon: Video, title: 'Imaging Settings' },
                  { id: 'users', icon: Users, title: 'User Management' },
                  { id: 'profiles', icon: Image, title: 'Profiles' },
                ].map((section) => (
                  <button
                    key={section.id}
                    onClick={() => {
                      setActiveSection(section.id);
                      setSettingsSidebarOpen(false);
                    }}
                    className={`w-full flex items-center gap-[12px] px-[16px] py-[12px] border-b border-[#3a3a3c] last:border-b-0 transition-colors ${
                      activeSection === section.id ? 'bg-[#dc262620] text-white' : 'text-[#a1a1a6] hover:bg-[#2c2c2e]'
                    }`}
                  >
                    <section.icon className="size-5" />
                    <span className="text-[14px]">{section.title}</span>
                  </button>
                ))}
              </div>
            </>
          )}
        </>
      )}

      {/* Main Content Area - Responsive */}
      <div className={`
        ${activeView === 'settings' ? 'lg:ml-[356.84px]' : 'lg:ml-[76.84px]'}
        ${activeView === 'settings' ? 'mt-[120px]' : 'mt-[60px]'}
        lg:mt-0
        ${activeView === 'settings' ? 'h-[calc(100vh-120px)]' : 'h-[calc(100vh-60px)]'}
        lg:h-screen
        overflow-auto
      `}>
        {activeView === 'settings' && (
          <>
            {activeSection === 'network' && <NetworkSettings />}
            {activeSection === 'identification' && <IdentificationSettings />}
            {activeSection === 'time' && <TimeSettings />}
            {activeSection === 'maintenance' && <MaintenanceSettings />}
            {activeSection === 'users' && <UserManagement />}
            {activeSection === 'profiles' && <ProfilesManagement />}
            {activeSection === 'video' && <ImagingSettings />}
          </>
        )}
        {activeView === 'live' && <LiveVideoPreview />}
        {activeView === 'diagnostics' && <DiagnosticsStatistics />}
      </div>

      {/* About Modal */}
      {aboutModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-[16px]">
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] w-full max-w-[480px] max-h-[90vh] overflow-y-auto">
            {/* Build Time Header */}
            <div className="px-[20px] md:px-[24px] py-[12px] border-b border-[#3a3a3c] text-right">
              <span className="text-[#a1a1a6] text-[11px] md:text-[12px]">Build time: Dec 11 2024 - 10:24:15</span>
            </div>

            {/* Modal Content */}
            <div className="px-[20px] md:px-[24px] py-[20px] md:py-[24px]">
              {/* Logo and Version */}
              <div className="flex items-center gap-[12px] md:gap-[16px] mb-[16px] md:mb-[20px]">
                <div className="size-[56px] md:size-[64px] rounded-[12px] bg-[#dc2626] flex items-center justify-center flex-shrink-0">
                  <Video className="size-7 md:size-8 text-white" />
                </div>
                <div>
                  <h2 className="text-white text-[18px] md:text-[20px] font-semibold mb-[4px]">IP Camera Pro 4K</h2>
                  <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">Firmware v2.4.1 (64-bit)</p>
                </div>
              </div>

              {/* Home/Website */}
              <div className="mb-[16px] md:mb-[20px]">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Home: </span>
                <a href="#" className="text-[#dc2626] text-[13px] md:text-[14px] hover:underline break-all">https://device.local/</a>
              </div>

              {/* License Section */}
              <div className="mb-[16px] md:mb-[20px]">
                <h3 className="text-white text-[14px] font-semibold text-center mb-[12px]">Proprietary License</h3>
                <div className="border border-[#3a3a3c] rounded-[8px] p-[12px] md:p-[16px] bg-[#0d0d0d] max-h-[200px] overflow-y-auto">
                  <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mb-[12px]">
                    This device and its software are provided under a proprietary license. 
                    Unauthorized copying, modification, or distribution is strictly prohibited.
                  </p>
                  <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mb-[12px]">
                    This device is provided "AS IS", <span className="text-white font-semibold">WITHOUT ANY WARRANTY</span>; 
                    without even the implied warranty of <span className="text-white font-semibold">MERCHANTABILITY</span> or{' '}
                    <span className="text-white font-semibold">FITNESS FOR A PARTICULAR PURPOSE</span>. See the End User 
                    License Agreement for more details.
                  </p>
                  <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6]">
                    © 2024 Device Management System. All rights reserved.
                  </p>
                  <p className="text-[#a1a1a6] text-[12px] md:text-[13px] leading-[1.6] mt-[12px]">
                    Serial Number: SN-2024-12345<br />
                    MAC Address: 00:11:22:33:44:55
                  </p>
                </div>
              </div>

              {/* OK Button */}
              <div className="flex justify-center">
                <button 
                  onClick={() => setAboutModalOpen(false)}
                  className="px-[32px] py-[8px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[6px] transition-colors text-[14px] min-w-[100px]"
                >
                  OK
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Change Password Modal */}
      {changePasswordModalOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-[100] p-[16px]">
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] w-full max-w-[480px]">
            {/* Modal Content */}
            <div className="px-[20px] md:px-[24px] py-[20px] md:py-[24px]">
              <h2 className="text-white text-[18px] md:text-[20px] font-semibold mb-[16px]">Change Password</h2>

              {/* Current Password */}
              <div className="mb-[16px]">
                <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">Current Password</label>
                <input
                  type="password"
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                />
              </div>

              {/* New Password */}
              <div className="mb-[16px]">
                <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">New Password</label>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                />
              </div>

              {/* Confirm Password */}
              <div className="mb-[16px]">
                <label className="text-[#a1a1a6] text-[13px] md:text-[14px] block mb-[4px]">Confirm Password</label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className="w-full px-[16px] py-[8px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[6px] text-[#a1a1a6] text-[14px] focus:outline-none focus:border-[#ff3b30]"
                />
              </div>

              {/* Error Message */}
              {passwordError && (
                <div className="text-[#ff3b30] text-[13px] md:text-[14px] mb-[16px]">
                  {passwordError}
                </div>
              )}

              {/* Success Message */}
              {passwordSuccess && (
                <div className="text-[#4ade80] text-[13px] md:text-[14px] mb-[16px]">
                  Password changed successfully!
                </div>
              )}

              {/* Buttons */}
              <div className="flex flex-col sm:flex-row justify-end gap-[12px] sm:gap-0">
                <button
                  onClick={() => setChangePasswordModalOpen(false)}
                  className="px-[32px] py-[8px] bg-[#3a3a3c] hover:bg-[#5a5a5c] text-[#a1a1a6] rounded-[6px] transition-colors text-[14px] min-w-[100px] sm:mr-[16px]"
                >
                  Cancel
                </button>
                <button
                  onClick={handleChangePassword}
                  className="px-[32px] py-[8px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[6px] transition-colors text-[14px] min-w-[100px]"
                >
                  Change Password
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}