import { Box, VStack, HStack, Input, IconButton, Text, Avatar } from '@chakra-ui/react'
import { useState, useEffect, useRef } from 'react'
import { FiSend, FiTrash2 } from 'react-icons/fi'
import { Relay, onlyEvents } from 'applesauce-relay'
import { DEFAULT_RELAYS, getLiveChatATag, CHAT_HISTORY_CONFIG } from '../config'
import { useAccountManager } from 'applesauce-react/hooks'

interface ChatMessage {
  pubkey: string
  time: number
  content: string
  id: string
}

interface ProfileMetadata {
  name?: string
  picture?: string
  display_name?: string
}

interface RelayConfig {
  url: string
  enabledForProfile: boolean
  enabledForChat: boolean
  status: 'connected' | 'connecting' | 'error' | 'disconnected'
}

interface ChatPanelProps {
  isOpen: boolean
  currentPubkey: string | null
  onViewProfile: (pubkey: string) => void
}

export function ChatPanel({ isOpen, currentPubkey, onViewProfile }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [inputMessage, setInputMessage] = useState('')
  const [isSending, setIsSending] = useState(false)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const manager = useAccountManager()

  // Keep relay instances in ref to reuse for both subscribing and publishing
  const relayInstancesRef = useRef<Map<string, Relay>>(new Map())
  const subscriptionsRef = useRef<any[]>([])

  // Cache profiles by pubkey
  const [profiles, setProfiles] = useState<Map<string, ProfileMetadata>>(new Map())
  const profileFetchQueueRef = useRef<Set<string>>(new Set())

  // Get enabled relays from configuration
  const [enabledRelays, setEnabledRelays] = useState<string[]>([])
  const [profileRelays, setProfileRelays] = useState<string[]>([])

  // Load enabled relays from localStorage
  useEffect(() => {
    const loadEnabledRelays = () => {
      try {
        const savedRelays = localStorage.getItem('crossworld_relays')
        if (savedRelays) {
          const relays = JSON.parse(savedRelays) as RelayConfig[]
          const chatEnabled = relays.filter(r => r.enabledForChat).map(r => r.url)
          const profileEnabled = relays.filter(r => r.enabledForProfile).map(r => r.url)
          setEnabledRelays(chatEnabled)
          setProfileRelays(profileEnabled)
        } else {
          setEnabledRelays(DEFAULT_RELAYS)
          setProfileRelays(DEFAULT_RELAYS)
        }
      } catch (error) {
        console.error('[ChatPanel] Failed to load relay config:', error)
        setEnabledRelays(DEFAULT_RELAYS)
        setProfileRelays(DEFAULT_RELAYS)
      }
    }

    loadEnabledRelays()

    // Listen for relay config changes
    const handleRelayConfigChanged = () => {
      loadEnabledRelays()
      // Clear profile cache when relay config changes
      setProfiles(new Map())
      profileFetchQueueRef.current.clear()
    }

    window.addEventListener('relayConfigChanged', handleRelayConfigChanged)

    return () => {
      window.removeEventListener('relayConfigChanged', handleRelayConfigChanged)
    }
  }, [])

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  // Fetch profile metadata for a pubkey
  const fetchProfile = async (pubkey: string) => {
    if (profiles.has(pubkey) || profileFetchQueueRef.current.has(pubkey)) {
      return
    }

    if (profileRelays.length === 0) {
      return
    }

    profileFetchQueueRef.current.add(pubkey)

    for (const relayUrl of profileRelays) {
      try {
        const relay = new Relay(relayUrl)
        const events = await new Promise<any[]>((resolve) => {
          const collectedEvents: any[] = []
          let isResolved = false

          const cleanup = () => {
            if (!isResolved) {
              isResolved = true
              try { relay.close() } catch (e) {}
              resolve(collectedEvents)
            }
          }

          relay.request({
            kinds: [0],
            authors: [pubkey],
            limit: 1
          }).subscribe({
            next: (event: any) => {
              if (event === 'EOSE') {
                cleanup()
              } else if (event && event.kind === 0) {
                collectedEvents.push(event)
              }
            },
            error: () => cleanup(),
            complete: () => cleanup()
          })

          setTimeout(cleanup, 2000)
        })

        if (events.length > 0) {
          const latestEvent = events.sort((a, b) => b.created_at - a.created_at)[0]
          try {
            const metadata = JSON.parse(latestEvent.content) as ProfileMetadata
            setProfiles(prev => new Map(prev).set(pubkey, metadata))
            profileFetchQueueRef.current.delete(pubkey)
            return
          } catch (e) {
            console.error('Failed to parse profile metadata:', e)
          }
        }
      } catch (error) {
        console.error(`Failed to fetch profile from ${relayUrl}:`, error)
      }
    }

    profileFetchQueueRef.current.delete(pubkey)
  }

  // Fetch profiles for new messages
  useEffect(() => {
    messages.forEach(msg => {
      if (!profiles.has(msg.pubkey)) {
        fetchProfile(msg.pubkey)
      }
    })
  }, [messages, profileRelays])

  // Manage streaming subscription
  useEffect(() => {
    if (!isOpen || enabledRelays.length === 0) return

    console.log('[ChatPanel] Opening subscriptions with history...')

    // Clear profile cache when chat opens to refresh profile data
    setProfiles(new Map())
    profileFetchQueueRef.current.clear()

    const currentTime_s = Math.floor(Date.now() / 1000)
    const historySince_s = currentTime_s - CHAT_HISTORY_CONFIG.MAX_TIME_RANGE_S

    console.log(`[ChatPanel] Fetching messages from last ${CHAT_HISTORY_CONFIG.MAX_TIME_RANGE_S}s (${CHAT_HISTORY_CONFIG.MAX_TIME_RANGE_S / 3600}h)`)
    console.log(`[ChatPanel] Max messages: ${CHAT_HISTORY_CONFIG.MAX_MESSAGES}`)

    const connectToRelays = async () => {
      for (const relayUrl of enabledRelays) {
        try {
          console.log(`[ChatPanel] Connecting to ${relayUrl}`)

          // Reuse existing relay instance or create new one
          let relay = relayInstancesRef.current.get(relayUrl)
          if (!relay) {
            relay = new Relay(relayUrl)
            relayInstancesRef.current.set(relayUrl, relay)
          }

          // Single subscription handles both:
          // 1. Historical messages (from configured time range)
          // 2. Streaming new messages (continues after EOSE)
          const streamingSub = relay.subscription({
            kinds: [1311],
            '#a': [getLiveChatATag()],
            since: historySince_s,
            limit: CHAT_HISTORY_CONFIG.MAX_MESSAGES,
          })
          .pipe(onlyEvents())
          .subscribe({
            next: (event: any) => {
              // Verify the event has our a-tag
              const hasCorrectTag = event.tags?.some(
                (tag: string[]) => tag[0] === 'a' && tag[1] === getLiveChatATag()
              )

              if (!hasCorrectTag) {
                console.warn(`[ChatPanel] Event ${event.id} missing correct a-tag, ignoring`)
                return
              }

              console.log(`[ChatPanel] Received message:`, event.id)

              setMessages(prev => {
                // Check if message already exists
                if (prev.some(m => m.id === event.id)) {
                  return prev
                }

                const newMessage: ChatMessage = {
                  pubkey: event.pubkey,
                  time: event.created_at,
                  content: event.content,
                  id: event.id,
                }

                // Insert in chronological order
                const updated = [...prev, newMessage].sort((a, b) => a.time - b.time)
                return updated
              })
            },
            error: (err: any) => {
              console.error(`Relay ${relayUrl} streaming error:`, err)
            },
          })

          subscriptionsRef.current.push(streamingSub)
          console.log(`[ChatPanel] Streaming subscription created for ${relayUrl}`)
        } catch (error) {
          console.error(`[ChatPanel] Failed to connect to ${relayUrl}:`, error)
        }
      }
    }

    connectToRelays()

    // Cleanup: unsubscribe when chat closes
    return () => {
      console.log(`[ChatPanel] Closing ${subscriptionsRef.current.length} subscriptions`)
      subscriptionsRef.current.forEach(sub => {
        try {
          sub.unsubscribe()
        } catch (e) {
          console.error('[ChatPanel] Error unsubscribing:', e)
        }
      })
      subscriptionsRef.current = []
    }
  }, [isOpen, enabledRelays])

  // Close relay connections completely when component unmounts
  useEffect(() => {
    return () => {
      relayInstancesRef.current.forEach(relay => {
        try {
          relay.close()
        } catch (e) {
          console.error('Error closing relay:', e)
        }
      })
      relayInstancesRef.current.clear()
    }
  }, [])

  const handleSendMessage = async () => {
    if (!inputMessage.trim() || !currentPubkey || isSending) return

    setIsSending(true)
    try {
      const account = manager.accounts.find(a => a.pubkey === currentPubkey)
      if (!account || !account.signer) {
        console.error('No account or signer found')
        return
      }

      const messageEvent = {
        kind: 1311,
        tags: [
          ['a', getLiveChatATag()],
        ],
        content: inputMessage.trim(),
        created_at: Math.floor(Date.now() / 1000),
      }

      const signedEvent = await account.signer.signEvent(messageEvent)

      console.log('[ChatPanel] Sending message:', signedEvent.id)

      // Publish to all enabled relays
      const publishPromises = []
      for (const relayUrl of enabledRelays) {
        const relay = relayInstancesRef.current.get(relayUrl)
        if (relay) {
          try {
            console.log(`[ChatPanel] Publishing to ${relayUrl}`)
            publishPromises.push(relay.publish(signedEvent))
          } catch (error) {
            console.error(`[ChatPanel] Failed to publish to ${relayUrl}:`, error)
          }
        }
      }

      await Promise.allSettled(publishPromises)
      console.log('[ChatPanel] Message sent')

      setInputMessage('')
    } catch (error) {
      console.error('Failed to send message:', error)
    } finally {
      setIsSending(false)
    }
  }

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSendMessage()
    }
  }

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000)
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, '0')
    const day = String(date.getDate()).padStart(2, '0')
    const hours = String(date.getHours()).padStart(2, '0')
    const minutes = String(date.getMinutes()).padStart(2, '0')
    const seconds = String(date.getSeconds()).padStart(2, '0')
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
  }

  const getProfileName = (pubkey: string) => {
    const profile = profiles.get(pubkey)
    return profile?.display_name || profile?.name || 'Anonymous'
  }

  const getProfilePicture = (pubkey: string) => {
    const profile = profiles.get(pubkey)
    return profile?.picture
  }

  const handleDeleteMessage = async (eventId: string) => {
    if (!currentPubkey) return

    try {
      const account = manager.accounts.find(a => a.pubkey === currentPubkey)
      if (!account || !account.signer) {
        console.error('[ChatPanel] No account or signer found for delete')
        return
      }

      // Remove message from local state immediately
      setMessages(prev => prev.filter(m => m.id !== eventId))

      // Create NIP-09 delete event (kind 5)
      const deleteEvent = {
        kind: 5,
        tags: [
          ['e', eventId],
        ],
        content: 'deleted',
        created_at: Math.floor(Date.now() / 1000),
      }

      const signedEvent = await account.signer.signEvent(deleteEvent)

      console.log('[ChatPanel] Sending delete request:', signedEvent.id)

      // Publish to all enabled relays
      const publishPromises = []
      for (const relayUrl of enabledRelays) {
        const relay = relayInstancesRef.current.get(relayUrl)
        if (relay) {
          try {
            publishPromises.push(relay.publish(signedEvent))
          } catch (error) {
            console.error(`[ChatPanel] Failed to publish delete to ${relayUrl}:`, error)
          }
        }
      }

      await Promise.allSettled(publishPromises)
      console.log('[ChatPanel] Delete request sent')
    } catch (error) {
      console.error('[ChatPanel] Failed to delete message:', error)
    }
  }

  if (!isOpen) return null

  return (
    <Box
      position="fixed"
      bottom={0}
      left="68px"
      right={0}
      height="300px"
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(12px)"
      borderTop="1px solid rgba(255, 255, 255, 0.1)"
      zIndex={900}
      display="flex"
      flexDirection="column"
      _before={{
        content: '""',
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: `
          radial-gradient(ellipse at 20% 30%, rgba(255, 255, 255, 0.03) 0%, transparent 50%),
          radial-gradient(ellipse at 80% 70%, rgba(255, 255, 255, 0.03) 0%, transparent 50%),
          repeating-linear-gradient(
            45deg,
            transparent,
            transparent 10px,
            rgba(255, 255, 255, 0.01) 10px,
            rgba(255, 255, 255, 0.01) 20px
          )
        `,
        pointerEvents: 'none',
        zIndex: -1,
      }}
    >
      {/* Messages List */}
      <VStack
        flex={1}
        overflowY="auto"
        spacing={3}
        p={4}
        align="stretch"
        css={{
          '&::-webkit-scrollbar': {
            width: '8px',
          },
          '&::-webkit-scrollbar-track': {
            background: 'rgba(255, 255, 255, 0.05)',
          },
          '&::-webkit-scrollbar-thumb': {
            background: 'rgba(255, 255, 255, 0.2)',
            borderRadius: '4px',
          },
          '&::-webkit-scrollbar-thumb:hover': {
            background: 'rgba(255, 255, 255, 0.3)',
          },
        }}
      >
        {messages.length === 0 ? (
          <Text color="whiteAlpha.500" textAlign="center" py={8}>
            No messages yet. Start chatting!
          </Text>
        ) : (
          messages.map((msg) => (
            <HStack key={msg.id} spacing={3} align="flex-start">
              <Avatar
                size="sm"
                src={getProfilePicture(msg.pubkey)}
                name={getProfileName(msg.pubkey)}
                bg="whiteAlpha.300"
                cursor="pointer"
                onClick={() => onViewProfile(msg.pubkey)}
              />
              <VStack align="stretch" spacing={1} flex={1}>
                {/* Header line: time, name, buttons */}
                <HStack spacing={2} fontSize="2xs" color="whiteAlpha.600">
                  <Text>{formatTime(msg.time)}</Text>
                  <Text
                    fontWeight="medium"
                    color="whiteAlpha.800"
                    cursor="pointer"
                    _hover={{ color: 'white' }}
                    onClick={() => onViewProfile(msg.pubkey)}
                  >
                    {getProfileName(msg.pubkey)}
                  </Text>
                  {msg.pubkey === currentPubkey && (
                    <IconButton
                      aria-label="Delete message"
                      icon={<FiTrash2 />}
                      size="xs"
                      variant="ghost"
                      minW="auto"
                      h="auto"
                      p={0.5}
                      color="whiteAlpha.500"
                      _hover={{ color: 'red.400' }}
                      onClick={() => handleDeleteMessage(msg.id)}
                    />
                  )}
                </HStack>
                {/* Message content */}
                <Text fontSize="sm" color="white" whiteSpace="pre-wrap" wordBreak="break-word">
                  {msg.content}
                </Text>
              </VStack>
            </HStack>
          ))
        )}
        <div ref={messagesEndRef} />
      </VStack>

      {/* Message Input */}
      <HStack
        p={3}
        borderTop="1px solid rgba(255, 255, 255, 0.1)"
        bg="rgba(0, 0, 0, 0.3)"
        spacing={2}
      >
        <Input
          value={inputMessage}
          onChange={(e) => setInputMessage(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder={currentPubkey ? "Type a message..." : "Login to send messages"}
          disabled={!currentPubkey || isSending}
          bg="rgba(255, 255, 255, 0.05)"
          border="1px solid rgba(255, 255, 255, 0.1)"
          color="white"
          _placeholder={{ color: 'whiteAlpha.500' }}
          _hover={{ borderColor: 'rgba(255, 255, 255, 0.2)' }}
          _focus={{ borderColor: 'blue.400', boxShadow: '0 0 0 1px #3182ce' }}
          _disabled={{ opacity: 0.5, cursor: 'not-allowed' }}
        />
        <IconButton
          aria-label="Send message"
          icon={<FiSend />}
          onClick={handleSendMessage}
          isDisabled={!currentPubkey || !inputMessage.trim() || isSending}
          isLoading={isSending}
          colorScheme="blue"
        />
      </HStack>
    </Box>
  )
}
