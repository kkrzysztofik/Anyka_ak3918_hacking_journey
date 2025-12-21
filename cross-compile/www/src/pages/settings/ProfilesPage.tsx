/**
 * Profiles Page
 *
 * Manage media profiles and their configurations (Video/Audio Sources, Encoders, PTZ, Analytics, Metadata).
 */
import React, { useEffect, useState } from 'react';

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
import { Label } from '@/components/ui/label';
import {
  type MediaProfile,
  type VideoEncoderConfiguration,
  type VideoEncoderConfigurationOptions,
  createProfile,
  deleteProfile,
  getProfiles,
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
  setVideoEncoderConfiguration,
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
  const [editingEncoder, setEditingEncoder] = useState<{
    profileToken: string;
    encoderToken: string;
  } | null>(null);

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
                        onEdit={
                          profile.videoEncoderConfiguration
                            ? () =>
                                setEditingEncoder({
                                  profileToken: profile.token,
                                  encoderToken: profile.videoEncoderConfiguration!.token,
                                })
                            : undefined
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

        {/* Video Encoder Edit Dialog */}
        {editingEncoder && (
          <VideoEncoderEditDialog
            encoderToken={editingEncoder.encoderToken}
            onClose={() => setEditingEncoder(null)}
          />
        )}
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
  onEdit,
}: {
  title: string;
  icon: React.ReactNode;
  active: boolean;
  token?: string;
  details?: string;
  onEdit?: () => void;
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
        onClick={onEdit}
        disabled={!onEdit}
      >
        {active ? 'Edit' : 'Add (Coming Soon)'}
      </Button>
    </div>
  );
}

function VideoEncoderEditDialog({
  encoderToken,
  onClose,
}: {
  encoderToken: string;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const [config, setConfig] = useState<VideoEncoderConfiguration | null>(null);
  const [options, setOptions] = useState<VideoEncoderConfigurationOptions | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // Fetch encoder configuration and options
  useEffect(() => {
    const loadData = async () => {
      try {
        const [encoderConfig, encoderOptions] = await Promise.all([
          getVideoEncoderConfiguration(encoderToken),
          getVideoEncoderConfigurationOptions(encoderToken),
        ]);
        if (encoderConfig) {
          setConfig(encoderConfig);
        }
        setOptions(encoderOptions);
      } catch (error) {
        toast.error('Failed to load encoder configuration', {
          description: error instanceof Error ? error.message : 'An error occurred',
        });
        onClose();
      } finally {
        setIsLoading(false);
      }
    };
    loadData();
  }, [encoderToken, onClose]);

  const updateMutation = useMutation({
    mutationFn: (updatedConfig: VideoEncoderConfiguration) =>
      setVideoEncoderConfiguration(updatedConfig, false),
    onSuccess: () => {
      toast.success('Video encoder configuration updated');
      queryClient.invalidateQueries({ queryKey: ['profiles'] });
      onClose();
    },
    onError: (error) => {
      toast.error('Failed to update encoder configuration', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
    },
  });

  const handleSave = () => {
    if (config) {
      updateMutation.mutate(config);
    }
  };

  if (isLoading || !config || !options) {
    return (
      <Dialog open onOpenChange={onClose}>
        <DialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[600px]">
          <div className="py-8 text-center text-[#a1a1a6]">Loading...</div>
        </DialogContent>
      </Dialog>
    );
  }

  const h264Options = options.h264;
  const availableResolutions = h264Options?.resolutionsAvailable || [];

  return (
    <Dialog open onOpenChange={onClose}>
      <DialogContent className="border-[#3a3a3c] bg-[#1c1c1e] text-white sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle className="text-white">Edit Video Encoder Configuration</DialogTitle>
          <DialogDescription className="text-[#a1a1a6]">
            Configure video encoding settings for this profile
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* Resolution */}
          <div className="space-y-2">
            <Label className="text-[#e5e5e5]">Resolution</Label>
            <select
              value={`${config.resolution.width}x${config.resolution.height}`}
              onChange={(e) => {
                const [width, height] = e.target.value.split('x').map(Number);
                setConfig({ ...config, resolution: { width, height } });
              }}
              className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-[#0a84ff] focus:outline-none"
            >
              {availableResolutions.map((res) => (
                <option key={`${res.width}x${res.height}`} value={`${res.width}x${res.height}`}>
                  {res.width} × {res.height}
                </option>
              ))}
            </select>
          </div>

          {/* Quality */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label className="text-[#e5e5e5]">Quality</Label>
              <span className="text-sm text-[#a1a1a6] tabular-nums">{config.quality}</span>
            </div>
            <input
              type="range"
              min={options.qualityRange.min}
              max={options.qualityRange.max}
              value={config.quality}
              onChange={(e) => setConfig({ ...config, quality: Number(e.target.value) })}
              className="w-full"
            />
          </div>

          {/* Frame Rate */}
          {config.rateControl && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">Frame Rate Limit</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">
                  {config.rateControl.frameRateLimit} fps
                </span>
              </div>
              <input
                type="range"
                min={h264Options.frameRateRange.min}
                max={h264Options.frameRateRange.max}
                value={config.rateControl.frameRateLimit}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    rateControl: {
                      ...config.rateControl!,
                      frameRateLimit: Number(e.target.value),
                    },
                  })
                }
                className="w-full"
              />
            </div>
          )}

          {/* Bitrate */}
          {config.rateControl && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">Bitrate Limit</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">
                  {config.rateControl.bitrateLimit} kbps
                </span>
              </div>
              <input
                type="range"
                min={h264Options.bitrateRange.min}
                max={h264Options.bitrateRange.max}
                value={config.rateControl.bitrateLimit}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    rateControl: {
                      ...config.rateControl!,
                      bitrateLimit: Number(e.target.value),
                    },
                  })
                }
                className="w-full"
              />
            </div>
          )}

          {/* H.264 Profile */}
          {config.h264 && h264Options && (
            <div className="space-y-2">
              <Label className="text-[#e5e5e5]">H.264 Profile</Label>
              <select
                value={config.h264.h264Profile}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    h264: { ...config.h264!, h264Profile: e.target.value },
                  })
                }
                className="h-10 w-full appearance-none rounded-md border border-[#3a3a3c] bg-[#2c2c2e] px-3 py-2 text-sm text-white focus:border-[#0a84ff] focus:outline-none"
              >
                {h264Options.h264ProfilesSupported.map((profile) => (
                  <option key={profile} value={profile}>
                    {profile}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* GOP Length */}
          {config.h264 && h264Options && (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-[#e5e5e5]">GOP Length</Label>
                <span className="text-sm text-[#a1a1a6] tabular-nums">{config.h264.govLength}</span>
              </div>
              <input
                type="range"
                min={h264Options.govLengthRange.min}
                max={h264Options.govLengthRange.max}
                value={config.h264.govLength}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    h264: { ...config.h264!, govLength: Number(e.target.value) },
                  })
                }
                className="w-full"
              />
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            className="border-[#3a3a3c] text-white hover:bg-[#2c2c2e]"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSave}
            disabled={updateMutation.isPending}
            className="bg-[#0a84ff] text-white hover:bg-[#0077ed]"
          >
            {updateMutation.isPending ? 'Saving...' : 'Save Changes'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
