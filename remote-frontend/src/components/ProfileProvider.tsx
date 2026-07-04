import {
  createContext,
  ReactNode,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';

export interface ProviderProfile {
  provider: string;
  username: string | null;
  display_name: string | null;
  email: string | null;
  avatar_url: string | null;
}

export interface ProfileResponse {
  user_id: string;
  username: string | null;
  email: string;
  providers: ProviderProfile[];
}

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
        const response = await fetch('/v1/profile', {
          credentials: 'include',
        });

        if (cancelled) return;

        if (response.ok) {
          const data: ProfileResponse = await response.json();
          setProfile(data);
          setIsSignedIn(true);
        } else {
          setProfile(null);
          setIsSignedIn(false);
        }
      } catch (err) {
        console.error('Failed to fetch profile:', err);
        setProfile(null);
        setIsSignedIn(false);
      } finally {
        if (!cancelled) {
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
