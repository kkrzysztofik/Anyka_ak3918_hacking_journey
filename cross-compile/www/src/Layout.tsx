/**
 * Main Layout Component
 *
 * Responsive layout with integrated sidebar navigation, header, and mobile sheet nav.
 * Matches the premium dark theme design from .ai/design.
 */
import React, { useState } from 'react';

import {
  Activity,
  Camera,
  Cctv,
  ChevronUp,
  Clock,
  Image,
  Info,
  KeyRound,
  Layers,
  LogOut,
  Menu,
  Network,
  Settings,
  User,
  Users,
  Wrench,
} from 'lucide-react';
import { NavLink, Outlet, useLocation } from 'react-router-dom';

import { AboutDialog } from '@/components/AboutDialog';
import { ChangePasswordDialog } from '@/components/ChangePasswordDialog';
import { ConnectionStatusBadge } from '@/components/common/ConnectionStatus';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import { useAuth } from '@/hooks/useAuth';
import { cn } from '@/lib/utils';

interface NavItem {
  path: string;
  label: string;
  description?: string;
  icon: React.ElementType;
  children?: NavItem[];
}

const navItems: NavItem[] = [
  {
    path: '/live',
    label: 'Live View',
    description: 'Real-time camera stream',
    icon: Camera,
  },
  {
    path: '/settings',
    label: 'Settings',
    description: 'System configuration',
    icon: Wrench,
    children: [
      { path: '/settings/identification', label: 'Identification', icon: User },
      { path: '/settings/network', label: 'Network', icon: Network },
      { path: '/settings/time', label: 'Time', icon: Clock },
      { path: '/settings/imaging', label: 'Imaging', icon: Image },
      { path: '/settings/profiles', label: 'Profiles', icon: Layers },
      { path: '/settings/users', label: 'Users', icon: Users },
      { path: '/settings/maintenance', label: 'Maintenance', icon: Settings },
    ],
  },
  {
    path: '/diagnostics',
    label: 'Diagnostics',
    description: 'System telemetry & logs',
    icon: Activity,
  },
];

