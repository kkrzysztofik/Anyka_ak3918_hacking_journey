import { useState } from 'react';
import { 
  Download, 
  Upload, 
  RotateCcw, 
  Power, 
  AlertTriangle, 
  CheckCircle2, 
  RefreshCw,
  HardDrive,
  Zap
} from 'lucide-react';

export default function MaintenanceSettings() {
  const [showSoftResetConfirm, setShowSoftResetConfirm] = useState(false);
  const [showHardResetConfirm, setShowHardResetConfirm] = useState(false);
  const [showRebootConfirm, setShowRebootConfirm] = useState(false);
  const [actionStatus, setActionStatus] = useState<'idle' | 'processing'>('idle');

  const currentFirmware = 'V5.05.R02.000A07F3.10010.346532.S.ONVIF 21.06';

  const handleBackup = () => {
    setActionStatus('processing');
    // Simulate backup process
    setTimeout(() => {
      setActionStatus('idle');
      // Trigger download
      alert('Configuration backup downloaded');
    }, 2000);
  };

  const handleRestore = () => {
    // Trigger file input
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.cfg,.conf,.config';
    input.onchange = (e: any) => {
      const file = e.target.files[0];
      if (file) {
        setActionStatus('processing');
        setTimeout(() => {
          setActionStatus('idle');
          alert('Configuration restored successfully');
        }, 2000);
      }
    };
    input.click();
  };

  const handleSoftReset = () => {
    setActionStatus('processing');
    setTimeout(() => {
      setActionStatus('idle');
      setShowSoftResetConfirm(false);
      alert('Soft reset completed');
    }, 3000);
  };

  const handleHardReset = () => {
    setActionStatus('processing');
    setTimeout(() => {
      setActionStatus('idle');
      setShowHardResetConfirm(false);
      alert('Hard reset completed - device will restart');
    }, 3000);
  };

  const handleReboot = () => {
    setActionStatus('processing');
    setTimeout(() => {
      setActionStatus('idle');
      setShowRebootConfirm(false);
      alert('Device is rebooting...');
    }, 2000);
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]" data-name="Container">
      <div className="max-w-[900px] p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Maintenance</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            System maintenance, backup, restore, and firmware updates
          </p>
        </div>

        {/* Configuration Backup & Restore */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <div className="flex items-center gap-[16px] mb-[24px]">
            <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
              <HardDrive className="size-6" color="#dc2626" />
            </div>
            <div>
              <h3 className="text-white text-[18px] mb-[4px]">Configuration</h3>
              <p className="text-[#a1a1a6] text-[14px]">Backup and restore device settings</p>
            </div>
          </div>

          <div className="flex gap-[16px]">
            <button
              onClick={handleBackup}
              disabled={actionStatus === 'processing'}
              className="h-[44px] px-[24px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-[8px]"
            >
              <Download className="size-4" />
              Backup
            </button>
            
            <button
              onClick={handleRestore}
              disabled={actionStatus === 'processing'}
              className="h-[44px] px-[24px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] hover:text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-[8px]"
            >
              <Upload className="size-4" />
              Restore
            </button>

            <div className="flex items-center px-[16px] text-[#6b6b6f] text-[14px]">
              Unsupported
            </div>
          </div>
        </div>

        {/* Soft Factory Reset */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[48px] bg-[rgba(255,159,10,0.1)] rounded-[12px] flex items-center justify-center">
                <RotateCcw className="size-6" color="#ff9f0a" />
              </div>
              <div>
                <h3 className="text-white text-[18px] mb-[4px]">Soft factory reset</h3>
                <p className="text-[#a1a1a6] text-[14px]">
                  Reset settings to default values, preserving network configuration
                </p>
              </div>
            </div>

            <button
              onClick={() => setShowSoftResetConfirm(true)}
              disabled={actionStatus === 'processing'}
              className="h-[44px] px-[24px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] hover:text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Soft reset
            </button>
          </div>
        </div>

        {/* Hard Factory Reset */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                <AlertTriangle className="size-6" color="#dc2626" />
              </div>
              <div>
                <h3 className="text-white text-[18px] mb-[4px]">Hard factory reset</h3>
                <p className="text-[#a1a1a6] text-[14px]">
                  Reset all settings to factory defaults including network configuration
                </p>
              </div>
            </div>

            <button
              onClick={() => setShowHardResetConfirm(true)}
              disabled={actionStatus === 'processing'}
              className="h-[44px] px-[24px] bg-transparent border border-[#dc2626] rounded-[8px] text-[#dc2626] font-semibold text-[15px] hover:bg-[rgba(220,38,38,0.1)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Hard reset
            </button>
          </div>
        </div>

        {/* Reboot Device */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-[16px]">
              <div className="size-[48px] bg-[rgba(0,122,255,0.1)] rounded-[12px] flex items-center justify-center">
                <Power className="size-6" color="#007AFF" />
              </div>
              <div>
                <h3 className="text-white text-[18px] mb-[4px]">Reboot device</h3>
                <p className="text-[#a1a1a6] text-[14px]">
                  Restart the device without changing any settings
                </p>
              </div>
            </div>

            <button
              onClick={() => setShowRebootConfirm(true)}
              disabled={actionStatus === 'processing'}
              className="h-[44px] px-[24px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] hover:text-white transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Reboot
            </button>
          </div>
        </div>

        {/* Firmware */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] mb-[24px]">
          <div className="flex items-center gap-[16px] mb-[24px]">
            <div className="size-[48px] bg-[rgba(52,199,89,0.1)] rounded-[12px] flex items-center justify-center">
              <Zap className="size-6" color="#34c759" />
            </div>
            <div>
              <h3 className="text-white text-[18px] mb-[4px]">Firmware</h3>
              <p className="text-[#a1a1a6] text-[14px]">Manage firmware updates and version</p>
            </div>
          </div>

          <div className="flex items-center gap-[16px]">
            <button
              disabled
              className="h-[44px] px-[24px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#6b6b6f] font-semibold text-[15px] cursor-not-allowed opacity-50"
            >
              Upgrade
            </button>

            <div className="flex-1 h-[44px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] px-[16px] flex items-center">
              <p className="text-[#6b6b6f] text-[14px]">{currentFirmware}</p>
            </div>

            <div className="flex items-center px-[16px] text-[#6b6b6f] text-[14px]">
              Unsupported
            </div>
          </div>
        </div>

        {/* Warning Info */}
        <div className="p-[16px] bg-[rgba(220,38,38,0.05)] border border-[rgba(220,38,38,0.2)] rounded-[8px]">
          <div className="flex gap-[12px]">
            <AlertTriangle className="size-5 text-[#dc2626] flex-shrink-0 mt-[2px]" />
            <div>
              <p className="text-[#dc2626] text-[14px] mb-[4px]">Important</p>
              <p className="text-[#a1a1a6] text-[13px]">
                Factory reset and reboot operations will interrupt device operation. Ensure no critical recordings or monitoring is in progress before proceeding.
              </p>
            </div>
          </div>
        </div>

        {/* Soft Reset Confirmation Modal */}
        {showSoftResetConfirm && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(255,159,10,0.1)] rounded-[12px] flex items-center justify-center">
                  <AlertTriangle className="size-6" color="#ff9f0a" />
                </div>
                <h3 className="text-white text-[20px]">Confirm Soft Reset</h3>
              </div>

              <p className="text-[#a1a1a6] text-[15px] mb-[24px]">
                This will reset all settings to default values while preserving network configuration. The device will restart after the reset. Continue?
              </p>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleSoftReset}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#ff9f0a] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ffb340] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Resetting...
                    </>
                  ) : (
                    'Confirm Reset'
                  )}
                </button>
                <button
                  onClick={() => setShowSoftResetConfirm(false)}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Hard Reset Confirmation Modal */}
        {showHardResetConfirm && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(220,38,38,0.1)] rounded-[12px] flex items-center justify-center">
                  <AlertTriangle className="size-6" color="#dc2626" />
                </div>
                <h3 className="text-white text-[20px]">Confirm Hard Reset</h3>
              </div>

              <p className="text-[#a1a1a6] text-[15px] mb-[24px]">
                <strong className="text-[#dc2626]">Warning:</strong> This will reset ALL settings to factory defaults including network configuration. You will need to reconfigure the device from scratch. This action cannot be undone. Continue?
              </p>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleHardReset}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#dc2626] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#ef4444] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Resetting...
                    </>
                  ) : (
                    'Confirm Hard Reset'
                  )}
                </button>
                <button
                  onClick={() => setShowHardResetConfirm(false)}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] font-semibold text-[15px] hover:bg-[#2c2c2e] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Reboot Confirmation Modal */}
        {showRebootConfirm && (
          <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[32px] max-w-[480px] w-full mx-[24px]">
              <div className="flex items-center gap-[16px] mb-[24px]">
                <div className="size-[48px] bg-[rgba(0,122,255,0.1)] rounded-[12px] flex items-center justify-center">
                  <Power className="size-6" color="#007AFF" />
                </div>
                <h3 className="text-white text-[20px]">Confirm Reboot</h3>
              </div>

              <p className="text-[#a1a1a6] text-[15px] mb-[24px]">
                The device will restart. This may take up to 2 minutes. All settings will be preserved. Continue?
              </p>

              <div className="flex gap-[16px]">
                <button
                  onClick={handleReboot}
                  disabled={actionStatus === 'processing'}
                  className="flex-1 h-[44px] bg-[#007AFF] rounded-[8px] text-white font-semibold text-[15px] hover:bg-[#0066CC] transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-[8px]"
                >
                  {actionStatus === 'processing' ? (
                    <>
                      <RefreshCw className="size-4 animate-spin" />
                      Rebooting...
                    </>
                  ) : (
                    'Confirm Reboot'
                  )}
                </button>
                <button
                  onClick={() => setShowRebootConfirm(false)}
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