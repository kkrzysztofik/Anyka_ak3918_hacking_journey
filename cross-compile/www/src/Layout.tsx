/**
 * Main Layout Component
 *
 * Responsive layout with sidebar navigation, header, and mobile sheet nav.
 */

import React, { useState, useCallback } from 'react'
import { Outlet, NavLink, useLocation } from 'react-router-dom'
import {
  Camera,
  Settings,
  Activity,
  Menu,
  X,
  LogOut,
  User,
  Network,
  Clock,
  Image,
  Users,
  Wrench,
  Layers,
  Home
} from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { Button } from '@/components/ui/button'
import { Sheet, SheetContent, SheetTrigger, SheetClose, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { ConnectionStatusBadge } from '@/components/common/ConnectionStatus'
import { cn } from '@/lib/utils'

interface NavItem {
  path: string
  label: string
  icon: React.ReactNode
}

const mainNavItems: NavItem[] = [
  { path: '/', label: 'Dashboard', icon: <Home className="w-5 h-5" /> },
  { path: '/live', label: 'Live View', icon: <Camera className="w-5 h-5" /> },
  { path: '/diagnostics', label: 'Diagnostics', icon: <Activity className="w-5 h-5" /> },
]

const settingsNavItems: NavItem[] = [
  { path: '/settings/identification', label: 'Identification', icon: <User className="w-5 h-5" /> },
  { path: '/settings/network', label: 'Network', icon: <Network className="w-5 h-5" /> },
  { path: '/settings/time', label: 'Time', icon: <Clock className="w-5 h-5" /> },
  { path: '/settings/imaging', label: 'Imaging', icon: <Image className="w-5 h-5" /> },
  { path: '/settings/users', label: 'Users', icon: <Users className="w-5 h-5" /> },
  { path: '/settings/maintenance', label: 'Maintenance', icon: <Wrench className="w-5 h-5" /> },
  { path: '/settings/profiles', label: 'Profiles', icon: <Layers className="w-5 h-5" /> },
]

function NavLinkItem({ item, onClick }: { item: NavItem; onClick?: () => void }) {
  return (
    <NavLink
      to={item.path}
      onClick={onClick}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-3 px-3 py-2 rounded-md transition-colors',
          isActive
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
        )
      }
    >
      {({ isActive }) => (
        <>
          {item.icon}
          <span>{item.label}</span>
        </>
      )}
    </NavLink>
  )
}

function Sidebar() {
  const location = useLocation()
  const isSettingsActive = location.pathname.startsWith('/settings')

  return (
    <aside
      className="hidden lg:flex flex-col w-64 bg-card border-r border-border"
      aria-label="Main navigation"
    >
      {/* Logo */}
      <div className="flex items-center gap-2 px-4 py-4 border-b border-border">
        <Camera className="w-8 h-8 text-primary" />
        <span className="font-semibold text-lg">Camera.UI</span>
      </div>

      {/* Main Navigation */}
      <nav className="flex-1 p-4 space-y-1" aria-label="Main">
        {mainNavItems.map((item) => (
          <NavLinkItem key={item.path} item={item} />
        ))}

        {/* Settings Section */}
        <div className="pt-4">
          <div className="flex items-center gap-2 px-3 py-2 text-sm font-medium text-muted-foreground">
            <Settings className="w-4 h-4" />
            <span>Settings</span>
          </div>
          <div className="ml-2 mt-1 space-y-1">
            {settingsNavItems.map((item) => (
              <NavLinkItem key={item.path} item={item} />
            ))}
          </div>
        </div>
      </nav>
    </aside>
  )
}

function MobileNav() {
  const [isOpen, setIsOpen] = useState(false)
  const { logout, username } = useAuth()

  const handleClose = useCallback(() => setIsOpen(false), [])

  return (
    <Sheet open={isOpen} onOpenChange={setIsOpen}>
      <SheetTrigger asChild>
        <Button variant="ghost" size="icon" className="lg:hidden" aria-label="Open menu">
          <Menu className="w-6 h-6" />
        </Button>
      </SheetTrigger>
      <SheetContent side="left" className="w-64 p-0">
        <SheetHeader className="px-4 py-4 border-b border-border">
          <SheetTitle className="flex items-center gap-2">
            <Camera className="w-6 h-6 text-primary" />
            <span>Camera.UI</span>
          </SheetTitle>
        </SheetHeader>

        <nav className="flex-1 p-4 space-y-1" aria-label="Mobile navigation">
          {mainNavItems.map((item) => (
            <NavLinkItem key={item.path} item={item} onClick={handleClose} />
          ))}

          <div className="pt-4">
            <div className="px-3 py-2 text-sm font-medium text-muted-foreground">Settings</div>
            <div className="space-y-1">
              {settingsNavItems.map((item) => (
                <NavLinkItem key={item.path} item={item} onClick={handleClose} />
              ))}
            </div>
          </div>
        </nav>

        {/* User section */}
        <div className="p-4 border-t border-border">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">{username}</span>
            <Button variant="ghost" size="sm" onClick={logout} aria-label="Log out">
              <LogOut className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  )
}

function Header() {
  const { username, logout } = useAuth()

  return (
    <header className="flex items-center justify-between h-14 px-4 bg-card border-b border-border">
      <div className="flex items-center gap-2">
        <MobileNav />
        <h1 className="text-lg font-semibold lg:hidden">Camera.UI</h1>
      </div>

      <div className="flex items-center gap-4">
        <ConnectionStatusBadge status="connected" />
        <div className="hidden lg:flex items-center gap-4">
          <span className="text-sm text-muted-foreground">{username}</span>
          <Button variant="ghost" size="sm" onClick={logout} aria-label="Log out">
            <LogOut className="w-4 h-4 mr-2" />
            Logout
          </Button>
        </div>
      </div>
    </header>
  )
}

export default function Layout() {
  return (
    <div className="flex h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header />
        <main className="flex-1 overflow-auto p-4">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
