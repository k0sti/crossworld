import { VStack, Text, Link, Divider, Heading, UnorderedList, ListItem, Button } from '@chakra-ui/react'
import ReactMarkdown from 'react-markdown'
import type { Components } from 'react-markdown'
import aboutContent from '@assets/about.md?raw'
import { ResponsivePanel } from './ResponsivePanel'

const markdownComponents: Components = {
  p: ({ children }) => <Text fontSize="sm">{children}</Text>,
  a: ({ href, children }) => (
    <Link href={href} isExternal color="blue.300" _hover={{ color: 'blue.200' }}>
      {children}
    </Link>
  ),
  h1: ({ children }) => (<Heading fontSize="xl" mt={0} mb={2}>{children}</Heading>),
  h3: ({ children }) => (<Heading fontSize="md" mt={2} mb={1}>{children}</Heading>),
  ul: ({ children }) => <UnorderedList fontSize="sm" ml={4}>{children}</UnorderedList>,
  li: ({ children }) => <ListItem>{children}</ListItem>,
  hr: () => <Divider borderColor="rgba(255, 255, 255, 0.2)" my={3} />,
}

interface InfoPanelProps {
  isOpen: boolean
  onClose: () => void
}

export function InfoPanel({ isOpen, onClose }: InfoPanelProps) {
  return (
    <ResponsivePanel
      isOpen={isOpen}
      onClose={onClose}
      forceFullscreen={true}
      title="ℹ️ About"
      actions={
        <Button onClick={onClose} colorScheme="blue">
          Close
        </Button>
      }
    >
      <VStack align="stretch" gap={3} maxW="800px" mx="auto" color="white">
        <ReactMarkdown components={markdownComponents}>{aboutContent}</ReactMarkdown>
      </VStack>
    </ResponsivePanel>
  )
}
