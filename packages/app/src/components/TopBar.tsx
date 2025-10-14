import { Box, Flex } from '@chakra-ui/react'
import { ProfileButton } from './ProfileButton'

interface TopBarProps {
  pubkey: string | null
  onLogin: (pubkey: string) => void
  onLogout: () => void
}

export function TopBar({ pubkey, onLogin, onLogout }: TopBarProps) {
  return (
    <Box as="header">
      <Flex justify="space-between" align="center">
        <Box>Crossworld</Box>
        <ProfileButton pubkey={pubkey} onLogin={onLogin} onLogout={onLogout} />
      </Flex>
    </Box>
  )
}
