/**
 * @file Settings Page
 * @description Application settings and configuration
 */

import React from 'react';
import { useAppSelector } from '../hooks/redux';
import Header from '../components/Header';
import Sidebar from '../components/Sidebar';
import { Settings as SettingsIcon, Camera, Wifi, Monitor } from '../utils/icons';

const SettingsPage: React.FC = () => {
  const camera = useAppSelector((state) => state.camera);
  const ui = useAppSelector((state) => state.ui);

  const settingsSections = [
    {
      title: 'Camera Settings',
      icon: Camera,
      items: [
        { label: 'Camera Name', value: camera.name || 'Unknown' },
        { label: 'Camera Type', value: camera.type || 'Unknown' },
        { label: 'IP Address', value: camera.ip || 'Not configured' },
      ]
    },
    {
      title: 'Network Settings',
      icon: Wifi,
      items: [
        { label: 'Connection Status', value: 'Connected' },
        { label: 'Signal Strength', value: 'Strong' },
        { label: 'Network Type', value: 'WiFi' },
      ]
    },
    {
      title: 'Display Settings',
      icon: Monitor,
      items: [
        { label: 'Theme', value: 'Dark' },
        { label: 'Language', value: 'English' },
        { label: 'Time Format', value: '24-hour' },
      ]
    }
  ];

  return (
    <div className="flex h-screen bg-dark-bg text-white">
      <Sidebar />
      
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header 
          cameraName={camera.name} 
          cameraType={camera.type}
          currentTime={ui.currentTime}
        />
        
        <div className="flex-1 p-6 overflow-y-auto">
          <div className="max-w-4xl mx-auto">
            <div className="mb-8">
              <h1 className="text-3xl font-bold mb-2">Settings</h1>
              <p className="text-gray-400">Configure your camera and application preferences</p>
            </div>

            <div className="space-y-6">
              {settingsSections.map((section, index) => (
                <div key={index} className="bg-gray-800 rounded-lg p-6">
                  <div className="flex items-center mb-4">
                    <section.icon className="h-6 w-6 text-blue-400 mr-3" />
                    <h2 className="text-xl font-semibold">{section.title}</h2>
                  </div>
                  
                  <div className="space-y-3">
                    {section.items.map((item, itemIndex) => (
                      <div key={itemIndex} className="flex justify-between items-center py-2 border-b border-gray-700 last:border-b-0">
                        <span className="text-gray-300">{item.label}</span>
                        <span className="text-white font-medium">{item.value}</span>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SettingsPage;
