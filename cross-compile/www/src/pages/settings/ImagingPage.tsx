/**
 * Imaging Settings Page
 *
 * Adjust brightness, contrast, saturation, and sharpness.
 */

import React from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Image, Save, Loader2, RotateCcw } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Slider } from '@/components/ui/slider'
import { Label } from '@/components/ui/label'
import { getImagingSettings, setImagingSettings, type ImagingSettings } from '@/services/imagingService'

export default function ImagingPage() {
  const queryClient = useQueryClient()
  const [localSettings, setLocalSettings] = React.useState<ImagingSettings | null>(null)

  // Fetch imaging settings
  const { data, isLoading, error } = useQuery<ImagingSettings>({
    queryKey: ['imagingSettings'],
    queryFn: () => getImagingSettings(),
  })

  // Update local state when data loads
  React.useEffect(() => {
    if (data) {
      setLocalSettings(data)
    }
  }, [data])

  // Mutation for saving settings
  const mutation = useMutation({
    mutationFn: (settings: ImagingSettings) => setImagingSettings(settings),
    onSuccess: () => {
      toast.success('Imaging settings saved')
      queryClient.invalidateQueries({ queryKey: ['imagingSettings'] })
    },
    onError: (error) => {
      toast.error('Failed to save imaging settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      })
    },
  })

  const handleSliderChange = (key: keyof ImagingSettings, value: number[]) => {
    if (!localSettings) return
    setLocalSettings({ ...localSettings, [key]: value[0] })
  }

  const handleSave = () => {
    if (!localSettings) return
    mutation.mutate(localSettings)
  }

  const handleReset = () => {
    if (!data) return
    setLocalSettings(data)
  }

  const hasChanges = localSettings && data && (
    localSettings.brightness !== data.brightness ||
    localSettings.contrast !== data.contrast ||
    localSettings.saturation !== data.saturation ||
    localSettings.sharpness !== data.sharpness
  )

  if (error) {
    return (
      <div className="space-y-6">
        <h1 className="text-2xl font-bold">Imaging</h1>
        <Card>
          <CardContent className="pt-6">
            <p className="text-destructive">Failed to load imaging settings</p>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Imaging</h1>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Image className="w-5 h-5" />
            Image Settings
          </CardTitle>
          <CardDescription>
            Adjust brightness, contrast, saturation, and sharpness
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading || !localSettings ? (
            <div className="flex items-center gap-2 text-muted-foreground py-4">
              <Loader2 className="w-4 h-4 animate-spin" />
              Loading imaging settings...
            </div>
          ) : (
            <div className="space-y-8">
              {/* Brightness */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label>Brightness</Label>
                  <span className="text-sm text-muted-foreground w-12 text-right">
                    {localSettings.brightness}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.brightness]}
                  onValueChange={(v) => handleSliderChange('brightness', v)}
                  min={0}
                  max={100}
                  step={1}
                />
              </div>

              {/* Contrast */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label>Contrast</Label>
                  <span className="text-sm text-muted-foreground w-12 text-right">
                    {localSettings.contrast}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.contrast]}
                  onValueChange={(v) => handleSliderChange('contrast', v)}
                  min={0}
                  max={100}
                  step={1}
                />
              </div>

              {/* Saturation */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label>Saturation</Label>
                  <span className="text-sm text-muted-foreground w-12 text-right">
                    {localSettings.saturation}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.saturation]}
                  onValueChange={(v) => handleSliderChange('saturation', v)}
                  min={0}
                  max={100}
                  step={1}
                />
              </div>

              {/* Sharpness */}
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label>Sharpness</Label>
                  <span className="text-sm text-muted-foreground w-12 text-right">
                    {localSettings.sharpness}%
                  </span>
                </div>
                <Slider
                  value={[localSettings.sharpness]}
                  onValueChange={(v) => handleSliderChange('sharpness', v)}
                  min={0}
                  max={100}
                  step={1}
                />
              </div>

              {/* Actions */}
              <div className="flex justify-end gap-2 pt-4">
                <Button
                  variant="outline"
                  onClick={handleReset}
                  disabled={!hasChanges}
                >
                  <RotateCcw className="mr-2 h-4 w-4" />
                  Reset
                </Button>
                <Button
                  onClick={handleSave}
                  disabled={mutation.isPending || !hasChanges}
                >
                  {mutation.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    <>
                      <Save className="mr-2 h-4 w-4" />
                      Save Changes
                    </>
                  )}
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
