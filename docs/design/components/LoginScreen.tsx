import { useState } from 'react';
import { Cctv, Eye, EyeOff } from 'lucide-react';

export default function LoginScreen({ onLogin }: { onLogin: () => void }) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // Simple validation - in production this would connect to actual auth
    if (username && password) {
      if (username === 'admin' && password === 'admin') {
        onLogin();
      } else {
        setError('Invalid username or password');
      }
    } else {
      setError('Please enter both username and password');
    }
  };

  return (
    <div className="size-full bg-[#0d0d0d] flex items-center justify-center p-[16px]">
      <div className="w-full max-w-[440px] bg-[#1c1c1e] border border-[#3a3a3c] rounded-[16px] p-[24px] sm:p-[32px] md:p-[48px]">
        {/* Logo/Brand */}
        <div className="flex flex-col items-center mb-[32px] md:mb-[40px]">
          <div className="size-[56px] md:size-[64px] bg-[#ff3b30] rounded-[16px] flex items-center justify-center mb-[12px] md:mb-[16px]">
            <Cctv className="size-[28px] md:size-[32px]" color="white" />
          </div>
          <h1 className="text-white text-[20px] md:text-[24px] mb-[6px] md:mb-[8px] text-center">ONVIF Device Manager</h1>
          <p className="text-[#a1a1a6] text-[13px] md:text-[14px] text-center">API Management, Live Preview & Telemetry</p>
        </div>

        {/* Login Form */}
        <form onSubmit={handleSubmit}>
          {/* Username/Email Field */}
          <div className="mb-[20px] md:mb-[24px]">
            <label className="block text-[#a1a1a6] text-[13px] md:text-[14px] mb-[8px]">
              Username
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => {
                setUsername(e.target.value);
                setError('');
              }}
              placeholder="Enter username"
              className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] text-white text-[15px] md:text-[16px] placeholder:text-[#3a3a3c] focus:outline-none focus:border-[#ff3b30] transition-colors"
            />
          </div>

          {/* Password Field */}
          <div className="mb-[24px] md:mb-[32px]">
            <label className="block text-[#a1a1a6] text-[13px] md:text-[14px] mb-[8px]">
              Password
            </label>
            <div className="relative">
              <input
                type={showPassword ? 'text' : 'password'}
                value={password}
                onChange={(e) => {
                  setPassword(e.target.value);
                  setError('');
                }}
                placeholder="Enter password"
                className="w-full h-[44px] bg-transparent border border-[#3a3a3c] rounded-[8px] px-[16px] pr-[48px] text-white text-[15px] md:text-[16px] placeholder:text-[#3a3a3c] focus:outline-none focus:border-[#ff3b30] transition-colors"
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-[12px] top-1/2 translate-y-[-50%] text-[#a1a1a6] hover:text-white transition-colors"
              >
                {showPassword ? (
                  <EyeOff className="size-5" />
                ) : (
                  <Eye className="size-5" />
                )}
              </button>
            </div>
          </div>

          {/* Error Message */}
          {error && (
            <div className="mb-[20px] md:mb-[24px] p-[12px] bg-[rgba(255,59,48,0.1)] border border-[#ff3b30] rounded-[8px]">
              <p className="text-[#ff3b30] text-[13px] md:text-[14px]">{error}</p>
            </div>
          )}

          {/* Login Button */}
          <button
            type="submit"
            className="w-full h-[44px] bg-[#ff3b30] rounded-[8px] text-white font-semibold text-[15px] md:text-[16px] hover:bg-[#ff4d42] transition-colors"
          >
            Sign In
          </button>
        </form>

        {/* Helper Text */}
        <div className="mt-[20px] md:mt-[24px] text-center">
          <p className="text-[#a1a1a6] text-[11px] md:text-[12px]">
            © 2025 ONVIF Device Manager. All rights reserved.
          </p>
        </div>
      </div>
    </div>
  );
}