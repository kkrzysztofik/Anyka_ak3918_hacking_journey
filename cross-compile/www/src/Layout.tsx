/**
 * Main Layout Component
 *
 * Responsive layout with integrated sidebar navigation, header, and mobile sheet nav.
 * Matches the premium dark theme design from .ai/design.
 */

import React, { useState } from 'react'
import { Outlet, NavLink, useLocation } from 'react-router-dom'
import {
  Camera,
  Activity,
  Menu,
  User,
  Network,
  Clock,
  Image,
  Users,
  Cctv,
  KeyRound,
  ChevronUp,
  Info,
  LogOut,
  Layers,
  Wrench,
  Settings
} from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { Button } from '@/components/ui/button'
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet'
import { ChangePasswordDialog } from '@/components/ChangePasswordDialog'
import { AboutDialog } from '@/components/AboutDialog'
import { ConnectionStatusBadge } from '@/components/common/ConnectionStatus'
import { cn } from '@/lib/utils'


interface NavItem {
  path: string
  label: string
  description?: string
  icon: React.ElementType
  children?: NavItem[]
}

const navItems: NavItem[] = [
  { path: '/live', label: 'Live View', description: 'Real-time camera stream', icon: Camera },
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
    ]
  },
  { path: '/diagnostics', label: 'Diagnostics', description: 'System telemetry & logs', icon: Activity },
]

function NavLinkItem({ item, onClick, isMobile = false }: { item: NavItem; onClick?: () => void; isMobile?: boolean }) {
  const [isOpen, setIsOpen] = useState(false)
  const location = useLocation()
  const isActive = item.path === '/settings'
    ? location.pathname.startsWith('/settings')
    : location.pathname === item.path

  // Auto-expand if child is active
  React.useEffect(() => {
    if (item.children && location.pathname.startsWith(item.path)) {
      setIsOpen(true)
    }
  }, [location.pathname, item.path, item.children])

  if (item.children) {
    return (
      <div className="flex flex-col gap-1">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={cn(
            'w-full relative flex items-center gap-4 px-4 py-3 rounded-lg transition-all group duration-200 text-left',
            isActive
              ? 'bg-accent-red/10 border-l-[3px] border-accent-red rounded-l-none'
              : 'hover:bg-white/5',
            isMobile ? 'border-l-0 rounded-lg h-auto' : 'h-[72px]'
          )}
        >
          <div className={cn(
            'p-2 rounded-lg transition-colors',
            isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white'
          )}>
            <item.icon className="w-5 h-5" />
          </div>
          <div className="flex flex-col">
            <span className={cn(
              'text-[15px] font-medium transition-colors',
              isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white'
            )}>
              {item.label}
            </span>
            {item.description && (
              <span className="text-[12px] text-dark-secondary-text/60 line-clamp-1 text-left">
                {item.description}
              </span>
            )}
          </div>
          <ChevronUp className={cn(
            "w-4 h-4 text-dark-secondary-text transition-transform duration-200",
            isOpen ? "" : "rotate-180"
          )} />
        </button>

        {isOpen && (
          <div className="flex flex-col gap-1 pl-4">
            {item.children.map(child => (
              <NavLink
                key={child.path}
                to={child.path}
                onClick={onClick}
                className={({ isActive }) =>
                  cn(
                    'flex items-center gap-3 px-4 py-2 rounded-lg transition-colors text-sm',
                    isActive
                      ? 'text-white bg-white/10'
                      : 'text-dark-secondary-text hover:text-white hover:bg-white/5'
                  )
                }
              >
                <child.icon className="w-4 h-4" />
                <span>{child.label}</span>
              </NavLink>
            ))}
          </div>
        )}
      </div>
    )
  }

  return (
    <NavLink
      to={item.path}
      onClick={onClick}
      className={({ isActive }) =>
        cn(
          'relative flex items-center gap-4 px-4 py-3 rounded-lg transition-all group duration-200',
          isActive
            ? 'bg-accent-red/10 border-l-[3px] border-accent-red rounded-l-none'
            : 'hover:bg-white/5',
          isMobile ? 'border-l-0 rounded-lg h-auto' : 'h-[72px]'
        )
      }
    >
      {({ isActive }) => (
        <>
          <div className={cn(
            'p-2 rounded-lg transition-colors',
            isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white'
          )}>
            <item.icon className="w-5 h-5" />
          </div>
          <div className="flex flex-col">
            <span className={cn(
              'text-[15px] font-medium transition-colors',
              isActive ? 'text-white' : 'text-dark-secondary-text group-hover:text-white'
            )}>
              {item.label}
            </span>
            {item.description && (
              <span className="text-[12px] text-dark-secondary-text/60 line-clamp-1">
                {item.description}
              </span>
            )}
          </div>
        </>
      )}
    </NavLink>
  )
}

