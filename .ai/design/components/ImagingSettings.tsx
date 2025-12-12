import { useState } from 'react';
import { Camera, Sun, Aperture, Focus } from 'lucide-react';

export default function ImagingSettings() {
  const [focusMode, setFocusMode] = useState('auto');
  const [brightness, setBrightness] = useState(50.00);
  const [colorSaturation, setColorSaturation] = useState(50.00);
  const [contrast, setContrast] = useState(50.00);
  const [sharpness, setSharpness] = useState(10.00);
  const [whiteBalanceMode, setWhiteBalanceMode] = useState('auto');
  const [whiteBalanceCb, setWhiteBalanceCb] = useState(10.00);
  const [whiteBalanceCr, setWhiteBalanceCr] = useState(10.00);
  const [backlightCompensationMode, setBacklightCompensationMode] = useState('off');
  const [backlightCompensationLevel, setBacklightCompensationLevel] = useState(10.00);
  const [wideDynamicRangeMode, setWideDynamicRangeMode] = useState('off');
  const [wideDynamicRangeLevel, setWideDynamicRangeLevel] = useState(50.00);
  const [exposureMode, setExposureMode] = useState('auto');
  const [exposureGain, setExposureGain] = useState(0.00);
  const [exposureMinGain, setExposureMinGain] = useState(0.00);
  const [exposureMaxGain, setExposureMaxGain] = useState(100.00);
  const [exposureTime, setExposureTime] = useState(4000.00);
  const [exposureMinTime, setExposureMinTime] = useState(10.00);
  const [exposureMaxTime, setExposureMaxTime] = useState(40000.00);
  const [exposureMinIris, setExposureMinIris] = useState(0.00);
  const [exposureMaxIris, setExposureMaxIris] = useState(10.00);
  const [infraredCutoffFilter, setInfraredCutoffFilter] = useState('auto');

  const handleApply = () => {
    console.log('Applying imaging settings...');
    // Here you would typically send the settings to the device
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_356.84px] overflow-auto bg-[#0d0d0d]">
      <div className="max-w-[900px] p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] md:mb-[40px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Imaging Settings</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Configure focus, exposure, and image quality settings
          </p>
        </div>

        {/* Focus Settings Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Focus className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">Focus & Sharpness</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Focus Mode</label>
              <select
                value={focusMode}
                onChange={(e) => setFocusMode(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="auto">Auto</option>
                <option value="manual">Manual</option>
              </select>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Sharpness</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{sharpness.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={sharpness}
                onChange={(e) => setSharpness(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>
          </div>
        </div>

        {/* Color Settings Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Sun className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">Color & Brightness</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Brightness</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{brightness.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={brightness}
                onChange={(e) => setBrightness(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Color Saturation</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{colorSaturation.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={colorSaturation}
                onChange={(e) => setColorSaturation(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Contrast</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{contrast.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={contrast}
                onChange={(e) => setContrast(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>
          </div>
        </div>

        {/* White Balance Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Aperture className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">White Balance</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">White Balance Mode</label>
              <select
                value={whiteBalanceMode}
                onChange={(e) => setWhiteBalanceMode(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="auto">Auto</option>
                <option value="manual">Manual</option>
              </select>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">White Balance Cb</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{whiteBalanceCb.toFixed(2)}</span>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">White Balance Cr</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{whiteBalanceCr.toFixed(2)}</span>
            </div>
          </div>
        </div>

        {/* Backlight & WDR Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Camera className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">Backlight & Wide Dynamic Range</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Backlight Compensation</label>
              <select
                value={backlightCompensationMode}
                onChange={(e) => setBacklightCompensationMode(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="off">Off</option>
                <option value="on">On</option>
              </select>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Compensation Level</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{backlightCompensationLevel.toFixed(2)}</span>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Wide Dynamic Range</label>
              <select
                value={wideDynamicRangeMode}
                onChange={(e) => setWideDynamicRangeMode(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="off">Off</option>
                <option value="on">On</option>
              </select>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">WDR Level</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{wideDynamicRangeLevel.toFixed(2)}</span>
            </div>
          </div>
        </div>

        {/* Exposure Settings Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Aperture className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">Exposure Settings</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Exposure Mode</label>
              <select
                value={exposureMode}
                onChange={(e) => setExposureMode(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="auto">Auto</option>
                <option value="manual">Manual</option>
              </select>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Exposure Gain</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureGain.toFixed(2)}</span>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Min Gain</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMinGain.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={exposureMinGain}
                onChange={(e) => setExposureMinGain(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Max Gain</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMaxGain.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={exposureMaxGain}
                onChange={(e) => setExposureMaxGain(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Exposure Time</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureTime.toFixed(2)}</span>
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Min Time</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMinTime.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="10000"
                step="0.01"
                value={exposureMinTime}
                onChange={(e) => setExposureMinTime(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Max Time</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMaxTime.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100000"
                step="0.01"
                value={exposureMaxTime}
                onChange={(e) => setExposureMaxTime(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Min Iris</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMinIris.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={exposureMinIris}
                onChange={(e) => setExposureMinIris(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>

            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Max Iris</label>
              <span className="text-[#ff8c00] text-[14px] w-[60px] text-right font-mono">{exposureMaxIris.toFixed(2)}</span>
              <input
                type="range"
                min="0"
                max="100"
                step="0.01"
                value={exposureMaxIris}
                onChange={(e) => setExposureMaxIris(parseFloat(e.target.value))}
                className="flex-1 h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
              />
            </div>
          </div>
        </div>

        {/* Infrared Filter Card */}
        <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[24px] mb-[24px]">
          <div className="flex items-center gap-[12px] mb-[20px]">
            <Camera className="size-5 text-[#dc2626]" />
            <h2 className="text-white text-[18px]">Infrared Settings</h2>
          </div>
          
          <div className="space-y-[20px]">
            <div className="flex items-center gap-[24px]">
              <label className="text-[#a1a1a6] text-[14px] w-[180px]">Infrared Cutoff Filter</label>
              <select
                value={infraredCutoffFilter}
                onChange={(e) => setInfraredCutoffFilter(e.target.value)}
                className="px-[16px] py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-white text-[14px] min-w-[200px] focus:outline-none focus:border-[#dc2626]"
              >
                <option value="auto">Auto</option>
                <option value="on">On</option>
                <option value="off">Off</option>
              </select>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-[16px] items-center">
          <button
            onClick={handleApply}
            className="px-[32px] py-[12px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[8px] transition-colors font-semibold"
          >
            Apply Settings
          </button>
          <button
            onClick={() => {
              // Reset to defaults
              setBrightness(50.00);
              setColorSaturation(50.00);
              setContrast(50.00);
              setSharpness(10.00);
            }}
            className="px-[32px] py-[12px] bg-[#3a3a3c] hover:bg-[#5a5a5c] text-[#a1a1a6] rounded-[8px] transition-colors"
          >
            Reset to Defaults
          </button>
        </div>
      </div>
    </div>
  );
}