import * as logger from '../utils/logger';
import { useState, useEffect } from 'react'
import {
  Modal,
  ModalOverlay,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  ModalCloseButton,
  Button,
  VStack,
  FormControl,
  FormLabel,
  Input,
  Textarea,
  Text,
  HStack,
  Badge,
  Box,
  Divider,
  useToast,
} from '@chakra-ui/react'
import { getMacroDepth, getMicroDepth } from '../config/depth-config'
import { getModelCSM, getModelStats, countCSMLines, getCSMSize, formatBytes } from '../utils/csmUtils'
import { publishWorld } from '../services/world-storage'
import type { AccountManager } from 'applesauce-accounts'

interface PublishWorldModalProps {
  isOpen: boolean
  onClose: () => void
  accountManager: AccountManager | null
  geometryControllerRef?: React.MutableRefObject<any>
}

export function PublishWorldModal({
  isOpen,
  onClose,
  accountManager,
  geometryControllerRef,
}: PublishWorldModalProps) {
  const [title, setTitle] = useState('')
  const [description, setDescription] = useState('')
  const [isPublishing, setIsPublishing] = useState(false)
  const [isLoadingStats, setIsLoadingStats] = useState(true)
  const [stats, setStats] = useState({
    macroDepth: 0,
    microDepth: 0,
    totalDepth: 0,
    worldSize: 0,
    faceCount: 0,
    vertexCount: 0,
    csmLines: 0,
    csmSize: 0,
  })
  const [csmPreview, setCsmPreview] = useState<string>('')

  const toast = useToast()

  // Load stats when modal opens
  useEffect(() => {
    if (!isOpen) return

    const loadStats = async () => {
      setIsLoadingStats(true)
      try {
        // Read depths from geometry controller if available, otherwise use global config
        let macroDepth = getMacroDepth()
        let microDepth = getMicroDepth()

        if (geometryControllerRef?.current) {
          macroDepth = geometryControllerRef.current.getMacroDepth()
          microDepth = geometryControllerRef.current.getMicroDepth()
        } else {
        }

        if (macroDepth === 0 || macroDepth < 1 || macroDepth > 10) {
          throw new Error(`Invalid macro depth: ${macroDepth}. Must be between 1 and 10.`)
        }

        const totalDepth = macroDepth + microDepth
        const worldSize = 1 << macroDepth

        const csmText = await getModelCSM(geometryControllerRef?.current)
        const meshStats = await getModelStats(geometryControllerRef?.current)

        setStats({
          macroDepth,
          microDepth,
          totalDepth,
          worldSize,
          faceCount: Math.floor(meshStats.faceCount),
          vertexCount: meshStats.vertexCount,
          csmLines: countCSMLines(csmText),
          csmSize: getCSMSize(csmText),
        })
        setCsmPreview(csmText)
      } catch (error) {
        logger.error('ui', '[PublishWorld] Failed to load stats:', error)
        toast({
          title: 'Failed to load world stats',
          description: error instanceof Error ? error.message : 'Unknown error',
          status: 'error',
          duration: 5000,
        })
      } finally {
        setIsLoadingStats(false)
      }
    }

    loadStats()
  }, [isOpen, toast, geometryControllerRef])

  const handlePublish = async () => {
    if (!accountManager) {
      toast({
        title: 'Not logged in',
        description: 'Please log in to publish worlds',
        status: 'error',
        duration: 3000,
      })
      return
    }

    setIsPublishing(true)
    try {
      const csmContent = await getModelCSM(geometryControllerRef?.current)

      await publishWorld(accountManager, csmContent, {
        title: title.trim() || undefined,
        description: description.trim() || undefined,
      })

      toast({
        title: 'World published!',
        description: 'Your world has been saved to Nostr',
        status: 'success',
        duration: 3000,
      })

      onClose()
      setTitle('')
      setDescription('')
    } catch (error) {
      logger.error('ui', '[PublishWorld] Failed to publish:', error)
      toast({
        title: 'Failed to publish world',
        description: error instanceof Error ? error.message : 'Unknown error',
        status: 'error',
        duration: 5000,
      })
    } finally {
      setIsPublishing(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="lg" isCentered>
      <ModalOverlay />
      <ModalContent>
        <ModalHeader>Publish World to Nostr</ModalHeader>
        <ModalCloseButton />

        <ModalBody>
          <VStack spacing={4} align="stretch">
            {/* World Stats */}
            {isLoadingStats ? (
              <Box textAlign="center" py={4}>
                <Text fontSize="sm" color="gray.500">Loading world information...</Text>
              </Box>
            ) : (
              <Box>
                <Text fontSize="sm" fontWeight="bold" mb={2} color="gray.600">
                  World Information
                </Text>
                <VStack spacing={2} align="stretch">
                  <HStack>
                    <Text fontSize="sm" color="gray.500">Configuration:</Text>
                    <Badge colorScheme="cyan">macro {stats.macroDepth}</Badge>
                    <Badge colorScheme="cyan">micro {stats.microDepth}</Badge>
                    <Badge colorScheme="blue">size {stats.worldSize}</Badge>
                  </HStack>

                  <HStack>
                    <Text fontSize="sm" color="gray.500">Geometry:</Text>
                    <Badge colorScheme="green">{stats.faceCount.toLocaleString()} faces</Badge>
                    <Badge colorScheme="green">{stats.vertexCount.toLocaleString()} vertices</Badge>
                  </HStack>

                  <HStack>
                    <Text fontSize="sm" color="gray.500">CSM Data:</Text>
                    <Badge colorScheme="purple">{stats.csmLines} lines</Badge>
                    <Badge colorScheme="purple">{formatBytes(stats.csmSize)}</Badge>
                  </HStack>
                </VStack>
              </Box>
            )}

            <Divider />

            {/* Title Input */}
            <FormControl>
              <FormLabel fontSize="sm">Title (optional)</FormLabel>
              <Input
                placeholder="My Amazing World"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                maxLength={100}
              />
            </FormControl>

            {/* Description Input */}
            <FormControl>
              <FormLabel fontSize="sm">Description (optional)</FormLabel>
              <Textarea
                placeholder="A beautiful voxel world with..."
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                maxLength={500}
                rows={3}
              />
            </FormControl>

            <Text fontSize="xs" color="gray.500">
              Your world will be published to Nostr as an addressable event.
              Only the latest version for this configuration will be stored.
            </Text>

            {/* CSM Preview */}
            {!isLoadingStats && csmPreview && (
              <>
                <Divider />
                <Box>
                  <Text fontSize="sm" fontWeight="bold" mb={2} color="gray.600">
                    CSM Code Preview
                  </Text>
                  <Box
                    bg="gray.50"
                    borderRadius="md"
                    border="1px solid"
                    borderColor="gray.200"
                    p={3}
                    maxH="200px"
                    overflowY="auto"
                    fontFamily="mono"
                    fontSize="xs"
                    whiteSpace="pre"
                    color="gray.700"
                  >
                    {csmPreview}
                  </Box>
                </Box>
              </>
            )}
          </VStack>
        </ModalBody>

        <ModalFooter>
          <Button variant="ghost" mr={3} onClick={onClose}>
            Cancel
          </Button>
          <Button
            colorScheme="blue"
            onClick={handlePublish}
            isLoading={isPublishing || isLoadingStats}
            loadingText="Publishing..."
          >
            Publish to Nostr
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  )
}
