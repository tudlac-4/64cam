export interface Camera {
  id:          string
  node_id:     string
  name:        string
  rtsp_url:    string
  stream_path: string | null
  enabled:     boolean
  whep_path:   string
  created_at:  string
  updated_at:  string
}

export interface Node {
  id:           string
  name:         string
  status:       'pending' | 'approved' | 'rejected'
  last_seen_at: string | null
  ip_addr:      string | null
}

export interface NodeCapacity {
  node_id:      string
  camera_count: number
  max_cameras:  number
  headroom:     number
}

export interface CameraStatusUpdate {
  type:        'camera_status_update'
  node_id:     string
  cameras:     CameraStatus[]
}

export interface CameraStatus {
  camera_id:   string
  stream_path: string
  connected:   boolean
  readers:     number
}

export interface NodeStatusUpdate {
  type:    'node_status_update'
  node_id: string
  online:  boolean
}

export interface Snapshot {
  type:  'snapshot'
  nodes: Node[]
}

export interface MotionEventMessage {
  type:        'motion_event'
  camera_id:   string
  occurred_at: string
  source:      'onvif' | 'diff'
  score:       number | null
}

export type DashboardMessage = CameraStatusUpdate | NodeStatusUpdate | Snapshot | MotionEventMessage

export type GridSize = 4 | 9 | 16

export interface RecordingSegment {
  id:            string
  started_at:    string
  ended_at:      string
  duration_secs: number
  size_bytes:    number
}

export interface MotionEvent {
  id:          string
  camera_id:   string
  occurred_at: string
  source:      'onvif' | 'diff'
  score:       number | null
}
