import { useEffect, useState } from 'react'
import { voiceManager, type VoiceStatus } from '../services/voice/manager'
import type { Participant } from '../services/voice/subscriber'

export function useVoice() {
  const [status, setStatus] = useState<VoiceStatus>('disconnected')
  const [micEnabled, setMicEnabled] = useState(false)
  const [speaking, setSpeaking] = useState(false)
  const [participants, setParticipants] = useState<Participant[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    // Subscribe to voice manager state
    const unsubStatus = voiceManager.status.subscribe(setStatus)
    const unsubMic = voiceManager.micEnabled.subscribe(setMicEnabled)
    const unsubSpeaking = voiceManager.speaking.subscribe(setSpeaking)
    const unsubParticipants = voiceManager.participants.subscribe((participants) => {
      setParticipants(Array.from(participants.values()))
    })
    const unsubError = voiceManager.error.subscribe(setError)

    return () => {
      unsubStatus()
      unsubMic()
      unsubSpeaking()
      unsubParticipants()
      unsubError()
    }
  }, [])

  const connect = async (streamingUrl: string, npub: string) => {
    try {
      await voiceManager.connect(streamingUrl, npub)
    } catch (err) {
      console.error('Failed to connect to voice:', err)
    }
  }

  const disconnect = async () => {
    try {
      await voiceManager.disconnect()
    } catch (err) {
      console.error('Failed to disconnect from voice:', err)
    }
  }

  const toggleMic = async () => {
    try {
      await voiceManager.toggleMic()
    } catch (err) {
      console.error('Failed to toggle mic:', err)
    }
  }

  const setParticipantVolume = (npub: string, volume: number) => {
    voiceManager.setParticipantVolume(npub, volume)
  }

  const setParticipantMuted = (npub: string, muted: boolean) => {
    voiceManager.setParticipantMuted(npub, muted)
  }

  return {
    status,
    micEnabled,
    speaking,
    participants,
    participantCount: participants.length,
    error,
    connect,
    disconnect,
    toggleMic,
    setParticipantVolume,
    setParticipantMuted,
    isConnected: status === 'connected',
    setClientStatusService: voiceManager.setClientStatusService.bind(voiceManager),
  }
}
