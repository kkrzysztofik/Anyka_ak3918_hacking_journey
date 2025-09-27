import React, { useState, useEffect } from 'react';
import { 
  User, 
  Settings, 
  Users, 
  Camera, 
  Image, 
  Clock,
  Key,
  Eye,
  EyeOff,
  Save,
  LogOut,
  RefreshCw,
  Power,
  RotateCcw,
  Info,
  CheckCircle,
  XCircle,
  AlertCircle,
  Activity
} from '../utils/icons';
import { useAppDispatch, useAppSelector } from '../hooks/redux';
import { 
  getDeviceInformation, 
  getSystemCapabilities, 
  getDeviceStatus,
  setSystemDateAndTime,
  rebootDevice,
  factoryReset,
  updateDeviceConfiguration
} from '../store/slices/deviceSlice';
import { createCameraConfig } from '../config/cameraConfig';
import type { CameraConfig } from '../types';
import SystemUtilization from './SystemUtilization';

interface NavigationItem {
  id: string;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  active?: boolean;
}

interface ProfileData {
  username: string;
  logoutAfter: number;
  logoutUnit: 'hours' | 'minutes' | 'days';
  permissions: string[];
}

interface PasswordData {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

const DeviceService: React.FC = () => {
  const dispatch = useAppDispatch();
  const camera = useAppSelector((state) => state.camera);
  const device = useAppSelector((state) => state.device);
  
  const [activeSection, setActiveSection] = useState('device');
  const [showPassword, setShowPassword] = useState(false);
  const [showNewPassword, setShowNewPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  
  const [profileData, setProfileData] = useState<ProfileData>({
    username: 'admin',
    logoutAfter: 4,
    logoutUnit: 'hours',
    permissions: ['admin']
  });
  
  const [passwordData, setPasswordData] = useState<PasswordData>({
    currentPassword: '',
    newPassword: '',
    confirmPassword: ''
  });

  // Load device information on component mount
  useEffect(() => {
    const cameraConfig = createCameraConfig(camera.ip);

    dispatch(getDeviceInformation(cameraConfig));
    dispatch(getSystemCapabilities(cameraConfig));
    dispatch(getDeviceStatus(cameraConfig));
  }, [dispatch, camera.ip]);

  const navigationItems: NavigationItem[] = [
    {
      id: 'device',
      label: 'Device',
      description: 'Device information and management',
      icon: Info,
      active: true
    },
    {
      id: 'account',
      label: 'Account',
      description: 'Manage your data such as username and password',
      icon: User
    },
    {
      id: 'appearance',
      label: 'Appearance',
      description: 'Customize the look and feel of the interface',
      icon: Paintbrush
    },
    {
      id: 'interface',
      label: 'Interface',
      description: 'Configure the user interface settings',
      icon: Settings
    },
    {
      id: 'users',
      label: 'Users',
      description: 'Manage users and their permissions',
      icon: Users
    },
    {
      id: 'cameras',
      label: 'Cameras',
      description: 'Configure and manage cameras',
      icon: Camera
    },
    {
      id: 'recordings',
      label: 'Recordings',
      description: 'Manage recordings and storage',
      icon: Image
    },
    {
      id: 'utilization',
      label: 'System Utilization',
      description: 'Monitor CPU usage and temperature',
      icon: Activity
    }
  ];

  const handleProfileSave = async () => {
    try {
      const cameraConfig = createCameraConfig(camera.ip);

      await dispatch(updateDeviceConfiguration({ 
        config: cameraConfig, 
        newConfig: { 
          ip: profileData.username // This would be a real config update
        } 
      })).unwrap();
      
      console.log('Profile saved successfully');
    } catch (error) {
      console.error(`Error saving profile: ${error}`);
    }
  };

  const handlePasswordChange = async () => {
    if (passwordData.newPassword !== passwordData.confirmPassword) {
      console.error('Passwords do not match');
      return;
    }
    
    if (passwordData.newPassword.length < 6) {
      console.error('Password must be at least 6 characters long');
      return;
    }

    try {
      // TODO: Implement actual password change API call
      console.log('Password changed successfully');
      setPasswordData({
        currentPassword: '',
        newPassword: '',
        confirmPassword: ''
      });
    } catch (error) {
      console.error('Error changing password');
    }
  };

  const handleLogout = () => {
    // TODO: Implement logout functionality
    console.log('Logging out...');
  };

  const handleRefreshDeviceInfo = async () => {
    try {
      const cameraConfig = createCameraConfig(camera.ip);

      await Promise.all([
        dispatch(getDeviceInformation(cameraConfig)).unwrap(),
        dispatch(getSystemCapabilities(cameraConfig)).unwrap(),
        dispatch(getDeviceStatus(cameraConfig)).unwrap()
      ]);
      
      console.log('Device information refreshed');
    } catch (error) {
      console.error(`Error refreshing device info: ${error}`);
    }
  };

  const handleSetSystemTime = async () => {
    try {
      const cameraConfig = createCameraConfig(camera.ip);

      await dispatch(setSystemDateAndTime({ 
        config: cameraConfig, 
        dateTime: new Date(),
        timeZone: 'UTC'
      })).unwrap();
      
      console.log('System time updated');
    } catch (error) {
      console.error(`Error updating system time: ${error}`);
    }
  };

  const handleRebootDevice = async () => {
    if (!window.confirm('Are you sure you want to reboot the device? This will temporarily disconnect the camera.')) {
      return;
    }

    try {
      const cameraConfig = createCameraConfig(camera.ip);

      await dispatch(rebootDevice(cameraConfig)).unwrap();
      console.log('Device reboot initiated');
    } catch (error) {
      console.error(`Error rebooting device: ${error}`);
    }
  };

  const handleFactoryReset = async () => {
    if (!window.confirm('Are you sure you want to factory reset the device? This will restore all settings to default values and cannot be undone.')) {
      return;
    }

    try {
      const cameraConfig = createCameraConfig(camera.ip);

      await dispatch(factoryReset(cameraConfig)).unwrap();
      console.log('Factory reset initiated');
    } catch (error) {
      console.error(`Error performing factory reset: ${error}`);
    }
  };

  const renderDeviceInfoSection = () => (
    <div className="space-y-8">
      {/* Device Status */}
      <div className="bg-gray-800 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">Device Status</h3>
          <button
            onClick={handleRefreshDeviceInfo}
            disabled={device.isLoading}
            className="inline-flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white text-sm rounded-lg transition-colors"
          >
            <RefreshCw className={`h-4 w-4 mr-2 ${device.isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
        
        {device.deviceStatus && (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="text-center">
              <div className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${
                device.deviceStatus.online ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
              }`}>
                {device.deviceStatus.online ? <CheckCircle className="h-3 w-3 mr-1" /> : <XCircle className="h-3 w-3 mr-1" />}
                {device.deviceStatus.online ? 'Online' : 'Offline'}
              </div>
              <p className="text-xs text-gray-400 mt-1">Connection</p>
            </div>
            
            <div className="text-center">
              <div className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${
                device.deviceStatus.services.device ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
              }`}>
                {device.deviceStatus.services.device ? <CheckCircle className="h-3 w-3 mr-1" /> : <XCircle className="h-3 w-3 mr-1" />}
                Device Service
              </div>
              <p className="text-xs text-gray-400 mt-1">ONVIF</p>
            </div>
            
            <div className="text-center">
              <div className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${
                device.deviceStatus.services.media ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
              }`}>
                {device.deviceStatus.services.media ? <CheckCircle className="h-3 w-3 mr-1" /> : <XCircle className="h-3 w-3 mr-1" />}
                Media Service
              </div>
              <p className="text-xs text-gray-400 mt-1">ONVIF</p>
            </div>
            
            <div className="text-center">
              <div className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${
                device.deviceStatus.services.ptz ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'
              }`}>
                {device.deviceStatus.services.ptz ? <CheckCircle className="h-3 w-3 mr-1" /> : <XCircle className="h-3 w-3 mr-1" />}
                PTZ Service
              </div>
              <p className="text-xs text-gray-400 mt-1">ONVIF</p>
            </div>
          </div>
        )}
      </div>

      {/* Device Information */}
      {device.deviceInfo && (
        <div className="bg-gray-800 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-white mb-4">Device Information</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">Manufacturer</label>
              <p className="text-white">{device.deviceInfo.manufacturer}</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">Model</label>
              <p className="text-white">{device.deviceInfo.model}</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">Firmware Version</label>
              <p className="text-white">{device.deviceInfo.firmwareVersion}</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">Serial Number</label>
              <p className="text-white font-mono text-sm">{device.deviceInfo.serialNumber}</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">Hardware ID</label>
              <p className="text-white font-mono text-sm">{device.deviceInfo.hardwareId}</p>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">System Time</label>
              <p className="text-white">{device.deviceInfo.systemDateAndTime.utcDateTime.toLocaleString()}</p>
            </div>
          </div>
        </div>
      )}

      {/* System Capabilities */}
      {device.systemCapabilities && (
        <div className="bg-gray-800 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-white mb-4">System Capabilities</h3>
          <div className="space-y-4">
            <div>
              <h4 className="text-sm font-medium text-gray-300 mb-2">Network Features</h4>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                {Object.entries(device.systemCapabilities.network).map(([key, value]) => (
                  <div key={key} className="flex items-center">
                    {value ? <CheckCircle className="h-4 w-4 text-green-400 mr-2" /> : <XCircle className="h-4 w-4 text-red-400 mr-2" />}
                    <span className="text-sm text-gray-300">{key}</span>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h4 className="text-sm font-medium text-gray-300 mb-2">Security Features</h4>
              <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                {Object.entries(device.systemCapabilities.security).map(([key, value]) => (
                  <div key={key} className="flex items-center">
                    {value ? <CheckCircle className="h-4 w-4 text-green-400 mr-2" /> : <XCircle className="h-4 w-4 text-red-400 mr-2" />}
                    <span className="text-sm text-gray-300">{key}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Device Actions */}
      <div className="bg-gray-800 rounded-lg p-6">
        <h3 className="text-lg font-semibold text-white mb-4">Device Actions</h3>
        <div className="flex flex-wrap gap-4">
          <button
            onClick={handleSetSystemTime}
            disabled={device.isLoading}
            className="inline-flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white font-medium rounded-lg transition-colors"
          >
            <Clock className="h-4 w-4 mr-2" />
            Sync System Time
          </button>
          
          <button
            onClick={handleRebootDevice}
            disabled={device.isLoading}
            className="inline-flex items-center px-4 py-2 bg-yellow-600 hover:bg-yellow-700 disabled:bg-gray-600 text-white font-medium rounded-lg transition-colors"
          >
            <Power className="h-4 w-4 mr-2" />
            Reboot Device
          </button>
          
          <button
            onClick={handleFactoryReset}
            disabled={device.isLoading}
            className="inline-flex items-center px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 text-white font-medium rounded-lg transition-colors"
          >
            <RotateCcw className="h-4 w-4 mr-2" />
            Factory Reset
          </button>
        </div>
      </div>

      {/* Error Display */}
      {device.error && (
        <div className="bg-red-900 border border-red-700 rounded-lg p-4">
          <div className="flex items-center">
            <AlertCircle className="h-5 w-5 text-red-400 mr-2" />
            <span className="text-red-200">Error: {device.error}</span>
          </div>
        </div>
      )}
    </div>
  );

  const renderAccountSection = () => (
    <div className="space-y-8">
      {/* Profile Section */}
      <div className="space-y-6">
        <div>
          <h3 className="text-lg font-semibold text-white mb-2">Profile</h3>
          <p className="text-gray-400 text-sm">General Information</p>
        </div>
        
        <div className="space-y-4">
          {/* Username Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Username
            </label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <User className="h-5 w-5 text-gray-400" />
              </div>
              <input
                type="text"
                value={profileData.username}
                onChange={(e) => setProfileData({ ...profileData, username: e.target.value })}
                className="block w-full pl-10 pr-3 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
                placeholder="Enter username"
              />
            </div>
          </div>

          {/* Logout After Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Logout After
            </label>
            <div className="flex gap-2">
              <div className="relative flex-1">
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <Clock className="h-5 w-5 text-gray-400" />
                </div>
                <input
                  type="number"
                  value={profileData.logoutAfter}
                  onChange={(e) => setProfileData({ ...profileData, logoutAfter: parseInt(e.target.value) || 0 })}
                  className="block w-full pl-10 pr-3 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
                  placeholder="4"
                />
              </div>
              <select
                value={profileData.logoutUnit}
                onChange={(e) => setProfileData({ ...profileData, logoutUnit: e.target.value as 'hours' | 'minutes' | 'days' })}
                className="px-4 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
              >
                <option value="minutes">Minutes</option>
                <option value="hours">Hours</option>
                <option value="days">Days</option>
              </select>
            </div>
          </div>

          {/* Permissions Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Permissions:
            </label>
            <div className="flex flex-wrap gap-2">
              {profileData.permissions.map((permission, index) => (
                <span
                  key={index}
                  className="inline-flex items-center px-3 py-1 rounded-full text-xs font-medium bg-red-100 text-red-800"
                >
                  {permission}
                </span>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Password Section */}
      <div className="space-y-6">
        <div>
          <h3 className="text-lg font-semibold text-white mb-2">Password</h3>
          <p className="text-gray-400 text-sm">Change your password</p>
        </div>
        
        <div className="space-y-4">
          {/* Current Password Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Current Password
            </label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <Key className="h-5 w-5 text-gray-400" />
              </div>
              <input
                type={showPassword ? 'text' : 'password'}
                value={passwordData.currentPassword}
                onChange={(e) => setPasswordData({ ...passwordData, currentPassword: e.target.value })}
                className="block w-full pl-10 pr-10 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
                placeholder="Enter current password"
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute inset-y-0 right-0 pr-3 flex items-center"
              >
                {showPassword ? (
                  <EyeOff className="h-5 w-5 text-gray-400 hover:text-white" />
                ) : (
                  <Eye className="h-5 w-5 text-gray-400 hover:text-white" />
                )}
              </button>
            </div>
          </div>

          {/* New Password Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              New Password
            </label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <Key className="h-5 w-5 text-gray-400" />
              </div>
              <input
                type={showNewPassword ? 'text' : 'password'}
                value={passwordData.newPassword}
                onChange={(e) => setPasswordData({ ...passwordData, newPassword: e.target.value })}
                className="block w-full pl-10 pr-10 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
                placeholder="Enter new password"
              />
              <button
                type="button"
                onClick={() => setShowNewPassword(!showNewPassword)}
                className="absolute inset-y-0 right-0 pr-3 flex items-center"
              >
                {showNewPassword ? (
                  <EyeOff className="h-5 w-5 text-gray-400 hover:text-white" />
                ) : (
                  <Eye className="h-5 w-5 text-gray-400 hover:text-white" />
                )}
              </button>
            </div>
          </div>

          {/* Confirm Password Field */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              Confirm New Password
            </label>
            <div className="relative">
              <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <Key className="h-5 w-5 text-gray-400" />
              </div>
              <input
                type={showConfirmPassword ? 'text' : 'password'}
                value={passwordData.confirmPassword}
                onChange={(e) => setPasswordData({ ...passwordData, confirmPassword: e.target.value })}
                className="block w-full pl-10 pr-10 py-3 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-red-500 focus:border-transparent"
                placeholder="Confirm new password"
              />
              <button
                type="button"
                onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                className="absolute inset-y-0 right-0 pr-3 flex items-center"
              >
                {showConfirmPassword ? (
                  <EyeOff className="h-5 w-5 text-gray-400 hover:text-white" />
                ) : (
                  <Eye className="h-5 w-5 text-gray-400 hover:text-white" />
                )}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex gap-4 pt-6">
        <button
          onClick={handleProfileSave}
          className="inline-flex items-center px-6 py-3 bg-red-600 hover:bg-red-700 text-white font-medium rounded-lg transition-colors"
        >
          <Save className="h-5 w-5 mr-2" />
          Save
        </button>
        <button
          onClick={handlePasswordChange}
          className="inline-flex items-center px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
        >
          <Key className="h-5 w-5 mr-2" />
          Change Password
        </button>
        <button
          onClick={handleLogout}
          className="inline-flex items-center px-6 py-3 bg-gray-600 hover:bg-gray-700 text-white font-medium rounded-lg transition-colors"
        >
          <LogOut className="h-5 w-5 mr-2" />
          Logout
        </button>
      </div>
    </div>
  );

  const renderSectionContent = () => {
    switch (activeSection) {
      case 'device':
        return renderDeviceInfoSection();
      case 'account':
        return renderAccountSection();
      case 'appearance':
        return (
          <div className="text-center py-12">
            <Paintbrush className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Appearance</h3>
            <p className="text-gray-400">This section is under development</p>
          </div>
        );
      case 'interface':
        return (
          <div className="text-center py-12">
            <Settings className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Interface</h3>
            <p className="text-gray-400">This section is under development</p>
          </div>
        );
      case 'users':
        return (
          <div className="text-center py-12">
            <Users className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Users</h3>
            <p className="text-gray-400">This section is under development</p>
          </div>
        );
      case 'cameras':
        return (
          <div className="text-center py-12">
            <Camera className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Cameras</h3>
            <p className="text-gray-400">This section is under development</p>
          </div>
        );
      case 'recordings':
        return (
          <div className="text-center py-12">
            <Image className="h-12 w-12 text-gray-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-white mb-2">Recordings</h3>
            <p className="text-gray-400">This section is under development</p>
          </div>
        );
      case 'utilization':
        return <SystemUtilization />;
      default:
        return renderAccountSection();
    }
  };

  return (
    <div className="flex h-screen bg-gray-900 text-white">
      {/* Sidebar */}
      <div className="w-80 bg-gray-800 border-r border-gray-700 flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-gray-700">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-red-600 rounded-lg flex items-center justify-center">
              <span className="text-white font-bold text-sm">U</span>
            </div>
            <div>
              <h1 className="text-lg font-semibold">camera.ui</h1>
              <p className="text-xs text-gray-400">Device Service</p>
            </div>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 p-4 space-y-2">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                onClick={() => setActiveSection(item.id)}
                className={`w-full text-left p-4 rounded-lg transition-colors ${
                  activeSection === item.id
                    ? 'bg-gray-700 text-red-400'
                    : 'text-gray-300 hover:bg-gray-700 hover:text-white'
                }`}
              >
                <div className="flex items-start gap-3">
                  <Icon className="h-5 w-5 mt-0.5 flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">{item.label}</div>
                    <div className="text-xs text-gray-400 mt-1 line-clamp-2">
                      {item.description}
                    </div>
                  </div>
                </div>
              </button>
            );
          })}
        </nav>

        {/* Footer */}
        <div className="p-4 border-t border-gray-700">
          <div className="text-xs text-gray-400">v1.0.0</div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <div className="bg-gray-800 border-b border-gray-700 px-8 py-6">
          <h2 className="text-2xl font-bold text-white">
            {navigationItems.find(item => item.id === activeSection)?.label || 'Device'}
          </h2>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-8">
          <div className="max-w-4xl">
            {renderSectionContent()}
          </div>
        </div>
      </div>
    </div>
  );
};

export default DeviceService;
