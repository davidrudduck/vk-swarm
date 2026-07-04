import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ProfileProvider } from '@/components/ProfileProvider'
import AppRouter from './AppRouter'

const queryClient = new QueryClient()

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ProfileProvider>
        <AppRouter />
      </ProfileProvider>
    </QueryClientProvider>
  )
}
