import React from 'react';
import type { HeaderProps } from '../types';

const Header: React.FC<HeaderProps> = ({ cameraName, cameraType, currentTime }) => {
  const formatTime = (date: Date): string => {
    return date.toISOString().slice(0, 19).replace('T', ' ');
  };

  return (
    <div className="bg-dark-card border-b border-gray-700 px-6 py-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{cameraName}</h1>
          <p className="text-gray-400">{cameraType}</p>
        </div>
        <div className="text-sm text-gray-400">
          {formatTime(currentTime)}
        </div>
      </div>
    </div>
  );
};

export default Header;
