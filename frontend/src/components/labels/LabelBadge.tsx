import { X } from 'lucide-react';
import { cn } from '@/lib/utils';
import { getLucideIcon } from './IconPicker';
import type { Label } from 'shared/types';

interface LabelBadgeProps {
  label: Label;
  size?: 'sm' | 'md';
  /** 'solid' (default) fills with the label colour; 'outline' uses a transparent
   *  background with a coloured border + text (task-card context). */
  variant?: 'solid' | 'outline';
  onClick?: () => void;
  onRemove?: () => void;
  className?: string;
}

// Calculate contrasting text color based on background
function getContrastColor(hexColor: string): string {
  const hex = hexColor.replace('#', '');
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.5 ? '#000000' : '#ffffff';
}

// Derive a text colour for the outline variant on a white/light surface.
// Uses WCAG relative luminance to darken colours that would fail 4.5:1 vs white.
function getOutlineTextColor(hexColor: string): string {
  const hex = hexColor.replace('#', '');
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);
  const lin = (c: number) => {
    const n = c / 255;
    return n <= 0.03928 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
  };
  const L = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  // Target: contrast >= 4.5:1 vs white (L=1.0); max L for text = 0.183
  if (L <= 0.183) return hexColor;
  const scale = Math.sqrt(0.18 / Math.max(L, 0.001));
  const ch = (v: number) =>
    Math.round(Math.min(255, v * scale))
      .toString(16)
      .padStart(2, '0');
  return `#${ch(r)}${ch(g)}${ch(b)}`;
}

export function LabelBadge({
  label,
  size = 'md',
  variant = 'solid',
  onClick,
  onRemove,
  className,
}: LabelBadgeProps) {
  const IconComponent = getLucideIcon(label.icon);
  const textColor = getContrastColor(label.color);

  const sizeClasses = {
    sm: 'text-xs px-1.5 py-0.5 gap-1',
    md: 'text-sm px-2 py-1 gap-1.5',
  };

  const iconSizes = {
    sm: 'h-3 w-3',
    md: 'h-3.5 w-3.5',
  };

  const badge = (
    <span
      className={cn(
        'inline-flex items-center rounded-full font-medium transition-opacity',
        sizeClasses[size],
        variant === 'outline' && 'border',
        onClick && 'cursor-pointer hover:opacity-80',
        className
      )}
      style={
        variant === 'outline'
          ? {
              backgroundColor: 'transparent',
              color: getOutlineTextColor(label.color),
              borderColor: label.color,
            }
          : {
              backgroundColor: label.color,
              color: textColor,
            }
      }
      onClick={onClick}
    >
      {IconComponent && <IconComponent className={iconSizes[size]} />}
      <span>{label.name}</span>
      {onRemove && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
          className="ml-0.5 hover:opacity-70 focus:outline-none"
          aria-label={`Remove ${label.name}`}
        >
          <X className={size === 'sm' ? 'h-2.5 w-2.5' : 'h-3 w-3'} />
        </button>
      )}
    </span>
  );

  return badge;
}
