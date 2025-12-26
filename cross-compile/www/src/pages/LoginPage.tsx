/**
 * Login Page
 *
 * Sign-in form with credential verification via authService.
 */
import React, { useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { Camera, Eye, EyeOff, Loader2 } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { useLocation, useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { z } from 'zod';

import { Button } from '@/components/ui/button';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { useAuth } from '@/hooks/useAuth';
import { verifyCredentials } from '@/services/authService';

const loginSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
});

type LoginFormData = z.infer<typeof loginSchema>;

export default function LoginPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const { login, isAuthenticated } = useAuth();
  const [isLoading, setIsLoading] = useState(false);
  const [showPassword, setShowPassword] = useState(false);

  const form = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      username: '',
      password: '',
    },
  });

  // Redirect if already authenticated
  React.useEffect(() => {
    if (isAuthenticated) {
      const rawFrom = (location.state as { from?: { pathname: string } })?.from?.pathname;
      let safeFrom = '/';
      // Strict validation: must be a string, start with /, and NOT start with //
      if (
        rawFrom &&
        typeof rawFrom === 'string' &&
        rawFrom.startsWith('/') &&
        !rawFrom.startsWith('//')
      ) {
        safeFrom = rawFrom;
      }
      navigate(safeFrom, { replace: true });
    }
  }, [isAuthenticated, navigate, location]);

  const onSubmit = async (data: LoginFormData) => {
    setIsLoading(true);

    try {
      const result = await verifyCredentials(data.username, data.password);

      if (result.success) {
        login(data.username, data.password);
        toast.success('Successfully signed in');
      } else {
        toast.error(result.error || 'Authentication failed');
      }
    } catch {
      toast.error('An error occurred during sign in');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="bg-dark-bg flex size-full min-h-screen items-center justify-center p-4">
      <div className="bg-dark-sidebar border-dark-border w-full max-w-[440px] rounded-[16px] border p-6 sm:p-8 md:p-12">
        {/* Logo/Brand */}
        <div className="mb-8 flex flex-col items-center md:mb-10">
          <div className="bg-accent-red shadow-accent-red/20 mb-3 flex size-[56px] items-center justify-center rounded-[16px] shadow-lg md:mb-4 md:size-[64px]">
            <Camera className="size-[28px] text-white md:size-[32px]" />
          </div>
          <h1
            className="mb-1.5 text-center text-[20px] font-semibold text-white md:mb-2 md:text-[24px]"
            data-testid="login-page-title"
          >
            ONVIF Device Manager
          </h1>
          <p
            className="text-dark-secondary-text text-center text-[13px] md:text-[14px]"
            data-testid="login-page-subtitle"
          >
            API Management, Live Preview & Telemetry
          </p>
        </div>

        {/* Login Form */}
        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-5 md:space-y-6">
            <FormField
              control={form.control}
              name="username"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text text-[13px] md:text-[14px]">
                    Username
                  </FormLabel>
                  <FormControl>
                    <Input
                      placeholder="Enter username"
                      autoComplete="username"
                      disabled={isLoading}
                      className="border-dark-border placeholder:text-dark-border focus-visible:ring-accent-red focus-visible:border-accent-red h-11 bg-transparent text-[15px] text-white transition-colors md:text-[16px]"
                      data-testid="login-form-username-input"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage
                    className="text-accent-red mt-1 text-xs"
                    data-testid="login-username-error"
                  />
                </FormItem>
              )}
            />
            <FormField
              control={form.control}
              name="password"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text text-[13px] md:text-[14px]">
                    Password
                  </FormLabel>
                  <FormControl>
                    <div className="relative">
                      <Input
                        type={showPassword ? 'text' : 'password'}
                        placeholder="Enter password"
                        autoComplete="current-password"
                        disabled={isLoading}
                        className="border-dark-border placeholder:text-dark-border focus-visible:ring-accent-red focus-visible:border-accent-red h-11 bg-transparent pr-12 text-[15px] text-white transition-colors md:text-[16px]"
                        data-testid="login-form-password-input"
                        {...field}
                      />
                      <button
                        type="button"
                        onClick={() => setShowPassword(!showPassword)}
                        className="text-dark-secondary-text absolute top-1/2 right-3 -translate-y-1/2 transition-colors hover:text-white"
                        disabled={isLoading}
                        data-testid="login-form-password-toggle-button"
                      >
                        {showPassword ? <EyeOff className="size-5" /> : <Eye className="size-5" />}
                      </button>
                    </div>
                  </FormControl>
                  <FormMessage
                    className="text-accent-red mt-1 text-xs"
                    data-testid="login-password-error"
                  />
                </FormItem>
              )}
            />

            <Button
              type="submit"
              className="bg-accent-red hover:bg-accent-red/90 mt-2 h-11 w-full text-[15px] font-semibold text-white transition-colors md:text-[16px]"
              disabled={isLoading}
              data-testid="login-form-submit-button"
            >
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  <span data-testid="login-loading-text">Signing in...</span>
                </>
              ) : (
                'Sign In'
              )}
            </Button>
          </form>
        </Form>

        {/* Footer */}
        <div className="mt-8 text-center md:mt-10">
          <p
            className="text-dark-secondary-text text-[11px] md:text-[12px]"
            data-testid="login-page-copyright"
          >
            © 2025 Krzysztof Krzysztofik. All rights reserved.
          </p>
        </div>
      </div>
    </div>
  );
}
