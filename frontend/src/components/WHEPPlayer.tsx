import { useCallback } from 'react'
import { useWHEP, type WHEPState } from '../hooks/useWHEP'
import styles from './WHEPPlayer.module.css'

interface Props {
  whepPath:  string
  cameraName: string
}

export function WHEPPlayer({ whepPath, cameraName }: Props) {
  const { videoRef, state } = useWHEP(whepPath)

  return (
    <div className={styles.player}>
      <video
        ref={videoRef}
        className={styles.video}
        autoPlay
        muted
        playsInline
      />
      <div className={styles.nameBar}>{cameraName}</div>
      {state !== 'connected' && (
        <div className={styles.overlay}>
          <StateIcon state={state} />
        </div>
      )}
    </div>
  )
}

function StateIcon({ state }: { state: WHEPState }) {
  switch (state) {
    case 'connecting': return <span className={styles.spinner} aria-label="Connecting" />
    case 'failed':     return <span className={styles.failed}>⚠ No signal</span>
    default:           return null
  }
}
