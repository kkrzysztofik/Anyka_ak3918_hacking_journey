/**
 * OSD settings page — camera name, date/time overlay, device-wide appearance.
 */
import { type CSSProperties, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Calendar, Save, Type } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
import { Slider } from '@/components/ui/slider';
import { Switch } from '@/components/ui/switch';
import {
  type OsdCorner,
  type OsdDateFormat,
  type OsdSettings,
  type OsdTimeFormat,
  assertAsciiOsdText,
  getOsdSettings,
  paletteCss,
  setOsd,
  setOsdEnabled,
} from '@/services/osdService';

const CORNERS: { value: OsdCorner; label: string }[] = [
  { value: 'UpperLeft', label: 'Upper left' },
  { value: 'UpperRight', label: 'Upper right' },
  { value: 'LowerLeft', label: 'Lower left' },
  { value: 'LowerRight', label: 'Lower right' },
];

const DATE_FORMATS: { value: OsdDateFormat; label: string }[] = [
  { value: 'yyyy-MM-dd', label: 'YYYY-MM-DD' },
  { value: 'dd/MM/yyyy', label: 'DD/MM/YYYY' },
  { value: 'MM/dd/yyyy', label: 'MM/DD/YYYY' },
];

const TIME_FORMATS: { value: OsdTimeFormat; label: string }[] = [
  { value: 'HH:mm:ss', label: '24-hour' },
  { value: 'hh:mm:ss tt', label: '12-hour' },
];

/** Swatches showing the real vendor palette, converted YCbCr → RGB. */
function paletteSwatchStyle(index: number): CSSProperties {
  return { backgroundColor: paletteCss(index) };
}

export default function OsdPage() {
  const { data, isLoading, isError } = useQuery<OsdSettings>({
    queryKey: ['osdSettings'],
    queryFn: getOsdSettings,
  });

  if (isError) {
    return (
      <div className="text-red-500" data-testid="osd-error">
        Failed to load OSD settings
      </div>
    );
  }

  if (isLoading || !data) {
    return (
      <div className="flex h-64 items-center justify-center" data-testid="osd-loading">
        <div className="border-primary h-8 w-8 animate-spin rounded-full border-b-2" />
      </div>
    );
  }

  return (
    <OsdForm
      key={`${data.name.text}|${data.name.position}|${data.datetime.position}|${data.datetime.dateFormat}|${data.datetime.timeFormat}|${data.appearance.color}|${data.appearance.alpha}|${data.name.enabled}|${data.datetime.enabled}`}
      data={data}
    />
  );
}