function NavLinkItem({
  item,
  onClick,
  isMobile = false,
}: Readonly<{
  item: NavItem;
  onClick?: () => void;
  isMobile?: boolean;
}>) {
  const [isOpen, setIsOpen] = useState(false);
  const location = useLocation();
  const isActive =
    item.path === '/settings'
      ? location.pathname.startsWith('/settings')
      : location.pathname === item.path;

  // Auto-expand if child is active
  React.useEffect(() => {
    if (item.children && location.pathname.startsWith(item.path)) {
      setIsOpen(true);
    }
  }, [location.pathname, item.path, item.children]);

  if (item.children) {
    return (
      <div className="flex flex-col gap-1">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={cn(
            'group relative flex w-full items-center gap-4 rounded-lg px-4 py-3 text-left transition-all duration-200',
            isActive
              ? 'bg-accent-red/10 border-accent-red rounded-l-none border-l-[3px]'
              : 'hover:bg-white/5',
            isMobile ? 'h-auto rounded-lg border-l-0' : 'h-[72px]',
          )}
        >
          <div
            className={cn(
              'rounded-lg p-2 transition-colors',
              isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white',
            )}
          >
            <item.icon className="h-5 w-5" />
          </div>
          <div className="flex flex-col">
            <span
              className={cn(
                'text-[15px] font-medium transition-colors',
                isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white',
              )}
            >
              {item.label}
            </span>
            {item.description && (
              <span className="text-dark-secondary-text/60 line-clamp-1 text-left text-[12px]">
                {item.description}
              </span>
            )}
          </div>
          <ChevronUp
            className={cn(
              'text-muted-foreground ml-auto h-4 w-4 transition-transform duration-200',
              isOpen ? '' : 'rotate-180',
            )}
          />
        </button>

        {isOpen && (
          <div className="flex flex-col gap-1 pl-4">
            {item.children.map((child) => (
              <NavLink
                key={child.path}
                to={child.path}
                onClick={onClick}
                className={({ isActive }) =>
                  cn(
                    'flex items-center gap-3 rounded-lg px-4 py-2 text-sm transition-colors',
                    isActive
                      ? 'bg-white/10 text-white'
                      : 'text-dark-secondary-text hover:bg-white/5 hover:text-white',
                  )
                }
              >
                <child.icon className="h-4 w-4" />
                <span>{child.label}</span>
              </NavLink>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <NavLink
      to={item.path}
      onClick={onClick}
      className={({ isActive }) =>
        cn(
          'group relative flex items-center gap-4 rounded-lg px-4 py-3 transition-all duration-200',
          isActive
            ? 'bg-accent-red/10 border-accent-red rounded-l-none border-l-[3px]'
            : 'hover:bg-white/5',
          isMobile ? 'h-auto rounded-lg border-l-0' : 'h-[72px]',
        )
      }
    >
      {({ isActive }) => (
        <>
          <div
            className={cn(
              'rounded-lg p-2 transition-colors',
              isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white',
            )}
          >
            <item.icon className="h-5 w-5" />
          </div>
          <div className="flex flex-col">
            <span
              className={cn(
                'text-[15px] font-medium transition-colors',
                isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white',
              )}
            >
              {item.label}
            </span>
            {item.description && (
              <span className="text-dark-secondary-text/60 line-clamp-1 text-[12px]">
                {item.description}
              </span>
            )}
          </div>
        </>
      )}
    </NavLink>
  );
}

function SidebarContent({ onClose }: Readonly<{ onClose?: () => void }>) {
  const { username, logout } = useAuth();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [isChangePasswordOpen, setIsChangePasswordOpen] = useState(false);
  const [isAboutOpen, setIsAboutOpen] = useState(false);

  return (
    <>
      <div className="bg-dark-sidebar border-dark-border flex h-full flex-col border-r">
        {/* Branding */}
        <div className="mb-2 p-6">
          <div className="flex items-center gap-3">
            <div className="bg-accent-red shadow-accent-red/20 flex size-10 items-center justify-center rounded-xl shadow-lg">
              <Cctv className="h-6 w-6 text-white" />
            </div>
            <div className="flex flex-col">
              <span className="text-lg leading-tight font-medium text-white">ONVIF</span>
              <span className="text-dark-secondary-text text-[11px] font-medium tracking-wider uppercase">
                Device Manager
              </span>
            </div>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 space-y-1 overflow-y-auto px-2 py-2">
          {navItems.map((item) => (
            <NavLinkItem key={item.path} item={item} onClick={onClose} />
          ))}
        </nav>

        {/* Footer / User section */}
        <div className="border-dark-border bg-dark-sidebar/50 relative border-t p-4">
          {/* User Menu Popup */}
          {isMenuOpen && (
            <>
              <button
                type="button"
                className="fixed inset-0 z-40"
                onClick={() => setIsMenuOpen(false)}
                onKeyDown={(e) => {
                  if (e.key === 'Escape') {
                    setIsMenuOpen(false);
                  }
                }}
                aria-label="Close menu"
              />
              <div className="bg-dark-sidebar border-dark-border animate-in fade-in zoom-in absolute right-4 bottom-full left-4 z-50 mb-2 rounded-lg border py-1 shadow-xl duration-200">
                <div className="border-dark-border border-b px-4 py-3">
                  <p className="text-sm font-semibold text-white">Admin User</p>
                  <p className="text-dark-secondary-text truncate text-xs">
                    {username}@device.local
                  </p>
                </div>

                <div className="py-1">
                  <button
                    className="text-dark-secondary-text hover:bg-dark-hover flex w-full items-center px-4 py-2 text-sm transition-colors hover:text-white"
                    onClick={() => {
                      setIsMenuOpen(false);
                      setIsChangePasswordOpen(true);
                    }}
                  >
                    <KeyRound className="mr-3 h-4 w-4" />
                    Change Password
                  </button>

                  <button
                    key="about"
                    onClick={() => {
                      setIsMenuOpen(false);
                      setIsAboutOpen(true);
                    }}
                    className="text-dark-secondary-text hover:bg-dark-hover flex w-full items-center px-4 py-2 text-sm transition-colors hover:text-white"
                  >
                    <Info className="mr-3 h-4 w-4" />
                    About
                  </button>
                </div>

                <div className="border-dark-border border-t py-1">
                  <button
                    className="text-accent-red hover:bg-dark-hover flex w-full items-center px-4 py-2 text-sm transition-colors"
                    onClick={() => {
                      setIsMenuOpen(false);
                      logout();
                    }}
                  >
                    <LogOut className="mr-3 h-4 w-4" />
                    Sign Out
                  </button>
                </div>
              </div>
            </>
          )}

          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className="bg-dark-card border-dark-border hover:bg-dark-hover group flex w-full items-center justify-between gap-3 rounded-lg border px-2 py-2 transition-colors"
          >
            <div className="flex items-center gap-3">
              <div className="bg-accent-red/10 border-accent-red/20 flex size-8 items-center justify-center rounded-full border">
                <User className="text-accent-red h-4 w-4" />
              </div>
              <div className="flex flex-col items-start transition-opacity">
                <span className="max-w-[100px] truncate text-[13px] font-medium text-white">
                  {username}
                </span>
                <span className="text-dark-secondary-text text-[10px]">Administrator</span>
              </div>
            </div>
            <ChevronUp
              className={cn(
                'text-dark-secondary-text h-4 w-4 transition-transform duration-200',
                isMenuOpen ? 'rotate-180' : '',
              )}
            />
          </button>

          <div className="mt-4 text-center">
            <p className="text-dark-secondary-text/40 text-[10px] italic">
              v1.0.0-beta • ONVIF 24.12
            </p>
          </div>
        </div>
      </div>

      <ChangePasswordDialog open={isChangePasswordOpen} onOpenChange={setIsChangePasswordOpen} />

      <AboutDialog open={isAboutOpen} onOpenChange={setIsAboutOpen} />
    </>
  );
}

function Header() {
  const [isMobileNavOpen, setIsMobileNavOpen] = useState(false);
  const location = useLocation();

  // Find the current page title based on the route
  const currentItem = navItems.find((item) =>
    item.path === '/' ? location.pathname === '/' : location.pathname.startsWith(item.path),
  );
  const pageTitle = currentItem?.label || 'Dashboard';

  return (
    <header className="bg-dark-bg/80 border-dark-border sticky top-0 z-30 flex h-16 items-center justify-between border-b px-6 backdrop-blur-md lg:hidden">
      <div className="flex items-center gap-4">
        <Sheet open={isMobileNavOpen} onOpenChange={setIsMobileNavOpen}>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" className="text-white lg:hidden">
              <Menu className="h-6 w-6" />
            </Button>
          </SheetTrigger>
          <SheetContent side="left" className="w-[300px] border-r-0 p-0">
            <SidebarContent onClose={() => setIsMobileNavOpen(false)} />
          </SheetContent>
        </Sheet>
        <div className="flex items-center gap-4">
          <h2 className="text-dark-secondary-text text-sm font-medium tracking-[0.2em] uppercase">
            {pageTitle}
          </h2>
        </div>
      </div>

      <div className="flex items-center gap-6">
        <ConnectionStatusBadge status="connected" />
        {/* Mobile User Profile could go here if needed, or we rely on the one in the sheet sidebar */}
      </div>
    </header>
  );
}

export default function Layout() {
  return (
    <div className="bg-dark-bg selection:bg-accent-red/30 flex h-screen font-sans">
      <aside className="hidden h-full w-[260px] shrink-0 overflow-hidden lg:block">
        <SidebarContent />
      </aside>
      <div className="flex min-w-0 flex-1 flex-col">
        <Header />
        <main className="flex-1 overflow-y-auto p-6 md:p-8">
          <div className="page-enter h-full w-full">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
