/**
 * Re-export relay settings from common package
 *
 * The relay-settings service has been moved to @crossworld/common
 * to be the single source of truth for all relay configuration.
 */

export {
  getEnabledWorldRelays,
  getEnabledProfileRelays,
  getAllEnabledRelays,
} from '@crossworld/common'
