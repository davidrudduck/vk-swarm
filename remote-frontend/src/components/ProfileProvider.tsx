import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

import type { ProfileResponse } from '@/types/shared/types';
import { profileApi } from '@/lib/api/profile';

interface ProfileState {
  profile: ProfileResponse | null;
  isSignedIn: boolean;
  isLoaded: boolean;
}

const ProfileContext = createContext<ProfileState | undefined>(undefined);

interface ProfileProviderProps {
  children: ReactNode;
}

export function ProfileProvider({ children }: ProfileProviderProps) {
  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [isSignedIn, setIsSignedIn] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const fetchProfile = async () => {
      try {
        const data = await profileApi.get();
        if (cancelled) return;
        setProfile(data);
        setIsSignedIn(true);
        setIsLoaded(true);
      } catch (err) {
        if (cancelled) return;
        // Clear stale token on 401
        if (err instanceof Error && err.message.includes('401')) {
          localStorage.removeItem('access_token');
        }
        setProfile(null);
        setIsSignedIn(false);
        setIsLoaded(true);
      }
    };

    fetchProfile();

    return () => {
      cancelled = true;
    };
  }, []);

  const value = useMemo<ProfileState>(() => ({
    profile,
    isSignedIn,
    isLoaded,
  }), [profile, isSignedIn, isLoaded]);

  return (
    <ProfileContext.Provider value={value}>
      {children}
    </ProfileContext.Provider>
  );
}

export function useProfile() {
  const context = useContext(ProfileContext);
  if (context === undefined) {
    throw new Error('useProfile must be used within a ProfileProvider');
  }
  return context;
}
