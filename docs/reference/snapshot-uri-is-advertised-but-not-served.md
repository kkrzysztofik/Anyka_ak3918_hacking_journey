# `GetSnapshotUri` advertises a URI nothing serves

Found 2026-08-15 on `.198` running `ec1d98ac-dirty`, while looking for a way to measure frame
brightness. Not a regression from that build — the handler has never existed.

## Symptom

```console
$ curl -s -u <user>:<password> -D- -o /dev/null 'http://<cam>/snapshot.jpg'
HTTP/1.1 200 OK
content-type: text/html
```

`<user>:<password>` is the camera's HTTP credential — the same one the WebUI sends — stored
in the camera's credential mechanism, never hardcoded in docs or scripts.

The request falls through to the WebUI's SPA fallback and returns `index.html`.

## Cause

`get_snapshot_uri` (`onvif/media/ops/streaming.rs:86`) builds and returns

```text
{base_url}{media.snapshot_path}?profile={token}
```

defaulting to `DEFAULT_SNAPSHOT_PATH = "/snapshot.jpg"` (`onvif/media/types.rs:49`). That is the
whole implementation: **there is no HTTP route for that path anywhere in the codebase.** A grep
for `snapshot` across `src/` returns only this URI construction, unrelated state-`snapshot()`
methods, and tests.

## Why it is worse than a 404

An ONVIF client (NVR, ONVIF Device Manager, a home-automation integration) does
`GetSnapshotUri` and then `GET`s the result. It receives **HTTP 200 with `text/html`**, so it
cannot distinguish "not implemented" from "here is your image" without sniffing the body. A 404
would at least fail cleanly and let the client fall back to pulling a frame off RTSP.

## Fixing it

Two honest options:

1. **Serve it.** The vendor daemon already owns the encoder; a snapshot means grabbing a frame
   and JPEG-encoding it. The camera ships `jpeg_snapshot` and a `/mnt/anyka_hack/snapshot`
   directory from the vendor payload, so the capability exists on the device — but wiring it
   through the IPC and the HTTP server is real work, and it competes with the VI the daemon
   already holds.
2. **Stop advertising it.** Return an ONVIF fault from `GetSnapshotUri`, or make the route
   return 404 rather than falling through to the SPA. Cheap, honest, and lets clients fall back
   to RTSP immediately.

Option 2 is the right default until someone actually needs stills; option 1 is only worth it if
a client in use requires snapshots.

## Note for anyone measuring image brightness

Because there is no snapshot endpoint, the working method is RTSP plus ffmpeg:

```bash
ffmpeg -rtsp_transport tcp -i "rtsp://<user>:<password>@<cam>:554/main" \
  -vf "signalstats,metadata=print:key=lavfi.signalstats.YAVG" -frames:v 12 -f null -
```

Same credential rule as above: `<user>:<password>` is a placeholder for the camera's
credential, not a literal.

Also available live, and unregulated by the AE, are the AWB colour bins in
`/api/diagnostics` (`vision.awb_cnt`) — see `ir-illuminator-bench-measurement.md`.
