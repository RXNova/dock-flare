import type { AppStatus } from './types';

export function lineStyle(line: string): string {
  if (/error|fatal/i.test(line))         return 'text-error font-medium';
  if (line.startsWith('==='))            return 'text-base-content font-bold';
  if (/\[\d+\/\d+\]/.test(line))         return 'text-info';
  if (/complete|success|\bOK\b|found|acquired|written|deployed/i.test(line))
                                         return 'text-success';
  if (line.trimStart().startsWith('->')) return 'text-base-content/60';
  return 'text-base-content/80';
}

export const statusCfg: Record<AppStatus, { dot: string; label: string; text: string }> = {
  idle:       { dot: 'bg-zinc-500',    label: 'Idle',      text: 'text-zinc-400'    },
  processing: { dot: 'bg-amber-400',   label: 'Deploying', text: 'text-amber-400'   },
  success:    { dot: 'bg-emerald-400', label: 'Success',   text: 'text-emerald-400' },
  failed:     { dot: 'bg-red-400',     label: 'Failed',    text: 'text-red-400'     },
};

export function domainError(d: string): string | null {
  if (!d.trim()) return null;
  if (d.includes(' ')) return 'No spaces allowed';
  const p = d.split('.');
  if (p.length < 2 || p.some(s => s.length === 0)) return 'Enter a full hostname';
  if (!/^[a-zA-Z0-9][a-zA-Z0-9.\-]*[a-zA-Z0-9]$/.test(d)) return 'Invalid characters';
  return null;
}
