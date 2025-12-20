/**
 * Profiles Page
 *
 * Manage media profiles and their configurations (Video/Audio Sources, Encoders, PTZ, Analytics, Metadata).
 */
import React, { useState } from 'react';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Activity,
  ChevronDown,
  ChevronUp,
  FileText,
  Mic,
  Monitor,
  Move,
  Plus,
  Trash2,
  Video,
} from 'lucide-react';
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
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
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
  type MediaProfile,
  createProfile,
  deleteProfile,
  getProfiles,
} from '@/services/profileService';

// Schema for creating a profile
const createProfileSchema = z.object({
  name: z.string().min(1, 'Profile name is required'),
});

type CreateProfileData = z.infer<typeof createProfileSchema>;

export default function ProfilesPage() {
  const queryClient = useQueryClient();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [profileToDelete, setProfileToDelete] = useState<MediaProfile | null>(null);

  // Track open state for each profile card
  const [openProfiles, setOpenProfiles] = useState<Record<string, boolean>>({});

  const toggleProfile = (token: string) => {
    setOpenProfiles((prev) => ({ ...prev, [token]: !prev[token] }));
  };

  // Fetch profiles
  const {
    data: profiles,
    isLoading,
    error,
  } = useQuery<MediaProfile[]>({
    queryKey: ['profiles'],
    queryFn: getProfiles,
  });

  // Create Mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateProfileData) => createProfile(data.name),
    onSuccess: () => {
      toast.success('Profile created successfully');
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      setShowCreateDialog(false);
      form.reset();
    },
    onError: (error) => {
      toast.error('Failed to create profile', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  // Delete Mutation
  const deleteMutation = useMutation({
    mutationFn: (token: string) => deleteProfile(token),
    onSuccess: () => {
      toast.success('Profile deleted successfully');
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      setProfileToDelete(null);
    },
    onError: (error) => {
      toast.error('Failed to delete profile', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const form = useForm<CreateProfileData>({
    resolver: zodResolver(createProfileSchema),
    defaultValues: { name: '' },
  });

  const onSubmit = (data: CreateProfileData) => {
    createMutation.mutate(data);
  };

  const handleDeleteConfirm = () => {
    if (profileToDelete) {
      deleteMutation.mutate(profileToDelete.token);
    }
  };

  if (isLoading) return <div className="text-white">Loading profiles...</div>;

  if (error) {
    return <div className="text-red-500">Error loading profiles: {(error as Error).message}</div>;
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
            <h1 className="mb-[8px] text-[22px] text-white md:text-[28px]">Profiles</h1>
            <p className="text-[13px] text-[#a1a1a6] md:text-[14px]">
              Manage media profiles for RTSP streaming
            </p>
          </div>
          <Button
            onClick={() => setShowCreateDialog(true)}
            className="rounded-[8px] bg-[#0a84ff] text-white hover:bg-[#0077ed]"
          >
            <Plus className="mr-2 size-4" />
            Create Profile
          </Button>
        </div>

        <div className="space-y-[16px]">
          {profiles?.map((profile) => (
            <Collapsible
              key={profile.token}
              open={openProfiles[profile.token]}
              onOpenChange={() => toggleProfile(profile.token)}
              className="w-full"
            >
              <div className="overflow-hidden rounded-[12px] border border-[#3a3a3c] bg-[#1c1c1e]">
                {/* Profile Header */}
                <div className="flex items-center justify-between p-[16px]">
                  <div className="flex items-center gap-[16px]">
                    <CollapsibleTrigger asChild>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="p-0 text-[#a1a1a6] hover:bg-transparent hover:text-white"
                      >
                        {openProfiles[profile.token] ? (
                          <ChevronUp className="size-5" />
                        ) : (
                          <ChevronDown className="size-5" />
                        )}
                      </Button>
                    </CollapsibleTrigger>

                    <div className="flex flex-col">
                      <span className="text-[16px] font-medium text-white">{profile.name}</span>
                      <span className="font-mono text-[12px] text-[#a1a1a6]">{profile.token}</span>
                    </div>

                    {/* Active Configuration Badges */}
                    <div className="ml-[16px] hidden items-center gap-[8px] md:flex">
                      {profile.videoSourceConfiguration && (
                        <div className="rounded-[4px] border border-[rgba(10,132,255,0.2)] bg-[rgba(10,132,255,0.1)] px-[8px] py-[2px] text-[11px] font-medium text-[#0a84ff]">
                          Video Source
                        </div>
                      )}
                      {profile.videoEncoderConfiguration && (
                        <div className="rounded-[4px] border border-[rgba(48,209,88,0.2)] bg-[rgba(48,209,88,0.1)] px-[8px] py-[2px] text-[11px] font-medium text-[#30d158]">
                          H.264
                        </div>
                      )}
                      {profile.audioSourceConfiguration && (
                        <div className="rounded-[4px] border border-[rgba(255,159,10,0.2)] bg-[rgba(255,159,10,0.1)] px-[8px] py-[2px] text-[11px] font-medium text-[#ff9f0a]">
                          Audio
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-[8px]">
                    {!profile.fixed && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => setProfileToDelete(profile)}
                        className="text-[#a1a1a6] hover:bg-[rgba(220,38,38,0.1)] hover:text-[#dc2626]"
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    )}
                    {/* Stub Edit Button - acts as trigger for collapsible usually, but here we just have expand */}
                  </div>
                </div>

                {/* Expanded Content */}
                <CollapsibleContent>
                  <div className="border-t border-[#3a3a3c] bg-[#151516] p-[16px] md:p-[24px]">
                    <div className="grid grid-cols-1 gap-[16px] md:grid-cols-2 lg:grid-cols-3">
                      {/* Video Source Config */}
                      <ConfigSection
                        title="Video Source"
                        icon={<Monitor className="size-4 text-[#0a84ff]" />}
                        active={!!profile.videoSourceConfiguration}
                        token={profile.videoSourceConfiguration?.token}
                        details={profile.videoSourceConfiguration?.name}
                      />

                      {/* Video Encoder Config */}
                      <ConfigSection
                        title="Video Encoder"
                        icon={<Video className="size-4 text-[#30d158]" />}
                        active={!!profile.videoEncoderConfiguration}
                        token={profile.videoEncoderConfiguration?.token}
                        details={
                          profile.videoEncoderConfiguration?.name
                            ? `${profile.videoEncoderConfiguration.name} (${profile.videoEncoderConfiguration.encoding})`
                            : ''
                        }
                      />

                      {/* Audio Source Config */}
                      <ConfigSection
                        title="Audio Source"
                        icon={<Mic className="size-4 text-[#ff9f0a]" />}
                        active={!!profile.audioSourceConfiguration}
                        token={profile.audioSourceConfiguration?.token}
                        details={profile.audioSourceConfiguration?.name}
                      />

                      {/* Audio Encoder Config */}
                      <ConfigSection
                        title="Audio Encoder"
                        icon={<Activity className="size-4 text-[#bf5af2]" />}
                        active={!!profile.audioEncoderConfiguration}
                        token={profile.audioEncoderConfiguration?.token}
                        details={profile.audioEncoderConfiguration?.name}
                      />

                      {/* PTZ Config */}
                      <ConfigSection
                        title="PTZ Configuration"
                        icon={<Move className="size-4 text-[#ff375f]" />}
                        active={!!profile.ptzConfiguration}
                        token={profile.ptzConfiguration?.token}
                        details={profile.ptzConfiguration?.name}
                      />

                      {/* Metadata/Analytics */}
                      <ConfigSection
                        title="Metadata & Analytics"
                        icon={<FileText className="size-4 text-[#64d2ff]" />}
                        active={!!profile.metadataConfiguration} // Assuming metadata implies analytics for now
                        token={profile.metadataConfiguration?.token}
                        details={profile.metadataConfiguration?.name}
                      />
                    </div>
                  </div>
                </CollapsibleContent>
              </div>
            </Collapsible>
          ))}

          {profiles?.length === 0 && (
            <div className="py-[48px] text-center text-[#a1a1a6]">
              No media profiles found. Create one to get started.
            </div>
          )}
        </div>

        {/* Create Dialog */}
        <Dialog open={showCreateDialog} onOpenChange={setShowCreateDialog}>
          <DialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[425px]">
            <DialogHeader>
              <DialogTitle className="text-white">Create Profile</DialogTitle>
              <DialogDescription className="text-[#a1a1a6]">
                Add a new media profile. You will be able to configure sources and encoders after
                creation.
              </DialogDescription>
            </DialogHeader>
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                <FormField
                  control={form.control}
                  name="name"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel className="text-[#a1a1a6]">Profile Name</FormLabel>
                      <FormControl>
                        <Input
                          placeholder="My Profile"
                          {...field}
                          className="border-[#3a3a3c] bg-transparent text-white focus:border-[#0a84ff]"
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
                <DialogFooter className="mt-4">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => setShowCreateDialog(false)}
                    className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
                  >
                    Cancel
                  </Button>
                  <Button type="submit" className="bg-[#0a84ff] text-white hover:bg-[#0077ed]">
                    Create
                  </Button>
                </DialogFooter>
              </form>
            </Form>
          </DialogContent>
        </Dialog>

        {/* Delete Confirmation */}
        <AlertDialog open={!!profileToDelete} onOpenChange={() => setProfileToDelete(null)}>
          <AlertDialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white">
            <AlertDialogHeader>
              <AlertDialogTitle className="text-white">Delete Profile?</AlertDialogTitle>
              <AlertDialogDescription className="text-[#a1a1a6]">
                Are you sure you want to delete profile "{profileToDelete?.name}"? This action
                cannot be undone and may affect active streams.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel className="border-[#3a3a3c] bg-transparent text-white hover:bg-[#2c2c2e]">
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={handleDeleteConfirm}
                className="bg-[#dc2626] text-white hover:bg-[#ef4444]"
              >
                Delete
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
  );
}

function ConfigSection({
  title,
  icon,
  active,
  token,
  details,
}: {
  title: string;
  icon: React.ReactNode;
  active: boolean;
  token?: string;
  details?: string;
}) {
  return (
    <div
      className={`rounded-[8px] border p-[12px] ${active ? 'border-[#3a3a3c] bg-[#1c1c1e]' : 'border-[#3a3a3c]/50 bg-[#1c1c1e]/50 opacity-50'}`}
    >
      <div className="mb-[8px] flex items-center gap-[8px]">
        {icon}
        <span className="text-[13px] font-medium text-white">{title}</span>
      </div>
      {active ? (
        <div className="space-y-[2px]">
          {details && <div className="truncate text-[12px] text-[#a1a1a6]">{details}</div>}
          <div className="truncate font-mono text-[10px] text-[#636366]">{token}</div>
        </div>
      ) : (
        <div className="text-[12px] text-[#636366] italic">Not configured</div>
      )}
      <Button
        variant="link"
        className={`mt-[8px] h-auto p-0 text-[11px] ${active ? 'text-[#0a84ff]' : 'text-[#a1a1a6]'}`}
        disabled // Disabled for now as editing is complex
      >
        {active ? 'Edit (Coming Soon)' : 'Add (Coming Soon)'}
      </Button>
    </div>
  );
}
