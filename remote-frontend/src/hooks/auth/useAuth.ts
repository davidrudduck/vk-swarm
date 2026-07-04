import { useProfile } from '@/components/ProfileProvider';

export function useAuth() {
  const profileState = useProfile();

  return {
    isSignedIn: profileState.isSignedIn,
    isLoaded: profileState.isLoaded,
    userId: profileState.profile?.user_id ?? null,
  };
}
