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

export const identificationSchema = z.object({
  deviceInfo: deviceInfoSchema,
  name: z.string().min(1, 'Device name is required').max(64, 'Name is too long'),
  location: z.string().max(128, 'Location is too long'),
});

export type IdentificationFormData = z.infer<typeof identificationSchema>;
