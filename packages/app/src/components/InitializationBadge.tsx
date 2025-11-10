/**
 * InitializationBadge - Compact visual indicator for initialization sub-components
 */
import { memo } from 'react';
import type { SubComponentStatus } from '../initialization/types';

interface InitializationBadgeProps {
  status: SubComponentStatus;
}

const colorMap = {
  purple: {
    bg: 'bg-purple-900/80',
    text: 'text-purple-200',
    activeBg: 'bg-purple-600',
    activeText: 'text-white',
  },
  blue: {
    bg: 'bg-blue-900/80',
    text: 'text-blue-200',
    activeBg: 'bg-blue-600',
    activeText: 'text-white',
  },
  green: {
    bg: 'bg-green-900/80',
    text: 'text-green-200',
    activeBg: 'bg-green-600',
    activeText: 'text-white',
  },
  orange: {
    bg: 'bg-orange-900/80',
    text: 'text-orange-200',
    activeBg: 'bg-orange-600',
    activeText: 'text-white',
  },
  cyan: {
    bg: 'bg-cyan-900/80',
    text: 'text-cyan-200',
    activeBg: 'bg-cyan-600',
    activeText: 'text-white',
  },
  red: {
    bg: 'bg-red-900/80',
    text: 'text-red-200',
    activeBg: 'bg-red-600',
    activeText: 'text-white',
  },
};

export const InitializationBadge = memo(({ status }: InitializationBadgeProps) => {
  const colors = colorMap[status.color];
  const isActive = status.status === 'loading' || status.status === 'complete';
  const isComplete = status.status === 'complete';
  const isError = status.status === 'error';

  const bgClass = isActive ? colors.activeBg : colors.bg;
  const textClass = isActive ? colors.activeText : colors.text;

  return (
    <div
      className={`
        ${bgClass} ${textClass}
        px-3 py-1 rounded-full
        text-xs font-medium
        transition-all duration-300
        flex items-center gap-2
        ${isActive ? 'opacity-100' : 'opacity-50'}
      `}
      title={status.message || status.name}
    >
      {/* Spinner for loading state */}
      {status.status === 'loading' && (
        <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin" />
      )}

      {/* Checkmark for complete state */}
      {isComplete && (
        <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
          <path
            fillRule="evenodd"
            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
            clipRule="evenodd"
          />
        </svg>
      )}

      {/* Error indicator */}
      {isError && (
        <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
          <path
            fillRule="evenodd"
            d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
            clipRule="evenodd"
          />
        </svg>
      )}

      {/* Component name */}
      <span className="uppercase tracking-wide">{status.name}</span>

      {/* Progress percentage if loading and available */}
      {status.status === 'loading' && status.progress > 0 && status.progress < 100 && (
        <span className="opacity-75">{Math.round(status.progress)}%</span>
      )}
    </div>
  );
});

InitializationBadge.displayName = 'InitializationBadge';
