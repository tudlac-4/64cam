# Platform Limitations

## Linux (Production Target)

64cam nodes are designed and tested for Linux. All features work as documented.

| Feature | Support |
|---------|---------|
| RTSP ingest | Full |
| MediaMTX recording | Full |
| ONVIF WS-Discovery (UDP multicast) | Full with `network_mode: host` |
| ONVIF manual entry (known IP) | Full |
| Hardware acceleration (VAAPI, NVENC) | Full (detected at startup) |
| Keyframe-diff motion detection | Full |
| WebRTC WHEP | Full |
| S3 segment migration | Full |
| Host networking | Full |

---

## Windows Docker Desktop (Development Only)

Docker Desktop on Windows runs containers inside a lightweight Linux VM
(WSL 2 or Hyper-V). This creates several limitations for NVR workloads:

### Host Networking Not Available

`network_mode: host` is silently ignored on Windows Docker Desktop — the
container does NOT share the Windows host's network interfaces. Consequences:

- **ONVIF WS-Discovery** (UDP multicast to 239.255.255.250:3702) does not work.
  Cameras that support only auto-discovery will not be found.
  **Workaround:** Enter camera RTSP/ONVIF URLs manually when creating cameras
  via the API or UI.

- **Direct RTSP access** to cameras on the same LAN may fail if cameras are
  not reachable from within Docker's NAT network.
  **Workaround:** Use explicit IP addresses; ensure cameras are reachable from
  the Docker bridge network (usually `172.x.x.x`).

### Hardware Acceleration Unavailable

- **Intel VAAPI / Quick Sync:** Not available inside Docker Desktop. The Linux
  DRM device (`/dev/dri`) is not passed through from the Windows host.
- **NVIDIA NVENC:** Requires the NVIDIA Container Toolkit, which is available
  on Windows only via WSL 2 with NVIDIA GPU driver ≥ 515. Even then, passthrough
  reliability varies. The node detects absence of acceleration gracefully and
  falls back to software decode for keyframe-diff motion detection.
- **AMD ROCm:** Not supported through Docker Desktop on Windows.

Consequence: keyframe-diff motion detection is disabled by default when
`cpu_cores < 2 && hw_accel.is_empty()`. On a Windows dev machine with a
multi-core host CPU, this condition is false and diff runs in software (fine
for development; not recommended for 64-camera production load).

### Volume I/O Performance

Docker Desktop mounts volumes via a FUSE filesystem bridge between the Windows
host and the Linux VM. For NVR workloads writing dozens of continuous fMP4
streams, this introduces latency and throughput constraints:

- Expect 2–10× worse write throughput vs. native Linux
- Large files (clip exports) may be noticeably slower to open
- The disk watchdog may trigger more aggressively due to virtual disk overhead

**Workaround:** Use named Docker volumes (already the default in all compose
files) rather than bind-mount paths. Named volumes reside inside the VM's
virtual disk and avoid the FUSE overhead.

### Summary for Windows

Windows Docker Desktop is suitable for:
- Running the coordinator and reviewing the UI
- Testing the API and WebSocket flows
- Local development with 1–2 cameras using manual RTSP URL entry

Windows Docker Desktop is **not** suitable for:
- Production CCTV/NVR operation
- Hardware-accelerated transcoding
- ONVIF network discovery
- High-throughput multi-camera recording

---

## macOS Docker Desktop (Development Only)

The same limitations as Windows apply, with one addition:

### `/dev/video*` and Capture Devices

macOS has no `/dev/video*` device interface. Local USB cameras cannot be
passed through to Docker containers. For development, use test RTSP streams
(e.g., FFmpeg serving a video file as RTSP, or a software IP camera app on a
phone).

### ARM Macs (Apple Silicon)

Docker Desktop on Apple Silicon emulates `linux/arm64` natively and
`linux/amd64` via Rosetta. The 64cam arm64 images run natively and perform
well for development purposes.

Host networking is still unavailable — the same ONVIF/RTSP LAN access
limitations apply as on Windows.

---

## ONVIF Discovery Details

ONVIF WS-Discovery uses UDP multicast. For it to work:

1. The node container **must** use `network_mode: host` (Linux only)
2. The node and cameras must be on the **same Layer 2 network segment** (no
   routing across VLANs without multicast routing enabled)
3. Some cameras use unicast `Hello` instead of multicast; these are discovered
   by waiting for them to announce themselves

If auto-discovery does not work, cameras can always be added manually by
specifying their ONVIF URL when creating a camera:

```bash
curl -X POST /api/v1/cameras \
  -d '{
    "name": "front-door",
    "rtsp_url": "rtsp://192.168.1.50/stream1",
    "onvif_url": "http://192.168.1.50/onvif/device_service",
    "onvif_username": "admin",
    "onvif_password": "password"
  }'
```

---

## Hardware Acceleration Support Matrix

| Hardware | Linux | Windows Docker | macOS Docker |
|----------|-------|---------------|--------------|
| Intel VAAPI (iGPU) | Supported | No | No |
| Intel Quick Sync | Supported | No | No |
| NVIDIA NVENC | Supported (NVIDIA Container Toolkit) | Limited (WSL2) | No |
| AMD VAAPI | Supported | No | No |
| Raspberry Pi VideoCore | Not yet (libcamera/V4L2 in roadmap) | N/A | N/A |
| Apple Neural Engine | N/A | N/A | No |

Hardware acceleration is used for:
- Keyframe-diff motion detection (FFmpeg decode of RTSP stream at 2fps)
- Future: transcoding for WHEP WebRTC delivery

The node reports detected acceleration in the heartbeat payload and the
coordinator displays it in node metadata.