function SidebarContent({ onClose }: { onClose?: () => void }) {
  const { username, logout } = useAuth()
  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const [isChangePasswordOpen, setIsChangePasswordOpen] = useState(false)
  const [isAboutOpen, setIsAboutOpen] = useState(false)

  return (
    <>
      <div className="flex flex-col h-full bg-dark-sidebar border-r border-dark-border">
        {/* Branding */}
        <div className="p-6 mb-2">
          <div className="flex items-center gap-3">
            <div className="size-10 bg-accent-red rounded-xl flex items-center justify-center shadow-lg shadow-accent-red/20">
              <Cctv className="w-6 h-6 text-white" />
            </div>
            <div className="flex flex-col">
              <span className="text-white font-medium text-lg leading-tight">ONVIF</span>
              <span className="text-dark-secondary-text text-[11px] uppercase tracking-wider font-medium">Device Manager</span>
            </div>
          </div>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto px-2 py-2 space-y-1">
          {navItems.map((item) => (
            <NavLinkItem key={item.path} item={item} onClick={onClose} />
          ))}
        </nav>

        {/* Footer / User section */}
        <div className="p-4 border-t border-dark-border bg-dark-sidebar/50 relative">
          {/* User Menu Popup */}
          {isMenuOpen && (
            <>
              <div
                className="fixed inset-0 z-40"
                onClick={() => setIsMenuOpen(false)}
              />
              <div className="absolute bottom-full left-4 right-4 mb-2 bg-dark-sidebar border border-dark-border rounded-lg shadow-xl py-1 z-50 animate-in fade-in zoom-in duration-200">
                <div className="px-4 py-3 border-b border-dark-border">
                  <p className="text-sm font-semibold text-white">Admin User</p>
                  <p className="text-xs text-dark-secondary-text truncate">{username}@device.local</p>
                </div>

                <div className="py-1">
                  <button
                    className="w-full flex items-center px-4 py-2 text-sm text-dark-secondary-text hover:bg-dark-hover hover:text-white transition-colors"
                    onClick={() => {
                      setIsMenuOpen(false)
                      setIsChangePasswordOpen(true)
                    }}
                  >
                    <KeyRound className="w-4 h-4 mr-3" />
                    Change Password
                  </button>

                  <button
                    key="about"
                    onClick={() => {
                      setIsMenuOpen(false)
                      setIsAboutOpen(true)
                    }}
                    className="w-full flex items-center px-4 py-2 text-sm text-dark-secondary-text hover:bg-dark-hover hover:text-white transition-colors"
                  >
                    <Info className="w-4 h-4 mr-3" />
                    About
                  </button>
                </div>

                <div className="border-t border-dark-border py-1">
                  <button
                    className="w-full flex items-center px-4 py-2 text-sm text-accent-red hover:bg-dark-hover transition-colors"
                    onClick={() => {
                      setIsMenuOpen(false)
                      logout()
                    }}
                  >
                    <LogOut className="w-4 h-4 mr-3" />
                    Sign Out
                  </button>
                </div>
              </div>
            </>
          )}

          <button
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            className="flex items-center justify-between gap-3 px-2 py-2 rounded-lg bg-dark-card border border-dark-border w-full hover:bg-dark-hover transition-colors group"
          >
            <div className="flex items-center gap-3">
              <div className="size-8 rounded-full bg-accent-red/10 flex items-center justify-center border border-accent-red/20">
                <User className="w-4 h-4 text-accent-red" />
              </div>
              <div className="flex flex-col items-start transition-opacity">
                <span className="text-[13px] font-medium text-white truncate max-w-[100px]">{username}</span>
                <span className="text-[10px] text-dark-secondary-text">Administrator</span>
              </div>
            </div>
            <ChevronUp className={cn(
              "w-4 h-4 text-dark-secondary-text transition-transform duration-200",
              isMenuOpen ? "rotate-180" : ""
            )} />
          </button>

          <div className="mt-4 text-center">
            <p className="text-[10px] text-dark-secondary-text/40 italic">
              v1.0.0-beta • ONVIF 24.12
            </p>
          </div>
        </div>
      </div>

      <ChangePasswordDialog
        open={isChangePasswordOpen}
        onOpenChange={setIsChangePasswordOpen}
      />

      <AboutDialog
        open={isAboutOpen}
        onOpenChange={setIsAboutOpen}
      />
    </>
  )
}


function Header() {
  const [isMobileNavOpen, setIsMobileNavOpen] = useState(false)
  const location = useLocation()

  // Find the current page title based on the route
  const currentItem = navItems.find(item =>
    item.path === '/'
      ? location.pathname === '/'
      : location.pathname.startsWith(item.path)
  )
  const pageTitle = currentItem?.label || 'Dashboard'

  return (
    <header className="flex lg:hidden items-center justify-between h-16 px-6 bg-dark-bg/80 backdrop-blur-md border-b border-dark-border sticky top-0 z-30">
      <div className="flex items-center gap-4">
        <Sheet open={isMobileNavOpen} onOpenChange={setIsMobileNavOpen}>
          <SheetTrigger asChild>
            <Button variant="ghost" size="icon" className="lg:hidden text-white">
              <Menu className="w-6 h-6" />
            </Button>
          </SheetTrigger>
          <SheetContent side="left" className="p-0 border-r-0 w-[300px]">
            <SidebarContent onClose={() => setIsMobileNavOpen(false)} />
          </SheetContent>
        </Sheet>
        <div className="flex items-center gap-4">
          <h2 className="text-sm font-medium text-dark-secondary-text uppercase tracking-[0.2em]">
            {pageTitle}
          </h2>
        </div>
      </div>

      <div className="flex items-center gap-6">
        <ConnectionStatusBadge status="connected" />
        {/* Mobile User Profile could go here if needed, or we rely on the one in the sheet sidebar */}
      </div>
    </header>
  )
}

export default function Layout() {
  return (
    <div className="flex h-screen bg-dark-bg font-sans selection:bg-accent-red/30">
      <aside className="hidden lg:block w-[320px] shrink-0 h-full overflow-hidden">
        <SidebarContent />
      </aside>
      <div className="flex-1 flex flex-col min-w-0">
        <Header />
        <main className="flex-1 overflow-y-auto p-6 md:p-8">
          <div className="w-full h-full animate-in fade-in slide-in-from-bottom-4 duration-500">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  )
}
