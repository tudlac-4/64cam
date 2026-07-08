import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../api/client'
import { useAuthStore } from '../store/auth'
import { useStatusWs } from '../hooks/useStatusWs'
import { CameraGrid } from '../components/CameraGrid'
import { EventFeed } from '../components/EventFeed'
import { NodeBar } from '../components/NodeBar'
import type { Camera, GridSize, Node } from '../types'
import styles from './Dashboard.module.css'

const GRID_KEY = 'cam64-gridsize'

function savedGridSize(): GridSize {
  const v = localStorage.getItem(GRID_KEY)
  if (v === '4' || v === '9' || v === '16') return Number(v) as GridSize
  return 4
}

export function Dashboard() {
  const navigate   = useNavigate()
  const token      = useAuthStore((s) => s.token)
  const clearToken = useAuthStore((s) => s.clearToken)
  const [cameras,  setCameras]  = useState<Camera[]>([])
  const [nodes,    setNodes]    = useState<Node[]>([])
  const [gridSize, setGridSize] = useState<GridSize>(savedGridSize)
  const { cameras: statusMap, nodeOnline, motionEvents } = useStatusWs(!!token)

  useEffect(() => {
    if (!token) { navigate('/login', { replace: true }); return }
    api.cameras()
      .then(setCameras)
      .catch(() => { clearToken(); navigate('/login', { replace: true }) })
    api.nodes()
      .then(setNodes)
      .catch(() => {})
  }, [token, navigate, clearToken])

  const handleGridSize = (n: GridSize) => {
    setGridSize(n)
    localStorage.setItem(GRID_KEY, String(n))
  }

  const onlineIds = new Set(
    [...nodeOnline.entries()]
      .filter(([, online]) => online)
      .map(([id]) => id),
  )

  return (
    <div className={styles.layout}>
      <div className={styles.main}>
        <NodeBar nodes={nodes} onlineIds={onlineIds} />
        <CameraGrid
          cameras={cameras}
          statusMap={statusMap}
          gridSize={gridSize}
          onGridSize={handleGridSize}
        />
      </div>
      <EventFeed events={motionEvents} cameras={cameras} />
    </div>
  )
}
