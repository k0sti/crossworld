import { Box, Flex } from '@chakra-ui/react'
import { ProfileButton } from './ProfileButton'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onLogout: () => void
}

export function TopBar({ pubkey, onLogin, onLogout }: TopBarProps) {
  return (
    <Box
      as="header"
      position="fixed"
      top={0}
      left={0}
      right={0}
      zIndex={1000}
      bg="rgba(255, 255, 255, 0.9)"
      backdropFilter="blur(10px)"
      px={4}
      py={2}
      boxShadow="sm"
    >
      <Flex justify="space-between" align="center">
        <Box fontSize="xl" fontWeight="bold">Crossworld</Box>
        <ProfileButton pubkey={pubkey} onLogin={onLogin} onLogout={onLogout} />
      </Flex>
    </Box>
  )
}
