import { Link } from 'react-router-dom'
import type { Camera, CameraStatus } from '../types'
import { WHEPPlayer } from './WHEPPlayer'
import styles from './CameraCell.module.css'

interface Props {
  camera: Camera | null
  status: CameraStatus | undefined
}

export function CameraCell({ camera, status }: Props) {
  if (!camera) {
    return <div className={styles.empty} />
  }

  const offline = status && !status.connected

  return (
    <div className={`${styles.cell} ${offline ? styles.offline : ''}`}>
      <WHEPPlayer whepPath={camera.whep_path} cameraName={camera.name} />
      {status && (
        <span
          className={`${styles.dot} ${status.connected ? styles.green : styles.red}`}
          title={status.connected ? `${status.readers} viewer(s)` : 'no signal from camera'}
        />
      )}
      <Link to={`/playback/${camera.id}`} className={styles.playbackLink}>
        &#9654; Playback
      </Link>
    </div>
  )
}
