import { useEffect, useRef, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import type { Camera, MotionEvent, RecordingSegment } from '../types'
import { api } from '../api/client'
import { Timeline } from '../components/Timeline'
import styles from './Playback.module.css'

const WINDOWS: { label: string; hrs: number }[] = [
  { label: '1h',  hrs: 1       },
  { label: '6h',  hrs: 6       },
  { label: '24h', hrs: 24      },
  { label: '7d',  hrs: 24 * 7  },
]

function toLocalInput(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return (
    `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}` +
    `T${p(d.getHours())}:${p(d.getMinutes())}`
  )
}

export function Playback() {
  const { cameraId } = useParams<{ cameraId: string }>()
  const navigate     = useNavigate()

  const [camera,     setCamera]     = useState<Camera | null>(null)
  const [segments,   setSegments]   = useState<RecordingSegment[]>([])
  const [events,     setEvents]     = useState<MotionEvent[]>([])
  const [windowHrs,  setWindowHrs]  = useState(24)
  const [activeIdx,  setActiveIdx]  = useState<number | null>(null)
  const [cursor,     setCursor]     = useState<Date | null>(null)
  const [exportFrom, setExportFrom] = useState(() => toLocalInput(new Date(Date.now() - 3600_000)))
  const [exportTo,   setExportTo]   = useState(() => toLocalInput(new Date()))

  const videoRef     = useRef<HTMLVideoElement>(null)
  // Desired seek offset (seconds into segment) after the next loadedmetadata
  const pendingSeek  = useRef<number | null>(null)

  const now         = new Date()
  const windowEnd   = now
  const windowStart = new Date(now.getTime() - windowHrs * 3600_000)

  useEffect(() => {
    if (!cameraId) return
    api.cameras().then((list) => {
      setCamera(list.find((c) => c.id === cameraId) ?? null)
    })
  }, [cameraId])

  useEffect(() => {
    if (!cameraId) return
    api.recordings(cameraId, windowStart, windowEnd).then(setSegments)
    api.events(cameraId, windowStart, windowEnd).then(setEvents)
  // windowStart/End are derived from windowHrs — only re-fetch when window size changes
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cameraId, windowHrs])

  // Load the active segment into the video element
  useEffect(() => {
    const v = videoRef.current
    if (activeIdx === null || !v || !cameraId) return
    const seg = segments[activeIdx]
    if (!seg) return
    v.src = api.segmentUrl(cameraId, seg.id)
    // onLoadedMetadata handles seek + play
  }, [activeIdx, cameraId, segments])

  const handleLoadedMetadata = () => {
    const v = videoRef.current
    if (!v) return
    if (pendingSeek.current !== null) {
      v.currentTime = pendingSeek.current
      pendingSeek.current = null
    }
    v.play().catch(() => {})
  }

  // Auto-advance to the next segment when playback ends
  const handleEnded = () => {
    setActiveIdx((prev) => {
      if (prev === null || prev + 1 >= segments.length) return prev
      return prev + 1
    })
  }

  const handleTimeUpdate = () => {
    const v = videoRef.current
    if (!v || activeIdx === null) return
    const seg = segments[activeIdx]
    if (!seg) return
    setCursor(new Date(new Date(seg.started_at).getTime() + v.currentTime * 1000))
  }

  const handleSeek = (t: Date) => {
    const idx = segments.findIndex(
      (s) => new Date(s.started_at) <= t && t < new Date(s.ended_at),
    )
    if (idx < 0) return
    const offset = (t.getTime() - new Date(segments[idx].started_at).getTime()) / 1000
    setCursor(t)
    if (idx === activeIdx) {
      // Same segment — seek directly without reloading
      const v = videoRef.current
      if (v) v.currentTime = offset
    } else {
      pendingSeek.current = offset
      setActiveIdx(idx)
    }
  }

  const activeSeg  = activeIdx !== null ? segments[activeIdx] : null
  const exportHref = cameraId
    ? api.exportUrl(cameraId, new Date(exportFrom), new Date(exportTo))
    : '#'

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <button className={styles.backBtn} onClick={() => navigate('/')}>← Back</button>
        <span className={styles.title}>{camera?.name ?? 'Loading…'}</span>
        {cursor && (
          <span className={styles.currentTime}>{cursor.toLocaleString()}</span>
        )}
      </div>

      <div className={styles.player}>
        {activeSeg ? (
          <video
            ref={videoRef}
            className={styles.video}
            controls
            onLoadedMetadata={handleLoadedMetadata}
            onEnded={handleEnded}
            onTimeUpdate={handleTimeUpdate}
          />
        ) : (
          <p className={styles.noSignal}>
            {segments.length === 0
              ? 'No recordings in this time window'
              : 'Click the timeline to start playback'}
          </p>
        )}
      </div>

      <div className={styles.controls}>
        <div className={styles.timelineRow}>
          <select
            className={styles.windowSelect}
            value={windowHrs}
            onChange={(e) => { setActiveIdx(null); setEvents([]); setWindowHrs(Number(e.target.value)) }}
          >
            {WINDOWS.map(({ label, hrs }) => (
              <option key={label} value={hrs}>{label}</option>
            ))}
          </select>
          <Timeline
            segments={segments}
            events={events}
            windowStart={windowStart}
            windowEnd={windowEnd}
            cursor={cursor}
            onSeek={handleSeek}
          />
        </div>

        <div className={styles.exportRow}>
          <label>Export clip:</label>
          <input
            type="datetime-local"
            value={exportFrom}
            onChange={(e) => setExportFrom(e.target.value)}
          />
          <span>→</span>
          <input
            type="datetime-local"
            value={exportTo}
            onChange={(e) => setExportTo(e.target.value)}
          />
          <a className={styles.exportBtn} href={exportHref} download="clip.mp4">
            Download MP4
          </a>
        </div>
      </div>
    </div>
  )
}
