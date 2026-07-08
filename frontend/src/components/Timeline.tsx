import type { MotionEvent, RecordingSegment } from '../types'
import styles from './Timeline.module.css'

interface Props {
  segments:    RecordingSegment[]
  events:      MotionEvent[]
  windowStart: Date
  windowEnd:   Date
  cursor:      Date | null
  onSeek:      (t: Date) => void
}

export function Timeline({ segments, events, windowStart, windowEnd, cursor, onSeek }: Props) {
  const span = windowEnd.getTime() - windowStart.getTime()

  const toPercent = (d: Date) =>
    Math.max(0, Math.min(100, ((d.getTime() - windowStart.getTime()) / span) * 100))

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    const frac = (e.clientX - rect.left) / rect.width
    onSeek(new Date(windowStart.getTime() + frac * span))
  }

  const fmt = (d: Date) =>
    d.toLocaleString(undefined, {
      month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit',
    })

  return (
    <div className={styles.root} onClick={handleClick} title="Click to seek">
      <div className={styles.track}>
        {segments.map((seg) => {
          const left  = toPercent(new Date(seg.started_at))
          const right = toPercent(new Date(seg.ended_at))
          return (
            <div
              key={seg.id}
              className={styles.segment}
              style={{ left: `${left}%`, width: `${Math.max(right - left, 0.1)}%` }}
            />
          )
        })}

        {events.map((ev) => (
          <div
            key={ev.id}
            className={styles.motion}
            style={{ left: `${toPercent(new Date(ev.occurred_at))}%` }}
            title={`Motion (${ev.source})${ev.score != null ? ` score=${ev.score.toFixed(3)}` : ''}`}
          />
        ))}

        {cursor && (
          <div
            className={styles.cursor}
            style={{ left: `${toPercent(cursor)}%` }}
          />
        )}
      </div>
      <div className={styles.labels}>
        <span>{fmt(windowStart)}</span>
        <span>{fmt(windowEnd)}</span>
      </div>
    </div>
  )
}
