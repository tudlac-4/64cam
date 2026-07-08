import { useEffect, useRef, useState } from 'react'
import { api } from '../api/client'
import type { CameraStatus, DashboardMessage, MotionEventMessage } from '../types'

export interface StatusMap {
  cameras:      Map<string, CameraStatus>
  nodeOnline:   Map<string, boolean>
  motionEvents: MotionEventMessage[]
}

const RECONNECT_MS  = 5_000
const MAX_EVENTS    = 100

export function useStatusWs(enabled: boolean): StatusMap {
  const [status, setStatus] = useState<StatusMap>({
    cameras:      new Map(),
    nodeOnline:   new Map(),
    motionEvents: [],
  })
  const wsRef    = useRef<WebSocket | null>(null)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!enabled) return

    let active = true

    const connect = () => {
      if (!active) return
      const ws = new WebSocket(api.dashboardWsUrl())
      wsRef.current = ws

      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data as string) as DashboardMessage
          setStatus((prev) => applyMessage(prev, msg))
        } catch {
          // ignore malformed frames
        }
      }

      ws.onclose = () => {
        if (active) timerRef.current = setTimeout(connect, RECONNECT_MS)
      }

      ws.onerror = () => ws.close()
    }

    connect()

    return () => {
      active = false
      if (timerRef.current) clearTimeout(timerRef.current)
      wsRef.current?.close()
    }
  }, [enabled])

  return status
}

function applyMessage(prev: StatusMap, msg: DashboardMessage): StatusMap {
  const cameras      = new Map(prev.cameras)
  const nodeOnline   = new Map(prev.nodeOnline)
  let   motionEvents = prev.motionEvents

  switch (msg.type) {
    case 'camera_status_update':
      for (const s of msg.cameras) cameras.set(s.camera_id, s)
      break

    case 'node_status_update':
      nodeOnline.set(msg.node_id, msg.online)
      break

    case 'snapshot':
      for (const n of msg.nodes) nodeOnline.set(n.id, false)
      break

    case 'motion_event':
      // Prepend newest, keep last MAX_EVENTS
      motionEvents = [msg, ...prev.motionEvents].slice(0, MAX_EVENTS)
      break
  }

  return { cameras, nodeOnline, motionEvents }
}
