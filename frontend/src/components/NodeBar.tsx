import { useEffect, useState } from 'react'
import { api } from '../api/client'
import type { Node, NodeCapacity } from '../types'
import styles from './NodeBar.module.css'

interface Props {
  nodes:     Node[]
  onlineIds: Set<string>
}

export function NodeBar({ nodes, onlineIds }: Props) {
  const [capacity, setCapacity] = useState<Map<string, NodeCapacity>>(new Map())

  useEffect(() => {
    const approved = nodes.filter((n) => n.status === 'approved')
    Promise.allSettled(
      approved.map((n) =>
        api.nodeCapacity(n.id).then((c) => ({ id: n.id, cap: c })),
      ),
    ).then((results) => {
      setCapacity((prev) => {
        const next = new Map(prev)
        for (const r of results) {
          if (r.status === 'fulfilled') {
            next.set(r.value.id, r.value.cap)
          }
        }
        return next
      })
    })
  }, [nodes])

  if (nodes.length === 0) return null

  return (
    <div className={styles.bar}>
      {nodes.map((node) => {
        const online = onlineIds.has(node.id)
        const cap    = capacity.get(node.id)
        const pct    = cap ? Math.round((cap.camera_count / cap.max_cameras) * 100) : null
        const warn   = pct !== null && pct >= 75

        return (
          <div key={node.id} className={styles.node}>
            <span
              className={`${styles.dot} ${online ? styles.online : styles.offline}`}
              title={online ? 'online' : 'offline'}
            />
            <span className={styles.name} title={`id: ${node.id}`}>{node.name}</span>
            {cap && (
              <span className={styles.cap} title={`${cap.camera_count} / ${cap.max_cameras} cameras`}>
                <span className={styles.track}>
                  <span
                    className={`${styles.fill} ${warn ? styles.warn : ''}`}
                    style={{ width: `${pct}%` }}
                  />
                </span>
                <span className={styles.label}>{cap.camera_count}/{cap.max_cameras}</span>
              </span>
            )}
            {node.status !== 'approved' && (
              <span className={styles.badge}>{node.status}</span>
            )}
          </div>
        )
      })}
    </div>
  )
}