function OsdForm({ data }: Readonly<{ data: OsdSettings }>) {
  const queryClient = useQueryClient();
  const [nameText, setNameText] = useState(data.name.text);
  const [namePosition, setNamePosition] = useState<OsdCorner>(data.name.position);
  const [datetimePosition, setDatetimePosition] = useState<OsdCorner>(data.datetime.position);
  const [dateFormat, setDateFormat] = useState<OsdDateFormat>(data.datetime.dateFormat);
  const [timeFormat, setTimeFormat] = useState<OsdTimeFormat>(data.datetime.timeFormat);
  const [color, setColor] = useState(data.appearance.color);
  const [alpha, setAlpha] = useState(data.appearance.alpha);
  const [nameEnabled, setNameEnabled] = useState(data.name.enabled);
  const [datetimeEnabled, setDatetimeEnabled] = useState(data.datetime.enabled);
  const [asciiError, setAsciiError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: async () => {
      assertAsciiOsdText(nameText);

      // Enable first: SetOSD on a disabled OSD is rejected, because a disabled
      // OSD does not exist as far as ONVIF is concerned.
      await setOsdEnabled(
        {
          token: data.name.token,
          videoSourceToken: data.name.videoSourceToken,
          position: namePosition,
        },
        nameEnabled,
      );
      if (nameEnabled) {
        await setOsd({
          token: data.name.token,
          videoSourceToken: data.name.videoSourceToken,
          position: namePosition,
          textType: 'Plain',
          plainText: nameText,
          color,
          alpha,
        });
      }

      await setOsdEnabled(
        {
          token: data.datetime.token,
          videoSourceToken: data.datetime.videoSourceToken,
          position: datetimePosition,
        },
        datetimeEnabled,
      );
      if (datetimeEnabled) {
        await setOsd({
          token: data.datetime.token,
          videoSourceToken: data.datetime.videoSourceToken,
          position: datetimePosition,
          textType: 'DateAndTime',
          dateFormat,
          timeFormat,
          color,
          alpha,
        });
      }
    },
    onSuccess: () => {
      toast.success('OSD settings saved');
      queryClient.invalidateQueries({ queryKey: ['osdSettings'] });
    },
    onError: (error) => {
      toast.error('Failed to save OSD settings', {
        description: error instanceof Error ? error.message : 'An error occurred',
      });
      queryClient.invalidateQueries({ queryKey: ['osdSettings'] });
    },
  });

  const onNameTextChange = (value: string) => {
    setNameText(value);
    try {
      assertAsciiOsdText(value);
      setAsciiError(null);
    } catch (e) {
      setAsciiError(e instanceof Error ? e.message : 'Invalid text');
    }
  };

  const onSave = () => {
    if (asciiError) {
      return;
    }
    mutation.mutate();
  };

  return (
    <div className="space-y-6" data-testid="osd-page">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight" data-testid="osd-title">
          On-screen display
        </h1>
        <p className="text-muted-foreground mt-1 text-sm">
          Burn camera name and timestamp into the encoded video.
        </p>
      </div>

      <SettingsCard data-testid="osd-name-card">
        <SettingsCardHeader>
          <SettingsCardTitle className="flex items-center gap-2">
            <Type className="h-4 w-4" />
            Camera name
          </SettingsCardTitle>
          <SettingsCardDescription>
            Empty text falls back to the device hostname. ASCII only — the camera font has no
            diacritics.
          </SettingsCardDescription>
        </SettingsCardHeader>
        <SettingsCardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="osd-name-enabled">Show camera name</Label>
            <Switch
              id="osd-name-enabled"
              data-testid="osd-name-enabled-switch"
              checked={nameEnabled}
              onCheckedChange={setNameEnabled}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="osd-name-text">Text</Label>
            <Input
              id="osd-name-text"
              data-testid="osd-name-text-input"
              value={nameText}
              onChange={(e) => onNameTextChange(e.target.value)}
              maxLength={64}
              placeholder="Device hostname"
            />
            {asciiError ? (
              <p className="text-destructive text-sm" data-testid="osd-name-ascii-error">
                {asciiError}
              </p>
            ) : null}
          </div>
          <div className="space-y-2">
            <Label>Corner</Label>
            <Select value={namePosition} onValueChange={(v) => setNamePosition(v as OsdCorner)}>
              <SelectTrigger data-testid="osd-name-position-select">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CORNERS.map((c) => (
                  <SelectItem key={c.value} value={c.value} data-testid={`osd-name-pos-${c.value}`}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </SettingsCardContent>
      </SettingsCard>

      <SettingsCard data-testid="osd-datetime-card">
        <SettingsCardHeader>
          <SettingsCardTitle className="flex items-center gap-2">
            <Calendar className="h-4 w-4" />
            Date &amp; time
          </SettingsCardTitle>
          <SettingsCardDescription>
            Live timestamp, updated once per second.
          </SettingsCardDescription>
        </SettingsCardHeader>
        <SettingsCardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="osd-datetime-enabled">Show timestamp</Label>
            <Switch
              id="osd-datetime-enabled"
              data-testid="osd-datetime-enabled-switch"
              checked={datetimeEnabled}
              onCheckedChange={setDatetimeEnabled}
            />
          </div>
          <div className="space-y-2">
            <Label>Corner</Label>
            <Select
              value={datetimePosition}
              onValueChange={(v) => setDatetimePosition(v as OsdCorner)}
            >
              <SelectTrigger data-testid="osd-datetime-position-select">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {CORNERS.map((c) => (
                  <SelectItem
                    key={c.value}
                    value={c.value}
                    data-testid={`osd-datetime-pos-${c.value}`}
                  >
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>Date format</Label>
              <Select value={dateFormat} onValueChange={(v) => setDateFormat(v as OsdDateFormat)}>
                <SelectTrigger data-testid="osd-date-format-select">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {DATE_FORMATS.map((f) => (
                    <SelectItem key={f.value} value={f.value}>
                      {f.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>Time format</Label>
              <Select value={timeFormat} onValueChange={(v) => setTimeFormat(v as OsdTimeFormat)}>
                <SelectTrigger data-testid="osd-time-format-select">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {TIME_FORMATS.map((f) => (
                    <SelectItem key={f.value} value={f.value}>
                      {f.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        </SettingsCardContent>
      </SettingsCard>

      <SettingsCard data-testid="osd-appearance-card">
        <SettingsCardHeader>
          <SettingsCardTitle>Appearance (device-wide)</SettingsCardTitle>
          <SettingsCardDescription>
            Colour and opacity apply to every OSD on this camera — the vendor API has no per-rect
            colour.
          </SettingsCardDescription>
        </SettingsCardHeader>
        <SettingsCardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Palette</Label>
            <div className="flex flex-wrap gap-2" data-testid="osd-palette">
              {Array.from({ length: 16 }, (_, i) => (
                <button
                  key={i}
                  type="button"
                  data-testid={`osd-palette-${i}`}
                  aria-label={`Colour ${i}`}
                  aria-pressed={color === i}
                  className={`h-8 w-8 rounded border ${color === i ? 'ring-primary ring-2 ring-offset-2' : ''}`}
                  style={paletteSwatchStyle(i)}
                  onClick={() => setColor(i)}
                />
              ))}
            </div>
          </div>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Opacity</Label>
              <span
                className="text-muted-foreground font-mono text-sm"
                data-testid="osd-alpha-value"
              >
                {alpha}%
              </span>
            </div>
            <Slider
              data-testid="osd-alpha-slider"
              min={1}
              max={100}
              step={1}
              value={[alpha]}
              onValueChange={(v) => setAlpha(v[0] ?? 80)}
            />
          </div>
        </SettingsCardContent>
      </SettingsCard>

      <div className="flex justify-end">
        <Button
          data-testid="osd-save-button"
          onClick={onSave}
          disabled={mutation.isPending || !!asciiError}
        >
          <Save className="mr-2 h-4 w-4" />
          {mutation.isPending ? 'Saving…' : 'Save'}
        </Button>
      </div>
    </div>
  );
}
