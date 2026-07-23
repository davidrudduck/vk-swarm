import { useState, useEffect } from 'react';

export type Breakpoint = 'mobile' | 'tablet' | 'desktop';

export function useBreakpoint(): Breakpoint {
  const get = (): Breakpoint =>
    typeof window === 'undefined'
      ? 'desktop'
      : window.innerWidth < 640
        ? 'mobile'
        : window.innerWidth < 1024
          ? 'tablet'
          : 'desktop';
  const [bp, setBp] = useState<Breakpoint>(get);
  useEffect(() => {
    const on = () => setBp(get());
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  }, []);
  return bp;
}
