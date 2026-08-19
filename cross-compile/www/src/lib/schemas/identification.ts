/**
 * Identification Settings Zod Schema
 */
import { z } from 'zod';

const deviceInfoSchema = z.object({
  manufacturer: z.string(),
  model: z.string(),
  firmwareVersion: z.string(),
  serialNumber: z.string(),
  hardwareId: z.string(),
});

const scopeItemSchema = z
  .string()
  .min(1, 'Scope cannot be empty')
  .max(256, 'Scope is too long')
  .startsWith('onvif://www.onvif.org/', 'Scope must start with onvif://www.onvif.org/')
  .refine(
    (scope) => !/[\p{White_Space}\p{Cc}]/u.test(scope),
    'Scope cannot contain spaces or control characters',
  );

const scopeSchema = z.object({
  scopeDef: z.enum(['Fixed', 'Configurable']),
  scopeItem: scopeItemSchema,
});

export const identificationSchema = z.object({
  deviceInfo: deviceInfoSchema,
  name: z.string().min(1, 'Device name is required').max(64, 'Name is too long'),
  location: z.string().max(128, 'Location is too long'),
  hostname: z
    .string()
    .min(1, 'Hostname is required')
    .max(63, 'Hostname is too long')
    .regex(
      /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?$/,
      'Hostname must be a DNS label (letters, digits, hyphens)',
    )
    .default('ipcam'),
  discoveryMode: z.enum(['Discoverable', 'NonDiscoverable']).default('Discoverable'),
  scopes: z.array(scopeSchema).default([]),
});

export type IdentificationFormInput = z.input<typeof identificationSchema>;
export type IdentificationFormData = z.output<typeof identificationSchema>;
