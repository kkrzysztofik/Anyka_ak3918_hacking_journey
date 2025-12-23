/**
 * Network Settings Zod Schema
 */
import { z } from 'zod';

// Simplified IPv4 regex to reduce complexity
// Octet pattern: 0-255 (25[0-5] | 2[0-4][0-9] | 1[0-9][0-9] | [1-9][0-9] | [0-9])
const octet = String.raw`(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)`;
const ipv4Regex = new RegExp(String.raw`^${octet}\.${octet}\.${octet}\.${octet}$`);

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
