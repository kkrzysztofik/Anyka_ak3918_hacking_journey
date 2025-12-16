/**
 * User Management Page
 *
 * Manage users and passwords.
 */

import React, { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Users, Plus, Key, Trash2, Loader2, Shield, UserCheck, User as UserIcon } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { AddUserDialog, ChangePasswordDialog } from '@/components/users/UserDialogs'
import { getUsers, createUser, deleteUser, setUser, type OnvifUser, type UserLevel } from '@/services/userService'
import { useAuth } from '@/hooks/useAuth'

const userLevelIcons: Record<UserLevel, React.ReactNode> = {
  Administrator: <Shield className="w-4 h-4 text-destructive" />,
  Operator: <UserCheck className="w-4 h-4 text-primary" />,
  User: <UserIcon className="w-4 h-4 text-muted-foreground" />,
  Anonymous: <UserIcon className="w-4 h-4 text-muted-foreground" />,
}

export default function UserManagementPage() {
  const queryClient = useQueryClient()
  const { username: currentUser } = useAuth()

  const [showAddDialog, setShowAddDialog] = useState(false)
  const [showPasswordDialog, setShowPasswordDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedUser, setSelectedUser] = useState<OnvifUser | null>(null)

  // Fetch users
  const { data: users, isLoading, error } = useQuery<OnvifUser[]>({
    queryKey: ['users'],
    queryFn: getUsers,
  })

  // Create user mutation
  const createMutation = useMutation({
    mutationFn: ({ username, password, userLevel }: { username: string; password: string; userLevel: UserLevel }) =>
      createUser(username, password, userLevel),
    onSuccess: () => {
      toast.success('User created successfully')
      queryClient.invalidateQueries({ queryKey: ['users'] })
    },
    onError: (error) => {
      toast.error('Failed to create user', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  // Delete user mutation
  const deleteMutation = useMutation({
    mutationFn: deleteUser,
    onSuccess: () => {
      toast.success('User deleted successfully')
      queryClient.invalidateQueries({ queryKey: ['users'] })
      setShowDeleteDialog(false)
      setSelectedUser(null)
    },
    onError: (error) => {
      toast.error('Failed to delete user', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  // Change password mutation
  const passwordMutation = useMutation({
    mutationFn: ({ username, password, userLevel }: { username: string; password: string; userLevel: UserLevel }) =>
      setUser(username, password, userLevel),
    onSuccess: () => {
      toast.success('Password changed successfully')
      setShowPasswordDialog(false)
      setSelectedUser(null)
    },
    onError: (error) => {
      toast.error('Failed to change password', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const handleAddUser = async (username: string, password: string, userLevel: UserLevel) => {
    await createMutation.mutateAsync({ username, password, userLevel })
  }

  const handleChangePassword = async (password: string) => {
    if (!selectedUser) return
    await passwordMutation.mutateAsync({
      username: selectedUser.username,
      password,
      userLevel: selectedUser.userLevel,
    })
  }

  const handleDeleteUser = () => {
    if (!selectedUser) return
    deleteMutation.mutate(selectedUser.username)
  }

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Users</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load users</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Users</h1>
        <Button onClick={() => setShowAddDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Add User
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="w-5 h-5" />
            User Accounts
          </CardTitle>
          <CardDescription>
            Manage user accounts and access levels
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading users...
            </div>
          ) : users && users.length > 0 ? (
            <div className="border rounded-lg">
              <table className="w-full">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="text-left p-3 font-medium">Username</th>
                    <th className="text-left p-3 font-medium">Level</th>
                    <th className="text-right p-3 font-medium">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {users.map((user) => (
                    <tr key={user.username} className="border-b last:border-0">
                      <td className="p-3">
                        <div className="flex items-center gap-2">
                          {userLevelIcons[user.userLevel]}
                          <span className="font-medium">{user.username}</span>
                          {user.username === currentUser && (
                            <span className="text-xs text-muted-foreground">(you)</span>
                          )}
                        </div>
                      </td>
                      <td className="p-3 text-muted-foreground">{user.userLevel}</td>
                      <td className="p-3">
                        <div className="flex justify-end gap-2">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => {
                              setSelectedUser(user)
                              setShowPasswordDialog(true)
                            }}
                          >
                            <Key className="w-4 h-4" />
                            <span className="sr-only">Change password</span>
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={user.username === currentUser}
                            onClick={() => {
                              setSelectedUser(user)
                              setShowDeleteDialog(true)
                            }}
                          >
                            <Trash2 className="w-4 h-4 text-destructive" />
                            <span className="sr-only">Delete user</span>
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-muted-foreground py-4">No users found</p>
          )}
        </CardContent>
      </Card>

      {/* Add User Dialog */}
      <AddUserDialog
        open={showAddDialog}
        onOpenChange={setShowAddDialog}
        onSubmit={handleAddUser}
      />

      {/* Change Password Dialog */}
      {selectedUser && (
        <ChangePasswordDialog
          open={showPasswordDialog}
          onOpenChange={setShowPasswordDialog}
          username={selectedUser.username}
          onSubmit={handleChangePassword}
        />
      )}

      {/* Delete Confirmation */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete User</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete user <strong>{selectedUser?.username}</strong>?
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteUser}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {deleteMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
