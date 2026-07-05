import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

import type { ProfileResponse } from '@/types/shared/types';

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
      const token = localStorage.getItem('access_token');

      if (!token) {
        if (!cancelled) {
          setProfile(null);
          setIsSignedIn(false);
          setIsLoaded(true);
        }
        return;
      }

      try {
        const response = await fetch('/v1/profile', {
          headers: {
            Authorization: `Bearer ${token}`,
          },
        });

        if (cancelled) return;

        if (response.ok) {
          const data: ProfileResponse = await response.json();
          setProfile(data);
          setIsSignedIn(true);
          setIsLoaded(true);
        } else if (response.status === 401) {
          localStorage.removeItem('access_token');
          setProfile(null);
          setIsSignedIn(false);
          setIsLoaded(true);
        } else {
          setProfile(null);
          setIsSignedIn(false);
          setIsLoaded(true);
        }
      } catch (err) {
        console.error('Failed to fetch profile:', err);
        if (!cancelled) {
          setProfile(null);
          setIsSignedIn(false);
          setIsLoaded(true);
        }
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
