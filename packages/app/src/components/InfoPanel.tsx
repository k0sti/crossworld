import { Box, VStack, Text, Link, Divider, Heading, UnorderedList, ListItem } from '@chakra-ui/react'
import ReactMarkdown from 'react-markdown'
import type { Components } from 'react-markdown'
import aboutContent from '@assets/about.md?raw'

const markdownComponents: Components = {
  p: ({ children }) => <Text fontSize="xs">{children}</Text>,
  a: ({ href, children }) => (
    <Link href={href} isExternal color="blue.300" _hover={{ color: 'blue.200' }}>
      {children}
    </Link>
  ),
  h1: ({ children }) => (<Heading fontSize="lg" mt={0} mb={0}>{children}</Heading>),
  h3: ({ children }) => (<Heading fontSize="sm" mt={0} mb={0}>{children}</Heading>),
  ul: ({ children }) => <UnorderedList fontSize="xs" ml={4}>{children}</UnorderedList>,
  li: ({ children }) => <ListItem>{children}</ListItem>,
  hr: () => <Divider borderColor="rgba(255, 255, 255, 0.2)" my={2} />,
}

export function InfoPanel() {
  return (
    <Box
      position="fixed"
      top="60px"
      right="16px"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.7)"
      backdropFilter="blur(8px)"
      p={4}
      minW="350px"
      maxW="400px"
      _before={{
        content: '""',
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: `
          radial-gradient(ellipse at 20% 30%, rgba(255, 255, 255, 0.03) 0%, transparent 50%),
          radial-gradient(ellipse at 80% 70%, rgba(255, 255, 255, 0.03) 0%, transparent 10%),
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
      <VStack align="stretch" gap={3}>
        <VStack align="stretch" gap={2} color="white">
          <ReactMarkdown components={markdownComponents}>{aboutContent}</ReactMarkdown>
        </VStack>
      </VStack>
    </Box>
  )
}
