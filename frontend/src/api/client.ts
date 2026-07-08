import { useAuthStore } from '../store/auth'
import type { Camera, MotionEvent, Node, NodeCapacity, RecordingSegment } from '../types'

const BASE = '/api/v1'

async function request<T>(path: string, opts?: RequestInit): Promise<T> {
  const token = useAuthStore.getState().token
  const resp = await fetch(`${BASE}${path}`, {
    ...opts,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...opts?.headers,
    },
  })
  if (resp.status === 401) {
    useAuthStore.getState().clearToken()
    throw new Error('Unauthorized')
  }
  if (!resp.ok) {
    throw new Error(`API error ${resp.status}: ${path}`)
  }
  return resp.json() as Promise<T>
}

export const api = {
  login: (email: string, password: string) =>
    request<{ access_token: string }>('/auth/login', {
      method:  'POST',
      body:    JSON.stringify({ email, password }),
    }),

  cameras: () => request<Camera[]>('/cameras'),

  nodes: () => request<Node[]>('/nodes'),

  nodeCapacity: (nodeId: string) => request<NodeCapacity>(`/nodes/${nodeId}/capacity`),

  /** Returns the WebSocket URL for the dashboard feed. */
  dashboardWsUrl: () => {
    const token = useAuthStore.getState().token ?? ''
    const proto = location.protocol === 'https:' ? 'wss' : 'ws'
    return `${proto}://${location.host}/api/v1/ws/dashboard?token=${encodeURIComponent(token)}`
  },

  recordings: (cameraId: string, from: Date, to: Date) =>
    request<RecordingSegment[]>(
      `/cameras/${cameraId}/recordings?from=${from.toISOString()}&to=${to.toISOString()}`,
    ),

  events: (cameraId: string, from: Date, to: Date) =>
    request<MotionEvent[]>(
      `/cameras/${cameraId}/events?from=${from.toISOString()}&to=${to.toISOString()}`,
    ),

  /** URL for streaming a single recorded segment through the coordinator proxy. */
  segmentUrl: (cameraId: string, recordingId: string): string => {
    const token = useAuthStore.getState().token ?? ''
    return `${BASE}/cameras/${cameraId}/segments/${recordingId}?token=${encodeURIComponent(token)}`
  },

  /** URL for downloading a clip (FFmpeg concat) through the coordinator proxy. */
  exportUrl: (cameraId: string, from: Date, to: Date): string => {
    const token = useAuthStore.getState().token ?? ''
    return (
      `${BASE}/cameras/${cameraId}/export` +
      `?token=${encodeURIComponent(token)}` +
      `&from=${encodeURIComponent(from.toISOString())}` +
      `&to=${encodeURIComponent(to.toISOString())}`
    )
  },
}
