import { Link, useLocation, useSearchParams } from 'react-router-dom';
import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  FolderOpen,
  Settings,
  BookOpen,
  MessageCircleQuestion,
  Menu,
  LogOut,
  LogIn,
  Archive,
  Activity,
  Search,
} from 'lucide-react';
import { VKSLogo } from '@/components/VKSLogo';
import { SearchBar } from '@/components/SearchBar';
import { ActivityFeed } from '@/components/activity';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useSearch } from '@/contexts/SearchContext';
import { openTaskForm } from '@/lib/openTaskForm';
import { useProject } from '@/contexts/ProjectContext';
import { useOpenProjectInEditor } from '@/hooks/useOpenProjectInEditor';
import { OpenInIdeButton } from '@/components/ide/OpenInIdeButton';
import { useTranslation } from 'react-i18next';
import { OAuthDialog } from '@/components/dialogs/global/OAuthDialog';
import { useUserSystem } from '@/components/ConfigProvider';
import { oauthApi } from '@/lib/api';
import { ProjectSwitcher } from './ProjectSwitcher';
import ThemeToggle from '@/components/ThemeToggle';

const INTERNAL_NAV = [
  { label: 'Projects', icon: FolderOpen, to: '/projects' },
  { label: 'Processes', icon: Activity, to: '/processes' },
];

const EXTERNAL_LINKS = [
  {
    label: 'Docs',
    icon: BookOpen,
    href: 'https://vibekanban.com/docs',
  },
  {
    label: 'Support',
    icon: MessageCircleQuestion,
    href: 'https://github.com/BloopAI/vibe-kanban/issues',
  },
];

function NavDivider() {
  return (
    <div
      className="mx-2 h-6 w-px bg-border/60"
      role="separator"
      aria-orientation="vertical"
    />
  );
}

