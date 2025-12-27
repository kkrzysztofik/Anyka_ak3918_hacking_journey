/**
 * User Management Page
 *
 * Manage system users, roles, and credentials.
 */
import React, { useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Edit2, Eye, EyeOff, Plus, Trash2, User, Users } from 'lucide-react';
import { useForm } from 'react-hook-form';
import { toast } from 'sonner';
import { z } from 'zod';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  SettingsCard,
  SettingsCardContent,
  SettingsCardDescription,
  SettingsCardHeader,
  SettingsCardTitle,
} from '@/components/ui/settings-card';
import {
  type OnvifUser as UserType,
  createUser,
  deleteUser,
  getUsers,
  setUser,
} from '@/services/userService';

// Schema for adding/editing user
const userSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(4, 'Password must be at least 4 characters'),
  role: z.enum(['Administrator', 'Operator', 'User', 'Anonymous'] as const),
});

type UserFormData = z.infer<typeof userSchema>;

export default function UserManagementPage() {
  const queryClient = useQueryClient();
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [editingUser, setEditingUser] = useState<UserType | null>(null);
  const [deletingUser, setDeletingUser] = useState<UserType | null>(null);
  const [showPassword, setShowPassword] = useState(false);

  // Fetch Users
  const {
    data: users,
    isLoading,
    error,
  } = useQuery<UserType[]>({
    queryKey: ['users'],
    queryFn: getUsers,
  });

  // Create User Mutation
  const createMutation = useMutation({
    mutationFn: (data: UserFormData) => createUser(data.username, data.password, data.role),
    onSuccess: () => {
      toast.success('User created successfully');
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setShowAddDialog(false);
      form.reset();
    },
    onError: (error) => {
      toast.error('Failed to create user', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Edit User Mutation
  const editMutation = useMutation({
    mutationFn: (data: UserFormData) => setUser(data.username, data.password, data.role),
    onSuccess: () => {
      toast.success('User updated successfully');
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setEditingUser(null);
      form.reset();
    },
    onError: (error) => {
      toast.error('Failed to update user', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Delete User Mutation
  const deleteMutation = useMutation({
    mutationFn: (username: string) => deleteUser(username),
    onSuccess: () => {
      toast.success('User deleted successfully');
      queryClient.invalidateQueries({ queryKey: ['users'] });
      setDeletingUser(null);
    },
    onError: (error) => {
      toast.error('Failed to delete user', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const form = useForm<UserFormData>({
    resolver: zodResolver(userSchema),
    defaultValues: {
      username: '',
      password: '',
      role: 'User',
    },
  });

  // Handle Edit Click
  const handleEditClick = (user: UserType) => {
    setEditingUser(user);
    form.reset({
      username: user.username,
      password: '', // Password not retrieved for security
      role: user.userLevel,
    });
  };

  // Handle Submit (Create or Edit)
  const onSubmit = (data: UserFormData) => {
    if (editingUser) {
      editMutation.mutate(data);
    } else {
      createMutation.mutate(data);
    }
  };

  if (isLoading) return <div className="text-white">Loading users...</div>;

  if (error) {
    return (
      <div className="text-red-500">
        Error loading users: {error instanceof Error ? error.message : String(error)}
      </div>
    );
  }

  return (
    <div
      className="absolute inset-0 overflow-auto bg-[#0d0d0d] lg:inset-[0_0_0_356.84px]"
      data-name="Container"
    >
      <div className="max-w-[1200px] p-[16px] pb-[80px] md:p-[32px] md:pb-[48px] lg:p-[48px]">
        {/* Header */}
        <div className="mb-[32px] flex items-center justify-between md:mb-[40px]">
          <div>
            <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">User Management</h1>
            <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
              Manage access accounts and security roles
            </p>
          </div>
          <Button
            onClick={() => {
              setEditingUser(null);
              form.reset({ username: '', password: '', role: 'User' });
              setShowAddDialog(true);
            }}
            className="rounded-[8px] bg-[#0a84ff] text-white hover:bg-[#0077ed]"
            data-testid="user-management-add-user-button"
          >
            <Plus className="mr-2 size-4" />
            Add User
          </Button>
        </div>

        <SettingsCard>
          <SettingsCardHeader>
            <div className="flex items-center gap-[12px]">
              <div className="flex size-[40px] items-center justify-center rounded-[10px] bg-[rgba(10,132,255,0.1)]">
                <Users className="size-5 text-[#0a84ff]" />
              </div>
              <div>
                <SettingsCardTitle>Users</SettingsCardTitle>
                <SettingsCardDescription>
                  {users?.length} accounts configured
                </SettingsCardDescription>
              </div>
            </div>
          </SettingsCardHeader>
          <SettingsCardContent className="p-0">
            <div className="overflow-hidden">
              <table className="w-full text-left text-sm text-[#a1a1a6]">
                <thead className="border-b border-[#3a3a3c] bg-[#1c1c1e] text-xs font-medium uppercase">
                  <tr>
                    <th className="px-6 py-4">User</th>
                    <th className="px-6 py-4">Role</th>
                    <th className="px-6 py-4 text-right">Actions</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#3a3a3c]">
                  {users?.map((user) => (
                    <tr key={user.username} className="transition-colors hover:bg-[#2c2c2e]/50">
                      <td className="flex items-center gap-3 px-6 py-4 font-medium text-white">
                        <div className="flex size-8 items-center justify-center rounded-full bg-[#3a3a3c] text-white">
                          <User className="size-4" />
                        </div>
                        {user.username}
                      </td>
                      <td className="px-6 py-4">
                        <Badge
                          className={`pointer-events-none rounded-md border px-2 py-1 text-xs font-medium ${user.userLevel === 'Administrator' ? 'border-[rgba(255,69,58,0.2)] bg-[rgba(255,69,58,0.1)] text-[#ff453a]' : ''} ${user.userLevel === 'Operator' ? 'border-[rgba(255,159,10,0.2)] bg-[rgba(255,159,10,0.1)] text-[#ff9f0a]' : ''} ${user.userLevel === 'User' ? 'border-[rgba(48,209,88,0.2)] bg-[rgba(48,209,88,0.1)] text-[#30d158]' : ''} ${user.userLevel === 'Anonymous' ? 'border-[rgba(142,142,147,0.2)] bg-[rgba(142,142,147,0.1)] text-[#8e8e93]' : ''} `}
                        >
                          {user.userLevel}
                        </Badge>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <div className="flex items-center justify-end gap-2">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleEditClick(user)}
                            className="h-8 w-8 text-[#a1a1a6] hover:bg-[#3a3a3c] hover:text-white"
                            data-testid={`edit-user-button-${user.username}`}
                          >
                            <Edit2 className="size-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => setDeletingUser(user)}
                            disabled={users.length <= 1} // Prevent deleting last user
                            className="h-8 w-8 text-[#a1a1a6] hover:bg-[rgba(220,38,38,0.1)] hover:text-[#dc2626]"
                            data-testid={`delete-user-button-${user.username}`}
                          >
                            <Trash2 className="size-4" />
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </SettingsCardContent>
        </SettingsCard>

        {/* Add/Edit Dialog */}
        <Dialog
          open={showAddDialog || !!editingUser}
          onOpenChange={(open) => {
            if (!open) {
              setShowAddDialog(false);
              setEditingUser(null);
            }
          }}
        >
          <DialogContent
            className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[425px]"
            data-testid={editingUser ? 'edit-user-dialog' : 'add-user-dialog'}
          >
            <DialogHeader>
              <DialogTitle
                className="text-white"
                data-testid={editingUser ? 'edit-user-dialog-title' : 'add-user-dialog-title'}
              >
                {editingUser ? 'Edit User' : 'Add User'}
              </DialogTitle>
              <DialogDescription className="text-[#a1a1a6]">
                {editingUser
                  ? 'Update user details and access level.'
                  : 'Create a new user account.'}
              </DialogDescription>
            </DialogHeader>
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                <FormField
                  control={form.control}
                  name="username"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Username</FormLabel>
                      <FormControl>
                        <Input
                          placeholder="jdoe"
                          {...field}
                          disabled={!!editingUser} // Can't change username in ONVIF usually directly without recreate, simplifying for now
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#0a84ff]"
                          data-testid={
                            editingUser
                              ? 'edit-user-dialog-username-input'
                              : 'add-user-dialog-username-input'
                          }
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="password"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Password</FormLabel>
                      <div className="relative">
                        <FormControl>
                          <Input
                            type={showPassword ? 'text' : 'password'}
                            placeholder={
                              editingUser ? 'Leave blank to keep current' : 'Enter password'
                            }
                            {...field}
                            className="border-[#3a3a3c] bg-transparent pr-10 text-white focus:border-[#0a84ff]"
                            data-testid={
                              editingUser
                                ? 'edit-user-dialog-password-input'
                                : 'add-user-dialog-password-input'
                            }
                          />
                        </FormControl>
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="absolute top-0 right-0 h-full px-3 py-2 text-[#a1a1a6] hover:text-white"
                          onClick={() => setShowPassword(!showPassword)}
                          data-testid={
                            editingUser
                              ? 'edit-user-dialog-password-toggle-button'
                              : 'add-user-dialog-password-toggle-button'
                          }
                        >
                          {showPassword ? (
                            <EyeOff className="size-4" />
                          ) : (
                            <Eye className="size-4" />
                          )}
                        </Button>
                      </div>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="role"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Role</FormLabel>
                      <Select
                        onValueChange={field.onChange}
                        defaultValue={field.value}
                        value={field.value}
                      >
                        <FormControl>
                          <SelectTrigger
                            className="border-[#3a3a3c] bg-transparent text-white"
                            data-testid={
                              editingUser
                                ? 'edit-user-dialog-role-select'
                                : 'add-user-dialog-role-select'
                            }
                          >
                            <SelectValue placeholder="Select a role" />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
                          <SelectItem
                            value="Administrator"
                            data-testid={`${editingUser ? 'edit-user-dialog' : 'add-user-dialog'}-role-option-Administrator`}
                          >
                            Administrator
                          </SelectItem>
                          <SelectItem
                            value="Operator"
                            data-testid={`${editingUser ? 'edit-user-dialog' : 'add-user-dialog'}-role-option-Operator`}
                          >
                            Operator
                          </SelectItem>
                          <SelectItem
                            value="User"
                            data-testid={`${editingUser ? 'edit-user-dialog' : 'add-user-dialog'}-role-option-User`}
                          >
                            User
                          </SelectItem>
                          <SelectItem
                            value="Anonymous"
                            data-testid={`${editingUser ? 'edit-user-dialog' : 'add-user-dialog'}-role-option-Anonymous`}
                          >
                            Anonymous
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <DialogFooter className="mt-4">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => {
                      setShowAddDialog(false);
                      setEditingUser(null);
                    }}
                    className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
                    data-testid={editingUser ? 'edit-user-dialog-cancel' : 'add-user-dialog-cancel'}
                  >
                    Cancel
                  </Button>
                  <Button
                    type="submit"
                    disabled={createMutation.isPending || editMutation.isPending}
                    className="bg-[#0a84ff] text-white hover:bg-[#0077ed]"
                    data-testid={editingUser ? 'edit-user-dialog-save' : 'add-user-dialog-save'}
                  >
                    {createMutation.isPending || editMutation.isPending ? 'Saving...' : 'Save User'}
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation */}
        <AlertDialog open={!!deletingUser} onOpenChange={() => setDeletingUser(null)}>
          <AlertDialogContent
            className="border-[#3a3a3c] bg-[#1c1c1e] text-white"
            data-testid="delete-user-dialog"
          >
            <AlertDialogHeader>
              <AlertDialogTitle className="text-white" data-testid="delete-user-dialog-title">
                Delete User?
              </AlertDialogTitle>
              <AlertDialogDescription className="text-[#a1a1a6]">
                Are you sure you want to delete user "<strong>{deletingUser?.username}</strong>"?
                This action cannot be undone.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel
                className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]"
                data-testid="delete-user-dialog-cancel"
              >
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={() => deletingUser && deleteMutation.mutate(deletingUser.username)}
                className="bg-[#dc2626] text-white hover:bg-[#ef4444]"
                data-testid="delete-user-dialog-confirm"
              >
                {deleteMutation.isPending ? 'Deleting...' : 'Delete User'}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  );
}
