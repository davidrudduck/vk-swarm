import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ProfileProvider } from '@/components/ProfileProvider'
import AppRouter from './AppRouter'
import { Toaster } from 'sonner'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000 },
  },
})

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ProfileProvider>
        <Toaster richColors position="bottom-right" />
        <AppRouter />
      </ProfileProvider>
    </QueryClientProvider>
  )
}
