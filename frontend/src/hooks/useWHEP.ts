import { useEffect, useRef, useState } from 'react'

export type WHEPState = 'idle' | 'connecting' | 'connected' | 'failed'

/**
 * Manages a single WebRTC WHEP connection.
 * Posts the SDP offer immediately (without waiting for ICE gathering) —
 * MediaMTX supports this and the browser negotiates candidates after the fact.
 */
export function useWHEP(whepPath: string | null) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const pcRef    = useRef<RTCPeerConnection | null>(null)
  const [state, setState] = useState<WHEPState>('idle')

  useEffect(() => {
    if (!whepPath) return

    let cancelled = false

    const pc = new RTCPeerConnection({
      iceServers: [{ urls: 'stun:stun.l.google.com:19302' }],
    })
    pcRef.current = pc

    // We only want to receive; adding transceivers triggers offer generation
    pc.addTransceiver('video', { direction: 'recvonly' })
    pc.addTransceiver('audio', { direction: 'recvonly' })

    pc.ontrack = (ev) => {
      const video = videoRef.current
      if (video && ev.streams[0]) {
        video.srcObject = ev.streams[0]
        video.play().catch(() => { /* autoplay policy — user interaction required */ })
      }
    }

    pc.onconnectionstatechange = () => {
      if (cancelled) return
      switch (pc.connectionState) {
        case 'connected':    setState('connected'); break
        case 'failed':
        case 'disconnected': setState('failed');    break
      }
    }

    setState('connecting')

    void (async () => {
      try {
        const offer = await pc.createOffer()
        await pc.setLocalDescription(offer)

        const resp = await fetch(whepPath, {
          method:  'POST',
          headers: { 'Content-Type': 'application/sdp' },
          body:    pc.localDescription!.sdp,
        })

        if (!resp.ok) {
          throw new Error(`WHEP signaling failed: ${resp.status}`)
        }

        const answerSdp = await resp.text()
        await pc.setRemoteDescription({ type: 'answer', sdp: answerSdp })
      } catch (err) {
        if (!cancelled) {
          console.error('[WHEP]', err)
          setState('failed')
        }
      }
    })()

    return () => {
      cancelled = true
      pc.close()
      pcRef.current = null
      if (videoRef.current) {
        videoRef.current.srcObject = null
      }
      setState('idle')
    }
  }, [whepPath])

  return { videoRef, state }
}
