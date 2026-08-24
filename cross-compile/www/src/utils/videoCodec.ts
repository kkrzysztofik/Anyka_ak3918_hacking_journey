/**
 * Turn an MP4 codec tag like `mp4a.40.2` into a human label like `AAC-LC`.
 * The trailing number is the MPEG-4 Audio Object Type (ISO/IEC 14496-3); the
 * camera only ever emits 2, so anything else stays visible as its raw type.
 */
export function formatAudioCodec(codec?: string): string | undefined {
  if (!codec) return undefined;
  const match = codec.match(/^mp4a\.40\.(\d+)$/i);
  if (!match) return codec;
  const objectType = Number.parseInt(match[1], 10);
  return objectType === 2 ? 'AAC-LC' : `AAC type ${objectType}`;
}

/** H.264 profile names keyed by `profile_idc`. */
const H264_PROFILE_NAMES: Record<number, string> = {
  66: 'Baseline',
  77: 'Main',
  88: 'Extended',
  100: 'High',
  110: 'High10',
  122: 'High422',
  244: 'High444',
};

/**
 * Turn an MP4 codec tag like `avc1.4de028` into a human label like
 * `H.264 Main@L4.0`. The suffix is the profile-level-id: profile_idc,
 * constraint flags, level_idc (level in tenths).
 */
export function formatVideoCodec(codec?: string): string | undefined {
  if (!codec) return undefined;
  const match = /^avc1\.([0-9a-f]{6})$/i.exec(codec);
  if (!match) return codec;
  const profileIdc = Number.parseInt(match[1].slice(0, 2), 16);
  const constraintFlags = Number.parseInt(match[1].slice(2, 4), 16);
  const levelIdc = Number.parseInt(match[1].slice(4, 6), 16);
  const profile = H264_PROFILE_NAMES[profileIdc] ?? `Profile ${profileIdc}`;
  // H.264: level_idc 11 is "Level 1b" for Baseline/Main/Extended with
  // constraint_set3_flag set (0x10) — label it L1b, not L1.1.
  const level =
    levelIdc === 11 &&
    (constraintFlags & 0x10) !== 0 &&
    (profileIdc === 66 || profileIdc === 77 || profileIdc === 88)
      ? 'L1b'
      : `L${(levelIdc / 10).toFixed(1)}`;
  return `H.264 ${profile}@${level}`;
}