export function Navbar() {
  const location = useLocation();
  const [searchParams, setSearchParams] = useSearchParams();
  const { projectId, project } = useProject();
  const { query, setQuery, active, clear, registerInputRef } = useSearch();
  const handleOpenInEditor = useOpenProjectInEditor(project || null);
  const { loginStatus, reloadSystem } = useUserSystem();
  const [mobileSearchOpen, setMobileSearchOpen] = useState(false);

  // Archive filter state from URL params
  const showArchived = searchParams.get('archived') === 'on';
  const toggleShowArchived = useCallback(() => {
    const params = new URLSearchParams(searchParams);
    if (showArchived) {
      params.delete('archived');
    } else {
      params.set('archived', 'on');
    }
    setSearchParams(params, { replace: true });
  }, [searchParams, setSearchParams, showArchived]);

  const setSearchBarRef = useCallback(
    (node: HTMLInputElement | null) => {
      registerInputRef(node);
    },
    [registerInputRef]
  );
  const { t } = useTranslation(['tasks', 'common']);

  const handleCreateTask = () => {
    if (projectId) {
      openTaskForm({ mode: 'create', projectId });
    }
  };

  const handleOpenInIDE = () => {
    handleOpenInEditor();
  };

  const handleOpenOAuth = async () => {
    const profile = await OAuthDialog.show();
    if (profile) {
      await reloadSystem();
    }
  };

  const handleOAuthLogout = async () => {
    try {
      await oauthApi.logout();
      await reloadSystem();
    } catch (err) {
      console.error('Error logging out:', err);
    }
  };

  const isOAuthLoggedIn = loginStatus?.status === 'loggedin';

  // Persist the active project so the Board tab can route back to it.
  useEffect(() => {
    if (projectId) {
      localStorage.setItem('lastVisitedProjectId', projectId);
    }
  }, [projectId]);

  // Board tab target: last-visited project's task board, else the projects list.
  const lastVisitedProjectId =
    typeof window !== 'undefined'
      ? localStorage.getItem('lastVisitedProjectId')
      : null;
  const boardProjectId = projectId ?? lastVisitedProjectId;
  const boardTo = boardProjectId
    ? `/projects/${boardProjectId}/tasks`
    : '/projects';

  return (
    <div className="border-b bg-background">
      <div className="w-full px-3">
        <div className="flex items-center h-12 py-2">
          <div className="flex-1 flex items-center">
            <Link to="/projects" className="shrink-0">
              <VKSLogo className="text-sm sm:text-base" />
            </Link>
            <ProjectSwitcher className="ml-2 hidden sm:inline-flex" />
          </div>

          {/* Mobile search button - visible on small screens only */}
          {active && (
            <Button
              variant="ghost"
              size="icon"
              className="h-9 w-9 sm:hidden shrink-0"
              onClick={() => setMobileSearchOpen(true)}
              aria-label={t('common:search', 'Search')}
            >
              <Search className="h-4 w-4" />
            </Button>
          )}

          {/* Desktop search bar and archive toggle */}
          <div className="hidden sm:flex items-center gap-2">
            <SearchBar
              ref={setSearchBarRef}
              className="shrink-0"
              value={query}
              onChange={setQuery}
              disabled={!active}
              onClear={clear}
              project={project || null}
            />
            {projectId && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant={showArchived ? 'default' : 'ghost'}
                      size="icon"
                      className="h-8 w-8 shrink-0"
                      onClick={toggleShowArchived}
                      aria-label={t('filters.archivedToggleAria')}
                      aria-pressed={showArchived}
                    >
                      <Archive className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom">
                    {t('filters.archivedToggleTooltip')}
                  </TooltipContent>
                </Tooltip>
              </TooltipProvider>
            )}
          </div>

          <div className="flex flex-1 items-center justify-end gap-1">
            {projectId ? (
              <>
                {/* OpenInIdeButton - hidden on mobile to prevent overflow */}
                <OpenInIdeButton
                  onClick={handleOpenInIDE}
                  className="h-9 w-9 hidden sm:inline-flex"
                />
                {/* Task creation button - text label for discoverability */}
                <Button
                  variant="default"
                  size="sm"
                  onClick={handleCreateTask}
                  aria-label="Create new task"
                >
                  {/* TODO(i18n): vk-swarm-node-ui-localize */}
                  + Task
                </Button>
                <NavDivider />
              </>
            ) : null}

            <div className="flex items-center gap-1">
              <ThemeToggle />
              <ActivityFeed />

              <Button
                variant="ghost"
                size="icon"
                className="h-9 w-9"
                asChild
                aria-label="Settings"
              >
                <Link
                  to={
                    projectId
                      ? `/settings/projects?projectId=${projectId}`
                      : '/settings'
                  }
                >
                  <Settings className="h-4 w-4" />
                </Link>
              </Button>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-9 w-9"
                    aria-label="Main navigation"
                  >
                    <Menu className="h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>

                <DropdownMenuContent align="end">
                  {/* Archive toggle - visible only on mobile (sm:hidden in CSS) */}
                  {projectId && (
                    <>
                      <DropdownMenuItem
                        className="sm:hidden"
                        onSelect={toggleShowArchived}
                      >
                        <Archive className="mr-2 h-4 w-4" />
                        {showArchived
                          ? t('actionsMenu.unarchive', 'Hide archived')
                          : t('actionsMenu.archive', 'Show archived')}
                      </DropdownMenuItem>
                      <DropdownMenuSeparator className="sm:hidden" />
                    </>
                  )}

                  {INTERNAL_NAV.map((item) => {
                    const active = location.pathname.startsWith(item.to);
                    const Icon = item.icon;
                    return (
                      <DropdownMenuItem
                        key={item.to}
                        asChild
                        className={active ? 'bg-accent' : ''}
                      >
                        <Link to={item.to}>
                          <Icon className="mr-2 h-4 w-4" />
                          {item.label}
                        </Link>
                      </DropdownMenuItem>
                    );
                  })}

                  <DropdownMenuSeparator />

                  {EXTERNAL_LINKS.map((item) => {
                    const Icon = item.icon;
                    return (
                      <DropdownMenuItem key={item.href} asChild>
                        <a
                          href={item.href}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          <Icon className="mr-2 h-4 w-4" />
                          {item.label}
                        </a>
                      </DropdownMenuItem>
                    );
                  })}

                  <DropdownMenuSeparator />

                  {isOAuthLoggedIn ? (
                    <DropdownMenuItem onSelect={handleOAuthLogout}>
                      <LogOut className="mr-2 h-4 w-4" />
                      {t('common:signOut')}
                    </DropdownMenuItem>
                  ) : (
                    <DropdownMenuItem onSelect={handleOpenOAuth}>
                      <LogIn className="mr-2 h-4 w-4" />
                      Sign in
                    </DropdownMenuItem>
                  )}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>
        </div>

        {/* Second nav row: primary section tabs */}
        <nav className="flex items-center gap-4 h-9 border-t text-sm">
          {/* TODO(i18n): vk-swarm-node-ui-localize */}
          <Link
            to={boardTo}
            className={
              location.pathname.startsWith('/projects')
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Board
          </Link>
          <Link
            to="/nodes"
            className={
              location.pathname === '/nodes'
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Nodes
          </Link>
          <Link
            to="/processes"
            className={
              location.pathname === '/processes'
                ? 'mb-[-1px] border-b-2 border-primary py-2 text-foreground'
                : 'mb-[-1px] py-2 text-muted-foreground hover:text-foreground'
            }
          >
            {/* TODO(i18n): vk-swarm-node-ui-localize */}
            Processes
          </Link>
        </nav>
      </div>

      {/* Mobile search dialog */}
      <Dialog open={mobileSearchOpen} onOpenChange={setMobileSearchOpen}>
        <DialogHeader>
          <DialogTitle className="sr-only">
            {t('common:search', 'Search')}
          </DialogTitle>
        </DialogHeader>
        <DialogContent>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={project ? `Search ${project.name}...` : 'Search...'}
              className="pl-10 h-10"
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  setMobileSearchOpen(false);
                }
              }}
            />
          </div>
          {query && (
            <Button
              variant="ghost"
              size="sm"
              className="w-full mt-2"
              onClick={() => {
                clear();
                setMobileSearchOpen(false);
              }}
            >
              {t('common:buttons.reset', 'Clear search')}
            </Button>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
