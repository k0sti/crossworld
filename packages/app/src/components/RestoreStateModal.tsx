import {
  Modal,
  ModalOverlay,
  ModalContent,
  ModalBody,
  VStack,
  Text,
  Spinner,
} from '@chakra-ui/react'

interface RestoreStateModalProps {
  isOpen: boolean
}

export function RestoreStateModal({
  isOpen,
}: RestoreStateModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={() => {}} isCentered closeOnOverlayClick={false}>
      <ModalOverlay />
      <ModalContent>
        <ModalBody py={8}>
          <VStack spacing={4}>
            <Spinner size="xl" color="blue.500" thickness="4px" />
            <Text fontSize="lg" fontWeight="medium">
              Fetching previous state...
            </Text>
          </VStack>
        </ModalBody>
      </ModalContent>
    </Modal>
  )
}
