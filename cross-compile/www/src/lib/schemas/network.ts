/**
 * Network Settings Zod Schema
 */
import { z } from 'zod';

const ipv4Regex =
  /^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;

export const networkSchema = z
  .object({
    dhcp: z.boolean(),
    address: z
      .string()
      .refine((val) => !val || ipv4Regex.test(val), 'Invalid IP address')
      .optional(),
    prefixLength: z.coerce.number().min(1).max(32).optional(),
    gateway: z
      .string()
      .refine((val) => !val || ipv4Regex.test(val), 'Invalid gateway address')
      .optional(),
    dnsFromDHCP: z.boolean(),
    dns1: z
      .string()
      .refine((val) => !val || ipv4Regex.test(val), 'Invalid DNS address')
      .optional(),
    dns2: z
      .string()
      .refine((val) => !val || ipv4Regex.test(val), 'Invalid DNS address')
      .optional(),
  })
  .refine((data) => data.dhcp || (data.address && data.prefixLength), {
    message: 'IP address and prefix are required when DHCP is disabled',
    path: ['address'],
  });

export type NetworkFormData = z.infer<typeof networkSchema>;
