import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api/organizations';
import { useProfile } from '@/components/ProfileProvider';
import type { Organization } from '@/types/shared/types';

export function useOrganizations() {
  const { isSignedIn } = useProfile();

  return useQuery<Organization[]>({
    queryKey: ['user', 'organizations'],
    queryFn: () => organizationsApi.list(),
    enabled: isSignedIn,
    staleTime: 5 * 60 * 1000,
  });
}
