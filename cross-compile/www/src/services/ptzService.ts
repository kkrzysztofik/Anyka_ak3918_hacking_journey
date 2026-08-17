/**
 * PTZ Service
 *
 * SOAP operations for PTZ control (continuous move, stop, home, presets).
 */
import { ENDPOINTS } from '@/services/api';
import { soapBodies, soapRequest } from '@/services/soap/client';
import { safeString } from '@/utils/safeString';

export interface PTZPreset {
  token: string;
  name: string;
}

/** Normalised ONVIF PTZ position (-1..1 for pan/tilt, 0..1 for zoom). */
export interface PTZStatus {
  pan: number;
  tilt: number;
  zoom: number;
}

/** Valid PTZ movement directions */
export type PTZDirection =
  'up' | 'down' | 'left' | 'right' | 'up-left' | 'up-right' | 'down-left' | 'down-right';

/** Direction-to-velocity unit vector mapping */
const DIRECTION_VELOCITIES: Record<PTZDirection, { pan: number; tilt: number }> = {
  up: { pan: 0, tilt: 1 },
  down: { pan: 0, tilt: -1 },
  left: { pan: -1, tilt: 0 },
  right: { pan: 1, tilt: 0 },
  'up-left': { pan: -0.7, tilt: 0.7 },
  'up-right': { pan: 0.7, tilt: 0.7 },
  'down-left': { pan: -0.7, tilt: -0.7 },
  'down-right': { pan: 0.7, tilt: -0.7 },
};

/**
 * Start continuous PTZ movement in the given direction at the given speed.
 *
 * @param profileToken - ONVIF media profile token
 * @param direction - One of: up, down, left, right, up-left, up-right, down-left, down-right
 * @param speedPercent - Speed as 0–100 percentage (mapped to ONVIF 0–1 velocity)
 */
export async function continuousMove(
  profileToken: string,
  direction: PTZDirection,
  speedPercent: number,
): Promise<void> {
  const vel = DIRECTION_VELOCITIES[direction];
  if (!vel) throw new Error(`Unknown PTZ direction: ${direction}`);
  const clamped = Number.isFinite(speedPercent) ? speedPercent : 0;
  const speed = Math.max(0, Math.min(100, clamped)) / 100;
  await soapRequest(
    ENDPOINTS.ptz,
    soapBodies.continuousMove(profileToken, vel.pan * speed, vel.tilt * speed),
  );
}

/**
 * Stop all PTZ movement (pan, tilt, and zoom).
 */
export async function stopMove(profileToken: string): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.ptzStop(profileToken));
}

/**
 * Move to the home position.
 */
export async function gotoHome(profileToken: string): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.gotoHomePosition(profileToken));
}

/** Read one XML axis attribute, treating anything unparseable as 0. */
function axis(value: unknown): number {
  const parsed = Number(safeString(value, ''));
  return Number.isFinite(parsed) ? parsed : 0;
}

/**
 * Get the current PTZ position.
 *
 * This is the *tracked* position, not a hardware reading: the V500 motor
 * ioctls accept a position write but never report one back, so the device
 * dead-reckons it from the moves it has been commanded to make. It drifts on
 * stalls and is wrong after a manual nudge. Anything that shows this to a
 * user must say where the number comes from.
 */
export async function getPtzStatus(profileToken: string): Promise<PTZStatus> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.ptz,
    soapBodies.getStatus(profileToken),
    'GetStatusResponse',
  );

  const status = data?.PTZStatus as Record<string, unknown> | undefined;
  const position = status?.Position as Record<string, unknown> | undefined;
  const panTilt = position?.PanTilt as Record<string, unknown> | undefined;
  const zoom = position?.Zoom as Record<string, unknown> | undefined;

  return {
    pan: axis(panTilt?.['@_x']),
    tilt: axis(panTilt?.['@_y']),
    zoom: axis(zoom?.['@_x']),
  };
}

/**
 * Get all saved presets for a profile.
 */
export async function getPresets(profileToken: string): Promise<PTZPreset[]> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.ptz,
    soapBodies.getPresets(profileToken),
    'GetPresetsResponse',
  );

  const presets = data?.Preset;
  if (!presets) {
    return [];
  }

  const presetsList = Array.isArray(presets) ? presets : [presets];

  return presetsList.map((preset: Record<string, unknown>) => ({
    token: safeString(preset['@_token'], ''),
    name: safeString(preset.Name, ''),
  }));
}

/**
 * Move to a saved preset position.
 */
export async function gotoPreset(profileToken: string, presetToken: string): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.gotoPreset(profileToken, presetToken));
}

/**
 * Save the current position as a preset (or update an existing one).
 *
 * @returns The preset token assigned by the device
 */
export async function setPreset(
  profileToken: string,
  name: string,
  presetToken?: string,
): Promise<string> {
  const data = await soapRequest<Record<string, unknown>>(
    ENDPOINTS.ptz,
    soapBodies.setPreset(profileToken, name, presetToken),
    'SetPresetResponse',
  );
  return safeString(data?.PresetToken, '');
}

/**
 * Remove a saved preset.
 */
export async function removePreset(profileToken: string, presetToken: string): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.removePreset(profileToken, presetToken));
}

/** Lamp control commands supported by this camera. */
export type AuxiliaryCommand =
  'tt:IRLamp|On' | 'tt:IRLamp|Off' | 'tt:IRLamp|Auto' | 'tt:WhiteLight|On' | 'tt:WhiteLight|Off';

/**
 * Send an ONVIF auxiliary command, used here for the IR lamp and white light.
 *
 * @param profileToken - ONVIF media profile token
 * @param command - One of the AuxiliaryCommand strings
 */
export async function sendAuxiliaryCommand(
  profileToken: string,
  command: AuxiliaryCommand,
): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.sendAuxiliaryCommand(profileToken, command));
}
