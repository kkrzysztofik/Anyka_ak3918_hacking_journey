/**
 * Timezone options for the Time page.
 *
 * Values are POSIX TZ strings — the exact format the camera's in-process
 * parser accepts (no zoneinfo database on the 36 MB rootfs). Bare
 * abbreviations like "CET" carry no DST rules and are an hour wrong in
 * spring/fall; every entry that observes DST spells the rules out.
 */
export const TIMEZONES = [
  { value: 'UTC0', label: 'UTC' },
  { value: 'GMT0BST,M3.5.0/1,M10.5.0', label: 'London (GMT/BST)' },
  { value: 'CET-1CEST,M3.5.0,M10.5.0/3', label: 'Warsaw / Berlin / Paris (CET/CEST)' },
  { value: 'EET-2EEST,M3.5.0/3,M10.5.0/4', label: 'Helsinki / Athens (EET/EEST)' },
  { value: 'EST5EDT,M3.2.0,M11.1.0', label: 'New York (EST/EDT)' },
  { value: 'PST8PDT,M3.2.0,M11.1.0', label: 'Los Angeles (PST/PDT)' },
  { value: 'CST-8', label: 'China (CST)' },
  { value: 'JST-9', label: 'Japan (JST)' },
];
