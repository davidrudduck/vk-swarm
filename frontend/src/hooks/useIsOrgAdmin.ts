import { useUserOrganizations } from './useUserOrganizations';
import { MemberRole } from 'shared/types';

/**
 * Hook to check if the current user is an admin in any of their organizations.
 *
 * This is used to enable admin-level actions on tasks (like deleting tasks
 * owned by other users). The backend enforces proper authorization, so this
 * is primarily for UI enablement.
 */
export function useIsOrgAdmin(): boolean {
  const { data: orgsData } = useUserOrganizations();

  if (!orgsData?.organizations) {
    return false;
  }

  // Check if user is admin in any of their orgs
  return orgsData.organizations.some(
    (org) => org.user_role === MemberRole.ADMIN
  );
}
