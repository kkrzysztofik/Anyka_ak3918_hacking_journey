/**
 * Identification Settings Zod Schema
 */

import { z } from 'zod'

export const identificationSchema = z.object({
  name: z.string().min(1, 'Device name is required').max(64, 'Name is too long'),
  location: z.string().max(128, 'Location is too long').optional().default(''),
})

export type IdentificationFormData = z.infer<typeof identificationSchema>
