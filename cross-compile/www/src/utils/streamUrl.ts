/**
 * HTTP-FLV stream URL construction.
 *
 * The WebUI is served from port 80 but the camera's HTTP-FLV server listens on
 * 8080, so production URLs must be absolute and cross-port. The camera sends
 * `Access-Control-Allow-Origin: *`, which is what makes that legal from the
 * browser. In development the Vite `/live` proxy points at 8080 for us, so a
 * relative path keeps the request same-origin and avoids a CORS preflight.
 */
export type StreamType = 'main' | 'sub';

export function buildFlvUrl(
  streamType: StreamType,
  { isDev = import.meta.env.DEV, hostname = window.location.hostname } = {},
): string {
  // 8080 is config.toml's media.httpflv_port.
  return isDev ? `/live/${streamType}.flv` : `http://${hostname}:8080/live/${streamType}.flv`;
}
