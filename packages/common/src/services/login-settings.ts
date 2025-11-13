/**
 * Login settings service for persisting user login information
 */

export type LoginMethod = 'extension' | 'guest' | 'amber'

export interface LoginSettings {
  method: LoginMethod
  pubkey: string
  /** Timestamp of last login */
  lastLogin: number
}

export interface GuestAccountData {
  account: any // Serialized SimpleAccount
  name: string
}

const STORAGE_KEY = 'crossworld_login_settings'
const GUEST_ACCOUNT_KEY = 'crossworld_guest_account'

export class LoginSettingsService {
  /**
   * Save login settings to localStorage
   */
  static save(settings: LoginSettings): void {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(settings))
    } catch (error) {
      console.error('[LoginSettings] Failed to save:', error)
    }
  }

  /**
   * Load login settings from localStorage
   */
  static load(): LoginSettings | null {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (!stored) {
        return null
      }

      const settings = JSON.parse(stored) as LoginSettings
      return settings
    } catch (error) {
      console.error('[LoginSettings] Failed to load:', error)
      return null
    }
  }

  /**
   * Clear login settings from localStorage (but keep guest account data)
   */
  static clear(): void {
    try {
      localStorage.removeItem(STORAGE_KEY)
    } catch (error) {
      console.error('[LoginSettings] Failed to clear:', error)
    }
  }

  /**
   * Save guest account data to localStorage (persistent)
   */
  static saveGuestAccount(data: GuestAccountData): void {
    try {
      localStorage.setItem(GUEST_ACCOUNT_KEY, JSON.stringify(data))
    } catch (error) {
      console.error('[LoginSettings] Failed to save guest account:', error)
    }
  }

  /**
   * Load guest account data from localStorage
   */
  static loadGuestAccount(): GuestAccountData | null {
    try {
      // First check new location
      const stored = localStorage.getItem(GUEST_ACCOUNT_KEY)
      if (stored) {
        const data = JSON.parse(stored) as GuestAccountData
        return data
      }

      // Fallback to legacy location
      const legacy = localStorage.getItem('guestAccount')
      if (legacy) {
        const data = JSON.parse(legacy) as GuestAccountData
        // Migrate to new location
        this.saveGuestAccount(data)
        localStorage.removeItem('guestAccount')
        return data
      }

      return null
    } catch (error) {
      console.error('[LoginSettings] Failed to load guest account:', error)
      return null
    }
  }

  /**
   * Clear guest account data from localStorage
   */
  static clearGuestAccount(): void {
    try {
      localStorage.removeItem(GUEST_ACCOUNT_KEY)
      localStorage.removeItem('guestAccount')
    } catch (error) {
      console.error('[LoginSettings] Failed to clear guest account:', error)
    }
  }

  /**
   * Check if login settings exist
   */
  static exists(): boolean {
    return localStorage.getItem(STORAGE_KEY) !== null
  }
}
