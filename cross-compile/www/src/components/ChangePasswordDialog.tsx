import React, { useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { Eye, EyeOff, KeyRound, Loader2 } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
import { getUsers, setUser } from '@/services/userService';

const changePasswordSchema = z
  .object({
    currentPassword: z.string().min(1, 'Current password is required'),
    newPassword: z
      .string()
      .min(6, 'Password must be at least 6 characters')
      .max(32, 'Password must be at most 32 characters'),
    confirmPassword: z.string().min(1, 'Please confirm your new password'),
  })
  .refine((data) => data.newPassword === data.confirmPassword, {
    message: "Passwords don't match",
    path: ['confirmPassword'],
  });

type ChangePasswordFormData = z.infer<typeof changePasswordSchema>;

interface ChangePasswordDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ChangePasswordDialog({ open, onOpenChange }: ChangePasswordDialogProps) {
  const { username, login } = useAuth();
  const [isLoading, setIsLoading] = useState(false);
  const [showCurrentPassword, setShowCurrentPassword] = useState(false);
  const [showNewPassword, setShowNewPassword] = useState(false);

  const form = useForm<ChangePasswordFormData>({
    resolver: zodResolver(changePasswordSchema),
    defaultValues: {
      currentPassword: '',
      newPassword: '',
      confirmPassword: '',
    },
  });

  const onSubmit = async (data: ChangePasswordFormData) => {
    if (!username) return;

    setIsLoading(true);
    try {
      // 1. Verify current password
      const verifyResult = await verifyCredentials(username, data.currentPassword);
      if (!verifyResult.success) {
        form.setError('currentPassword', {
          message: 'Invalid current password',
        });
        setIsLoading(false);
        return;
      }

      // 2. Fetch user level
      const users = await getUsers();
      const currentUser = users.find((u) => u.username === username);

      if (!currentUser) {
        toast.error('Could not find user information');
        setIsLoading(false);
        return;
      }

      // 3. Update password via SOAP
      await setUser(username, data.newPassword, currentUser.userLevel);

      // 4. Update local credentials
      login(username, data.newPassword);

      toast.success('Password updated successfully');
      onOpenChange(false);
      form.reset();
    } catch (error) {
      console.error('Failed to change password:', error);
      toast.error('Failed to update password. Please check your connection.');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(val) => {
        if (!isLoading) {
          onOpenChange(val);
          if (!val) form.reset();
        }
      }}
    >
      <DialogContent className="bg-dark-sidebar border-dark-border text-white sm:max-w-[425px]">
        <DialogHeader>
          <div className="mb-2 flex items-center gap-3">
            <div className="bg-accent-red/10 flex size-10 items-center justify-center rounded-lg">
              <KeyRound className="text-accent-red size-6" />
            </div>
            <div>
              <DialogTitle className="text-xl">Change Password</DialogTitle>
              <DialogDescription className="text-dark-secondary-text">
                Update your account password for {username}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4 py-4">
            <FormField
              control={form.control}
              name="currentPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text">Current Password</FormLabel>
                  <FormControl>
                    <div className="relative">
                      <Input
                        type={showCurrentPassword ? 'text' : 'password'}
                        placeholder="••••••••"
                        disabled={isLoading}
                        className="bg-dark-bg border-dark-border focus-visible:ring-accent-red pr-10"
                        {...field}
                      />
                      <button
                        type="button"
                        onClick={() => setShowCurrentPassword(!showCurrentPassword)}
                        className="text-dark-secondary-text absolute top-1/2 right-3 -translate-y-1/2 transition-colors hover:text-white"
                      >
                        {showCurrentPassword ? (
                          <EyeOff className="size-4" />
                        ) : (
                          <Eye className="size-4" />
                        )}
                      </button>
                    </div>
                  </FormControl>
                  <FormMessage className="text-accent-red text-xs" />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="newPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text">New Password</FormLabel>
                  <FormControl>
                    <div className="relative">
                      <Input
                        type={showNewPassword ? 'text' : 'password'}
                        placeholder="••••••••"
                        disabled={isLoading}
                        className="bg-dark-bg border-dark-border focus-visible:ring-accent-red pr-10"
                        {...field}
                      />
                      <button
                        type="button"
                        onClick={() => setShowNewPassword(!showNewPassword)}
                        className="text-dark-secondary-text absolute top-1/2 right-3 -translate-y-1/2 transition-colors hover:text-white"
                      >
                        {showNewPassword ? (
                          <EyeOff className="size-4" />
                        ) : (
                          <Eye className="size-4" />
                        )}
                      </button>
                    </div>
                  </FormControl>
                  <FormMessage className="text-accent-red text-xs" />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="confirmPassword"
              render={({ field }) => (
                <FormItem>
                  <FormLabel className="text-dark-secondary-text">Confirm New Password</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="••••••••"
                      disabled={isLoading}
                      className="bg-dark-bg border-dark-border focus-visible:ring-accent-red"
                      {...field}
                    />
                  </FormControl>
                  <FormMessage className="text-accent-red text-xs" />
                </FormItem>
              )}
            />

            <DialogFooter className="pt-4">
              <Button
                type="button"
                variant="ghost"
                onClick={() => onOpenChange(false)}
                disabled={isLoading}
                className="text-dark-secondary-text hover:bg-dark-hover hover:text-white"
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isLoading}
                className="bg-accent-red hover:bg-accent-red/90 min-w-[140px] text-white"
              >
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Updating...
                  </>
                ) : (
                  'Change Password'
                )}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
