import { useOperation } from '../../composables/useOperation'

// Keep the most recent workspace operation visible when returning to settings.
export const workspaceOperation = useOperation()
