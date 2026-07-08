import type { Camera, CameraStatus, GridSize } from '../types'
import { CameraCell } from './CameraCell'
import styles from './CameraGrid.module.css'

interface Props {
  cameras:    Camera[]
  statusMap:  Map<string, CameraStatus>
  gridSize:   GridSize
  onGridSize: (n: GridSize) => void
}

const COLS: Record<GridSize, number> = { 4: 2, 9: 3, 16: 4 }

export function CameraGrid({ cameras, statusMap, gridSize, onGridSize }: Props) {
  const cols  = COLS[gridSize]
  const slots = Array.from({ length: gridSize }, (_, i) => cameras[i] ?? null)

  return (
    <div className={styles.wrapper}>
      <div className={styles.toolbar}>
        <span className={styles.title}>64cam</span>
        <div className={styles.sizeButtons}>
          {([4, 9, 16] as GridSize[]).map((n) => (
            <button
              key={n}
              className={`${styles.sizeBtn} ${gridSize === n ? styles.active : ''}`}
              onClick={() => onGridSize(n)}
              title={`${Math.sqrt(n)}×${Math.sqrt(n)} grid`}
            >
              {Math.sqrt(n)}×{Math.sqrt(n)}
            </button>
          ))}
        </div>
      </div>

      <div
        className={styles.grid}
        style={{ '--cols': cols } as React.CSSProperties}
      >
        {slots.map((cam, i) => (
          <CameraCell
            key={cam?.id ?? `empty-${i}`}
            camera={cam}
            status={cam ? statusMap.get(cam.id) : undefined}
          />
        ))}
      </div>
    </div>
  )
}
