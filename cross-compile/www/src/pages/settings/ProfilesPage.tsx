/**
 * Profiles Page
 *
 * Manage media profiles.
 */

import React, { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Layers, Plus, Trash2, Loader2, Video, Mic } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
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
import { getProfiles, createProfile, deleteProfile, type MediaProfile } from '@/services/profileService'

export default function ProfilesPage() {
  const queryClient = useQueryClient()
  const [showCreateDialog, setShowCreateDialog] = useState(false)
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)
  const [selectedProfile, setSelectedProfile] = useState<MediaProfile | null>(null)
  const [newProfileName, setNewProfileName] = useState('')

  // Fetch profiles
  const { data: profiles, isLoading, error } = useQuery<MediaProfile[]>({
    queryKey: ['profiles'],
    queryFn: getProfiles,
  })

  // Create mutation
  const createMutation = useMutation({
    mutationFn: createProfile,
    onSuccess: () => {
      toast.success('Profile created successfully')
      queryClient.invalidateQueries({ queryKey: ['profiles'] })
      setShowCreateDialog(false)
      setNewProfileName('')
    },
    onError: (error) => {
      toast.error('Failed to create profile', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: deleteProfile,
    onSuccess: () => {
      toast.success('Profile deleted successfully')
      queryClient.invalidateQueries({ queryKey: ['profiles'] })
      setShowDeleteDialog(false)
      setSelectedProfile(null)
    },
    onError: (error) => {
      toast.error('Failed to delete profile', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const handleCreate = () => {
    if (!newProfileName.trim()) return
    createMutation.mutate(newProfileName.trim())
  }

  const handleDelete = () => {
    if (!selectedProfile) return
    deleteMutation.mutate(selectedProfile.token)
  }

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Profiles</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load profiles</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Profiles</h1>
        <Button onClick={() => setShowCreateDialog(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Create Profile
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Layers className="w-5 h-5" />
            Media Profiles
          </CardTitle>
          <CardDescription>
            Manage streaming profiles for video and audio
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading profiles...
            </div>
          ) : profiles && profiles.length > 0 ? (
            <div className="space-y-3">
              {profiles.map((profile) => (
                <div
                  key={profile.token}
                  className="flex items-center justify-between p-4 border rounded-lg"
                >
                  <div>
                    <h3 className="font-medium">{profile.name}</h3>
                    <div className="flex gap-4 mt-1 text-sm text-muted-foreground">
                      {profile.videoEncoderToken && (
                        <span className="flex items-center gap-1">
                          <Video className="w-3 h-3" />
                          Video
                        </span>
                      )}
                      {profile.audioEncoderToken && (
                        <span className="flex items-center gap-1">
                          <Mic className="w-3 h-3" />
                          Audio
                        </span>
                      )}
                      <span className="font-mono text-xs">{profile.token}</span>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setSelectedProfile(profile)
                      setShowDeleteDialog(true)
                    }}
                  >
                    <Trash2 className="w-4 h-4 text-destructive" />
                  </Button>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-muted-foreground py-4">No profiles found</p>
          )}
        </CardContent>
      </Card>

      {/* Create Dialog */}
      <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Profile</DialogTitle>
            <DialogDescription>Create a new media profile</DialogDescription>
          </DialogHeader>
          <div className="py-4">
            <Input
              placeholder="Profile Name"
              value={newProfileName}
              onChange={(e) => setNewProfileName(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowCreateDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleCreate}
              disabled={createMutation.isPending || !newProfileName.trim()}
            >
              {createMutation.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              Create
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Dialog */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Profile</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete profile <strong>{selectedProfile?.name}</strong>?
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDelete}
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
