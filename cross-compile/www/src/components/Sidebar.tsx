import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { Camera, HardDrive, Monitor, Settings } from 'lucide-react';
import type { SidebarProps } from '../types';

const Sidebar: React.FC<SidebarProps> = () => {
  const location = useLocation();

  const navItems = [
    { icon: Camera, label: 'Camera', href: '/camera' },
    { icon: HardDrive, label: 'Device', href: '/device' },
    { icon: Monitor, label: 'System', href: '/system' },
    { icon: Settings, label: 'Settings', href: '/settings' },
  ];

  return (
    <div className="w-16 bg-dark-sidebar flex flex-col items-center py-4 space-y-6">
      {/* Logo */}
      <div className="flex items-center justify-center w-10 h-10 bg-accent-red rounded-lg">
        <span className="text-white font-bold text-lg">U</span>
      </div>

      {/* Navigation Icons */}
      <nav className="flex flex-col space-y-4">
        {navItems.map((item, index) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={index}
              to={item.href}
              className={`flex items - center justify - center w - 12 h - 12 rounded - lg transition - colors ${isActive
                ? 'bg-accent-red/20 border-l-2 border-accent-red'
                : 'hover:bg-gray-600'
                } `}
              title={item.label}
            >
              <Icon className="sidebar-icon" />
            </Link>
          );
        })}
      </nav>

      {/* Version */}
      <div className="mt-auto text-xs text-gray-400">
        v1.0.0
      </div>
    </div>
  );
};

export default Sidebar;
