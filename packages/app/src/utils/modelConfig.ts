import * as logger from './logger';
export interface ModelConfig {
  glb: [string, string][];
  vox: [string, string][];
}

let modelsConfigCache: ModelConfig | null = null;

export async function loadModelsConfig(): Promise<ModelConfig> {
  if (modelsConfigCache) {
    return modelsConfigCache;
  }

  try {
    const response = await fetch(`${import.meta.env.BASE_URL}assets/models.json`);
    if (!response.ok) {
      throw new Error(`Failed to load models.json: ${response.status}`);
    }
    modelsConfigCache = await response.json();
    return modelsConfigCache!;
  } catch (error) {
    logger.error('common', 'Failed to load models config:', error);
    // Return empty config as fallback
    modelsConfigCache = { glb: [], vox: [] };
    return modelsConfigCache;
  }
}

export function getModelFilename(modelId: string, type: 'vox' | 'glb'): string | null {
  if (!modelsConfigCache) {
    // If not loaded yet, return null
    return null;
  }

  const models = type === 'vox' ? modelsConfigCache.vox : modelsConfigCache.glb;

  // Find by ID (filename without extension)
  for (const [_label, filename] of models) {
    const id = filename.replace(`.${type}`, '');
    if (id === modelId) {
      return filename;
    }
  }

  return null;
}

export function getModelUrl(modelId: string, type: 'vox' | 'glb'): string | null {
  const filename = getModelFilename(modelId, type);
  if (!filename) return null;

  return `${import.meta.env.BASE_URL}assets/models/${type}/${filename}`;
}
