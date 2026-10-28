import { computed, type WritableComputedRef } from 'vue'
import { useRoute, useRouter } from 'vue-router'

export function useQueryState<T extends string>(key: string, fallback: T, options: readonly T[]): WritableComputedRef<T>
export function useQueryState(key: string, fallback: string): WritableComputedRef<string>
export function useQueryState(key: string, fallback: string, options?: readonly string[]) {
  const route = useRoute()
  const router = useRouter()
  return computed({
    get: () => {
      const value = route.query[key]
      return typeof value === 'string' && (!options || options.includes(value)) ? value : fallback
    },
    set: (value: string) => { void router.replace({ query: { ...route.query, [key]: value === fallback ? undefined : value } }) },
  })
}
