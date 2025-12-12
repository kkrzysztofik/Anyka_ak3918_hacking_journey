import { useState } from 'react';
import { Play, Pause, Volume2, VolumeX, Maximize, Camera, Wifi, Clock, Aperture, Grid3x3, ZoomIn, ZoomOut, Focus, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Home, Bookmark } from 'lucide-react';

export default function LiveVideoPreview() {
  const [isPlaying, setIsPlaying] = useState(true);
  const [isMuted, setIsMuted] = useState(true);
  const [showGrid, setShowGrid] = useState(false);
  const [selectedStream, setSelectedStream] = useState('main');
  const [ptzSpeed, setPtzSpeed] = useState(50);
  const [presets, setPresets] = useState([
    { id: 1, name: 'Entrance', active: false },
    { id: 2, name: 'Parking Lot', active: false },
    { id: 3, name: 'Exit', active: false },
  ]);

  const handlePTZMove = (direction: string) => {
    console.log(`PTZ Move: ${direction} at speed ${ptzSpeed}`);
    // Here you would send PTZ command to the camera
  };

  const handleZoom = (direction: 'in' | 'out') => {
    console.log(`Zoom ${direction}`);
    // Here you would send zoom command
  };

  const handleFocus = (direction: 'near' | 'far') => {
    console.log(`Focus ${direction}`);
    // Here you would send focus command
  };

  const handleGoToPreset = (presetId: number) => {
    console.log(`Go to preset ${presetId}`);
    // Here you would send go to preset command
  };

  const handleSavePreset = (presetId: number) => {
    console.log(`Save current position to preset ${presetId}`);
    // Here you would save current position
  };

  return (
    <div className="absolute inset-0 lg:inset-[0_0_0_76.84px] overflow-auto bg-[#0d0d0d]">
      <div className="p-[16px] md:p-[32px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[24px] lg:mb-[32px]">
          <h1 className="text-white text-[22px] md:text-[28px] mb-[8px]">Live Video Preview</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px]">
            Real-time ONVIF stream monitoring and playback controls
          </p>
        </div>

        {/* Main Content - Video Player with Side Controls */}
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-[16px] md:gap-[24px] mb-[16px] md:mb-[24px]">
          {/* Left Column - Video Player */}
          <div className="space-y-[16px] md:space-y-[24px]">
            {/* Video Player Card */}
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[24px]">
              {/* Stream Selection */}
              <div className="flex flex-col sm:flex-row items-start sm:items-center gap-[12px] sm:gap-[16px] mb-[16px] md:mb-[20px]">
                <div className="flex items-center gap-[12px]">
                  <Camera className="size-5 text-[#dc2626]" />
                  <h2 className="text-white text-[16px] md:text-[18px]">Video Stream</h2>
                </div>
                <div className="flex gap-[8px] w-full sm:w-auto sm:ml-auto">
                  <button
                    onClick={() => setSelectedStream('main')}
                    className={`flex-1 sm:flex-none px-[16px] py-[6px] rounded-[6px] text-[14px] transition-colors ${
                      selectedStream === 'main'
                        ? 'bg-[#dc2626] text-white'
                        : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                    }`}
                  >
                    Main Stream
                  </button>
                  <button
                    onClick={() => setSelectedStream('sub')}
                    className={`flex-1 sm:flex-none px-[16px] py-[6px] rounded-[6px] text-[14px] transition-colors ${
                      selectedStream === 'sub'
                        ? 'bg-[#dc2626] text-white'
                        : 'bg-[#3a3a3c] text-[#a1a1a6] hover:bg-[#4a4a4c]'
                    }`}
                  >
                    Sub Stream
                  </button>
                </div>
              </div>

              {/* Video Player */}
              <div className="relative bg-black rounded-[8px] overflow-hidden aspect-video mb-[16px] md:mb-[20px]">
                {/* Simulated Video Feed */}
                <div className="absolute inset-0 bg-gradient-to-br from-[#1a1a1a] to-[#0a0a0a] flex items-center justify-center">
                  {/* Grid Overlay */}
                  {showGrid && (
                    <div className="absolute inset-0 grid grid-cols-3 grid-rows-3">
                      {[...Array(9)].map((_, i) => (
                        <div key={i} className="border border-[#ffffff20]"></div>
                      ))}
                    </div>
                  )}
                  
                  {/* Camera Icon */}
                  <div className="relative z-10 flex flex-col items-center gap-[16px]">
                    <Camera className="size-[48px] md:size-[64px] text-[#3a3a3c]" />
                    <div className="text-center">
                      <p className="text-[#a1a1a6] text-[14px] md:text-[16px] mb-[4px]">ONVIF Stream Preview</p>
                      <p className="text-[#636366] text-[11px] md:text-[12px]">1920×1080 @ 30fps</p>
                    </div>
                  </div>

                  {/* Status Indicators */}
                  <div className="absolute top-[12px] md:top-[16px] left-[12px] md:left-[16px] flex items-center gap-[8px] px-[10px] md:px-[12px] py-[6px] md:py-[8px] bg-black/50 backdrop-blur-sm rounded-[6px]">
                    <div className="size-[8px] rounded-full bg-[#ff3b30] animate-pulse"></div>
                    <span className="text-white text-[11px] md:text-[12px] font-medium">LIVE</span>
                  </div>
                  <div className="absolute top-[12px] md:top-[16px] right-[12px] md:right-[16px] flex items-center gap-[8px] px-[10px] md:px-[12px] py-[6px] md:py-[8px] bg-black/50 backdrop-blur-sm rounded-[6px]">
                    <Wifi className="size-3 md:size-4 text-[#34c759]" />
                    <span className="text-white text-[11px] md:text-[12px] font-medium">Connected</span>
                  </div>

                  {/* Playback Controls */}
                  <div className="absolute bottom-[12px] md:bottom-[16px] left-[12px] md:left-[16px] right-[12px] md:right-[16px] flex items-center justify-between">
                    <div className="flex items-center gap-[8px]">
                      <button
                        onClick={() => setIsPlaying(!isPlaying)}
                        className="size-[36px] md:size-[40px] rounded-full bg-[#dc2626] hover:bg-[#b91c1c] flex items-center justify-center transition-colors"
                      >
                        {isPlaying ? (
                          <Pause className="size-4 md:size-5 text-white fill-white" />
                        ) : (
                          <Play className="size-4 md:size-5 text-white fill-white ml-[2px]" />
                        )}
                      </button>
                      <button
                        onClick={() => setIsMuted(!isMuted)}
                        className="size-[32px] md:size-[36px] rounded-full bg-black/50 hover:bg-black/70 backdrop-blur-sm flex items-center justify-center transition-colors"
                      >
                        {isMuted ? (
                          <VolumeX className="size-4 text-white" />
                        ) : (
                          <Volume2 className="size-4 text-white" />
                        )}
                      </button>
                      <button
                        onClick={() => setShowGrid(!showGrid)}
                        className={`size-[32px] md:size-[36px] rounded-full backdrop-blur-sm flex items-center justify-center transition-colors ${
                          showGrid ? 'bg-[#dc2626]' : 'bg-black/50 hover:bg-black/70'
                        }`}
                      >
                        <Grid3x3 className="size-4 text-white" />
                      </button>
                    </div>
                    <button className="size-[32px] md:size-[36px] rounded-full bg-black/50 hover:bg-black/70 backdrop-blur-sm flex items-center justify-center transition-colors">
                      <Maximize className="size-4 text-white" />
                    </button>
                  </div>
                </div>
              </div>

              {/* Stream URL */}
              <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-[12px] sm:gap-[16px]">
                <label className="text-[#a1a1a6] text-[14px] sm:w-[100px]">Stream URL</label>
                <input
                  type="text"
                  value={`rtsp://192.168.1.100:554/${selectedStream}`}
                  readOnly
                  className="flex-1 px-[12px] md:px-[16px] py-[8px] md:py-[10px] bg-[#0d0d0d] border border-[#3a3a3c] rounded-[8px] text-[#a1a1a6] text-[13px] md:text-[14px] font-mono"
                />
                <button className="px-[16px] md:px-[20px] py-[8px] md:py-[10px] bg-[#3a3a3c] hover:bg-[#4a4a4c] text-white rounded-[8px] transition-colors text-[13px] md:text-[14px]">
                  Copy
                </button>
              </div>
            </div>
          </div>

          {/* Right Column - PTZ Controls (shows on desktop) */}
          <div className="hidden lg:flex lg:flex-col space-y-[16px] md:space-y-[24px]">
            {/* PTZ Directional Controls */}
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
              <div className="flex items-center gap-[12px] mb-[16px]">
                <Camera className="size-5 text-[#dc2626]" />
                <h3 className="text-white text-[15px] md:text-[16px]">Pan & Tilt</h3>
              </div>
              
              <div className="flex flex-col items-center gap-[4px]">
                {/* Top Row */}
                <div className="flex gap-[4px]">
                  <button
                    onMouseDown={() => handlePTZMove('up-left')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-[-45deg]">
                      <ArrowUp className="size-4 md:size-5 text-white" />
                    </div>
                  </button>
                  <button
                    onMouseDown={() => handlePTZMove('up')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <ArrowUp className="size-4 md:size-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePTZMove('up-right')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-[45deg]">
                      <ArrowUp className="size-4 md:size-5 text-white" />
                    </div>
                  </button>
                </div>

                {/* Middle Row */}
                <div className="flex gap-[4px]">
                  <button
                    onMouseDown={() => handlePTZMove('left')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <ArrowLeft className="size-4 md:size-5 text-white" />
                  </button>
                  <button
                    onClick={() => handlePTZMove('home')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#dc2626] hover:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <Home className="size-4 md:size-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePTZMove('right')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <ArrowRight className="size-4 md:size-5 text-white" />
                  </button>
                </div>

                {/* Bottom Row */}
                <div className="flex gap-[4px]">
                  <button
                    onMouseDown={() => handlePTZMove('down-left')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-[-135deg]">
                      <ArrowUp className="size-4 md:size-5 text-white" />
                    </div>
                  </button>
                  <button
                    onMouseDown={() => handlePTZMove('down')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <ArrowDown className="size-4 md:size-5 text-white" />
                  </button>
                  <button
                    onMouseDown={() => handlePTZMove('down-right')}
                    className="size-[44px] md:size-[48px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#dc2626] active:bg-[#b91c1c] flex items-center justify-center transition-colors"
                  >
                    <div className="rotate-[135deg]">
                      <ArrowUp className="size-4 md:size-5 text-white" />
                    </div>
                  </button>
                </div>

                {/* Speed Control */}
                <div className="w-full mt-[16px]">
                  <div className="flex justify-between items-center mb-[8px]">
                    <label className="text-[#a1a1a6] text-[13px] md:text-[14px]">Speed</label>
                    <span className="text-[#ff8c00] text-[13px] md:text-[14px] font-mono">{ptzSpeed}%</span>
                  </div>
                  <input
                    type="range"
                    min="1"
                    max="100"
                    value={ptzSpeed}
                    onChange={(e) => setPtzSpeed(parseInt(e.target.value))}
                    className="w-full h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
                  />
                </div>
              </div>
            </div>

            {/* Preset Positions */}
            <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
              <div className="flex items-center gap-[12px] mb-[16px]">
                <Bookmark className="size-5 text-[#dc2626]" />
                <h3 className="text-white text-[15px] md:text-[16px]">Presets</h3>
              </div>
              
              <div className="space-y-[8px]">
                {presets.map((preset) => (
                  <div key={preset.id} className="flex items-center gap-[8px]">
                    <button
                      onClick={() => handleGoToPreset(preset.id)}
                      className={`flex-1 px-[10px] md:px-[12px] py-[6px] md:py-[8px] rounded-[8px] text-white text-[13px] md:text-[14px] flex items-center justify-between transition-colors ${
                        preset.active
                          ? 'bg-[#dc2626] hover:bg-[#b91c1c]'
                          : 'bg-[#3a3a3c] hover:bg-[#4a4a4c]'
                      }`}
                    >
                      <span>{preset.name}</span>
                      <span className="text-[11px] md:text-[12px] text-[#a1a1a6]">#{preset.id}</span>
                    </button>
                    <button
                      onClick={() => handleSavePreset(preset.id)}
                      className="size-[28px] md:size-[32px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#4a4a4c] flex items-center justify-center transition-colors"
                      title="Save current position"
                    >
                      <span className="text-white text-[14px] md:text-[16px]">+</span>
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Mobile PTZ & Actions Section */}
        <div className="lg:hidden space-y-[16px] mb-[16px] md:mb-[24px]">
          {/* PTZ Directional Controls - Mobile */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px]">
            <div className="flex items-center gap-[12px] mb-[16px]">
              <Camera className="size-5 text-[#dc2626]" />
              <h3 className="text-white text-[15px]">Pan & Tilt</h3>
            </div>
            
            <div className="flex flex-col items-center gap-[4px]">
              {/* Top Row */}
              <div className="flex gap-[4px]">
                <button
                  onMouseDown={() => handlePTZMove('up-left')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <div className="rotate-[-45deg]">
                    <ArrowUp className="size-5 text-white" />
                  </div>
                </button>
                <button
                  onMouseDown={() => handlePTZMove('up')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <ArrowUp className="size-5 text-white" />
                </button>
                <button
                  onMouseDown={() => handlePTZMove('up-right')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <div className="rotate-[45deg]">
                    <ArrowUp className="size-5 text-white" />
                  </div>
                </button>
              </div>

              {/* Middle Row */}
              <div className="flex gap-[4px]">
                <button
                  onMouseDown={() => handlePTZMove('left')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <ArrowLeft className="size-5 text-white" />
                </button>
                <button
                  onClick={() => handlePTZMove('home')}
                  className="size-[48px] rounded-[8px] bg-[#dc2626] hover:bg-[#b91c1c] flex items-center justify-center transition-colors"
                >
                  <Home className="size-5 text-white" />
                </button>
                <button
                  onMouseDown={() => handlePTZMove('right')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <ArrowRight className="size-5 text-white" />
                </button>
              </div>

              {/* Bottom Row */}
              <div className="flex gap-[4px]">
                <button
                  onMouseDown={() => handlePTZMove('down-left')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <div className="rotate-[-135deg]">
                    <ArrowUp className="size-5 text-white" />
                  </div>
                </button>
                <button
                  onMouseDown={() => handlePTZMove('down')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <ArrowDown className="size-5 text-white" />
                </button>
                <button
                  onMouseDown={() => handlePTZMove('down-right')}
                  className="size-[48px] rounded-[8px] bg-[#3a3a3c] active:bg-[#dc2626] flex items-center justify-center transition-colors"
                >
                  <div className="rotate-[135deg]">
                    <ArrowUp className="size-5 text-white" />
                  </div>
                </button>
              </div>

              {/* Speed Control */}
              <div className="w-full mt-[16px]">
                <div className="flex justify-between items-center mb-[8px]">
                  <label className="text-[#a1a1a6] text-[13px]">Speed</label>
                  <span className="text-[#ff8c00] text-[13px] font-mono">{ptzSpeed}%</span>
                </div>
                <input
                  type="range"
                  min="1"
                  max="100"
                  value={ptzSpeed}
                  onChange={(e) => setPtzSpeed(parseInt(e.target.value))}
                  className="w-full h-[6px] bg-[#3a3a3c] rounded-full appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-[18px] [&::-webkit-slider-thumb]:h-[18px] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-[#dc2626] [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:border-2 [&::-webkit-slider-thumb]:border-[#1c1c1e]"
                />
              </div>
            </div>
          </div>

          {/* Preset Positions - Mobile */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px]">
            <div className="flex items-center gap-[12px] mb-[16px]">
              <Bookmark className="size-5 text-[#dc2626]" />
              <h3 className="text-white text-[15px]">Presets</h3>
            </div>
            
            <div className="space-y-[8px]">
              {presets.map((preset) => (
                <div key={preset.id} className="flex items-center gap-[8px]">
                  <button
                    onClick={() => handleGoToPreset(preset.id)}
                    className={`flex-1 px-[12px] py-[8px] rounded-[8px] text-white text-[14px] flex items-center justify-between transition-colors ${
                      preset.active
                        ? 'bg-[#dc2626] hover:bg-[#b91c1c]'
                        : 'bg-[#3a3a3c] hover:bg-[#4a4a4c]'
                    }`}
                  >
                    <span>{preset.name}</span>
                    <span className="text-[12px] text-[#a1a1a6]">#{preset.id}</span>
                  </button>
                  <button
                    onClick={() => handleSavePreset(preset.id)}
                    className="size-[32px] rounded-[8px] bg-[#3a3a3c] hover:bg-[#4a4a4c] flex items-center justify-center transition-colors"
                    title="Save current position"
                  >
                    <span className="text-white text-[16px]">+</span>
                  </button>
                </div>
              ))}
            </div>
          </div>

          {/* Quick Actions - Mobile */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px]">
            <div className="flex flex-col gap-[10px]">
              <button className="w-full px-[20px] py-[10px] bg-[#dc2626] hover:bg-[#b91c1c] text-white rounded-[8px] transition-colors text-[14px]">
                Take Snapshot
              </button>
              <button className="w-full px-[20px] py-[10px] bg-[#3a3a3c] hover:bg-[#4a4a4c] text-white rounded-[8px] transition-colors text-[14px]">
                Start Recording
              </button>
              <button className="w-full px-[20px] py-[10px] bg-[#3a3a3c] hover:bg-[#4a4a4c] text-white rounded-[8px] transition-colors text-[14px]">
                Configure Stream
              </button>
            </div>
          </div>
        </div>

        {/* Bottom Row - Stream Information (Compact) */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-[16px] md:gap-[24px]">
          {/* Video Settings */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center gap-[12px] mb-[16px]">
              <Aperture className="size-5 text-[#dc2626]" />
              <h2 className="text-white text-[15px] md:text-[16px]">Stream Info</h2>
            </div>
            
            <div className="grid grid-cols-2 gap-x-[20px] md:gap-x-[24px] gap-y-[10px] md:gap-y-[12px]">
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Resolution</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">
                  {selectedStream === 'main' ? '1920x1080' : '640x480'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Frame Rate</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">
                  {selectedStream === 'main' ? '30 fps' : '15 fps'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Bitrate</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">
                  {selectedStream === 'main' ? '4096 Kbps' : '512 Kbps'}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Codec</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">H.264</span>
              </div>
            </div>
          </div>

          {/* Network Statistics */}
          <div className="bg-[#1c1c1e] border border-[#3a3a3c] rounded-[12px] p-[16px] md:p-[20px]">
            <div className="flex items-center gap-[12px] mb-[16px]">
              <Wifi className="size-5 text-[#dc2626]" />
              <h2 className="text-white text-[15px] md:text-[16px]">Network Stats</h2>
            </div>
            
            <div className="grid grid-cols-2 gap-x-[20px] md:gap-x-[24px] gap-y-[10px] md:gap-y-[12px]">
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Status</span>
                <div className="flex items-center gap-[6px]">
                  <div className="size-[6px] rounded-full bg-[#4ade80]"></div>
                  <span className="text-[#4ade80] text-[13px] md:text-[14px]">Connected</span>
                </div>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Latency</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">45 ms</span>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Packet Loss</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">0.0%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-[#a1a1a6] text-[13px] md:text-[14px]">Bandwidth</span>
                <span className="text-white text-[13px] md:text-[14px] font-mono">
                  {selectedStream === 'main' ? '4.2 Mbps' : '0.6 Mbps'}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}