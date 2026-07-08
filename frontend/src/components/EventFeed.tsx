import type { Camera, MotionEventMessage } from '../types'
import styles from './EventFeed.module.css'

interface Props {
  events:  MotionEventMessage[]
  cameras: Camera[]
}

export function EventFeed({ events, cameras }: Props) {
  const nameOf = (id: string) =>
    cameras.find((c) => c.id === id)?.name ?? id.slice(0, 8)

  const fmtTime = (iso: string) => {
    const d = new Date(iso)
    return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  }

  return (
    <div className={styles.panel}>
      <div className={styles.header}>Motion events</div>
      <div className={styles.list}>
        {events.length === 0 ? (
          <p className={styles.empty}>No motion detected yet</p>
        ) : (
          events.map((ev, i) => (
            <div key={`${ev.camera_id}-${ev.occurred_at}-${i}`} className={styles.item}>
              <span className={styles.dot} />
              <div className={styles.body}>
                <span className={styles.camName}>{nameOf(ev.camera_id)}</span>
                <span className={styles.meta}>
                  {fmtTime(ev.occurred_at)} · {ev.source}
                  {ev.score != null ? ` · ${(ev.score * 100).toFixed(1)}%` : ''}
                </span>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
