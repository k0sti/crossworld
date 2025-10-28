/**
 * Tagged logging system
 * Allows toggling log categories on/off
 */

export type LogTag =
  | 'common'
  | 'avatar'
  | 'geometry'
  | 'renderer'
  | 'voice'
  | 'storage'
  | 'network'
  | 'ui'
  | 'worker'
  | 'profile'
  | 'service';

interface LogConfig {
  enabled: Set<LogTag>;
  masterEnabled: boolean;
  listeners: Set<() => void>;
}

const config: LogConfig = {
  enabled: new Set<LogTag>(['common']), // common enabled by default
  masterEnabled: true, // master toggle enabled by default
  listeners: new Set(),
};

// Load saved preferences from localStorage
const STORAGE_KEY = 'crossworld:log-config';

function loadConfig(): void {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const parsed = JSON.parse(saved);
      if (Array.isArray(parsed.enabled)) {
        config.enabled = new Set(parsed.enabled);
      }
      if (typeof parsed.masterEnabled === 'boolean') {
        config.masterEnabled = parsed.masterEnabled;
      }
    }
  } catch (err) {
    console.error('Failed to load log config:', err);
  }
}

function saveConfig(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({
      enabled: Array.from(config.enabled),
      masterEnabled: config.masterEnabled,
    }));
  } catch (err) {
    console.error('Failed to save log config:', err);
  }
}

// Load config on module initialization
loadConfig();

/**
 * Check if master logging is enabled
 */
export function isMasterLogEnabled(): boolean {
  return config.masterEnabled;
}

/**
 * Enable or disable master logging
 */
export function setMasterLogEnabled(enabled: boolean): void {
  config.masterEnabled = enabled;
  saveConfig();
  notifyListeners();
}

/**
 * Check if a log tag is enabled
 */
export function isLogEnabled(tag: LogTag): boolean {
  return config.masterEnabled && config.enabled.has(tag);
}

/**
 * Enable or disable a log tag
 */
export function setLogEnabled(tag: LogTag, enabled: boolean): void {
  if (enabled) {
    config.enabled.add(tag);
  } else {
    config.enabled.delete(tag);
  }
  saveConfig();
  notifyListeners();
}

/**
 * Get all enabled tags
 */
export function getEnabledTags(): Set<LogTag> {
  return new Set(config.enabled);
}

/**
 * Subscribe to log config changes
 */
export function subscribeToLogConfig(listener: () => void): () => void {
  config.listeners.add(listener);
  return () => {
    config.listeners.delete(listener);
  };
}

function notifyListeners(): void {
  config.listeners.forEach(listener => listener());
}

/**
 * Tagged log function
 */
export function log(tag: LogTag, ...args: any[]): void {
  if (config.masterEnabled && config.enabled.has(tag)) {
    console.log(`[${tag}]`, ...args);
  }
}

/**
 * Tagged warn function
 */
export function warn(tag: LogTag, ...args: any[]): void {
  if (config.masterEnabled && config.enabled.has(tag)) {
    console.warn(`[${tag}]`, ...args);
  }
}

/**
 * Tagged error function (always shown when master is enabled)
 */
export function error(tag: LogTag, ...args: any[]): void {
  if (config.masterEnabled) {
    console.error(`[${tag}]`, ...args);
  }
}
