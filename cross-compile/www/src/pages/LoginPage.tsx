/**
 * Login Page
 *
 * Sign-in form with credential verification via authService.
 */

import React, { useState } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { z } from 'zod'
import { Camera, Eye, EyeOff, Loader2 } from 'lucide-react'
import { toast } from 'sonner'
import { useAuth } from '@/hooks/useAuth'
import { verifyCredentials } from '@/services/authService'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from '@/components/ui/form'

const loginSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
})

type LoginFormData = z.infer<typeof loginSchema>

export default function LoginPage() {
  const navigate = useNavigate()
  const location = useLocation()
  const { login, isAuthenticated } = useAuth()
  const [isLoading, setIsLoading] = useState(false)
  const [showPassword, setShowPassword] = useState(false)

  const form = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      username: '',
      password: '',
    },
  })

  // Redirect if already authenticated
  React.useEffect(() => {
    if (isAuthenticated) {
      const rawFrom = (location.state as { from?: { pathname: string } })?.from?.pathname
      let safeFrom = '/'
      // Strict validation: must be a string, start with /, and NOT start with //
      if (rawFrom && typeof rawFrom === 'string' && rawFrom.startsWith('/') && !rawFrom.startsWith('//')) {
        safeFrom = rawFrom
      }
      navigate(safeFrom, { replace: true })
    }
  }, [isAuthenticated, navigate, location])

  const onSubmit = async (data: LoginFormData) => {
    setIsLoading(true)

    try {
      const result = await verifyCredentials(data.username, data.password)

      if (result.success) {
        login(data.username, data.password)
        toast.success('Successfully signed in')
      } else {
        toast.error(result.error || 'Authentication failed')
      }
    } catch (error) {
      toast.error('An error occurred during sign in')
      console.error('Login error:', error)
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="size-full min-h-screen bg-dark-bg flex items-center justify-center p-4">
      <div className="w-full max-w-[440px] bg-dark-sidebar border border-dark-border rounded-[16px] p-6 sm:p-8 md:p-12">
        {/* Logo/Brand */}
        <div className="flex flex-col items-center mb-8 md:mb-10">
          <div className="size-[56px] md:size-[64px] bg-accent-red rounded-[16px] flex items-center justify-center mb-3 md:mb-4 shadow-lg shadow-accent-red/20">
            <Camera className="size-[28px] md:size-[32px] text-white" />
          </div>
          <h1 className="text-white text-[20px] md:text-[24px] font-semibold mb-1.5 md:mb-2 text-center">ONVIF Device Manager</h1>
          <p className="text-dark-secondary-text text-[13px] md:text-[14px] text-center">API Management, Live Preview & Telemetry</p>
        </div>

        {/* Login Form */}
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-5 md:space-y-6">
            <FormField
              control={form.control}
              name="username"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text text-[13px] md:text-[14px]">Username</FormLabel>
                  <FormControl>
                    <Input
                      placeholder="Enter username"
                      autoComplete="username"
                      disabled={isLoading}
                      className="h-11 bg-transparent border-dark-border text-white text-[15px] md:text-[16px] placeholder:text-dark-border focus-visible:ring-accent-red focus-visible:border-accent-red transition-colors"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage className="text-accent-red text-xs mt-1" />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="password"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text text-[13px] md:text-[14px]">Password</FormLabel>
                  <FormControl>
                    <div className="relative">
                      <Input
                        type={showPassword ? 'text' : 'password'}
                        placeholder="Enter password"
                        autoComplete="current-password"
                        disabled={isLoading}
                        className="h-11 bg-transparent border-dark-border text-white text-[15px] md:text-[16px] placeholder:text-dark-border pr-12 focus-visible:ring-accent-red focus-visible:border-accent-red transition-colors"
                        {...field}
                      />
                      <button
                        type="button"
                        onClick={() => setShowPassword(!showPassword)}
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-dark-secondary-text hover:text-white transition-colors"
                        disabled={isLoading}
                      >
                        {showPassword ? (
                          <EyeOff className="size-5" />
                        ) : (
                          <Eye className="size-5" />
                        )}
                      </button>
                    </div>
                  </FormControl>
                  <FormMessage className="text-accent-red text-xs mt-1" />
                </FormItem>
              )}
            />

            <Button
              type="submit"
              className="w-full h-11 bg-accent-red hover:bg-accent-red/90 text-white font-semibold text-[15px] md:text-[16px] transition-colors mt-2"
              disabled={isLoading}
            >
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Signing in...
                </>
              ) : (
                'Sign In'
              )}
            </Button>
          </form>
        </Form>

        {/* Footer */}
        <div className="mt-8 md:mt-10 text-center">
          <p className="text-dark-secondary-text text-[11px] md:text-[12px]">
            © 2025 Krzysztof Krzysztofik. All rights reserved.
          </p>
        </div>
      </div>
    </div>
  )
}
