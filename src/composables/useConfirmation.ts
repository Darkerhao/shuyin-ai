import { shallowRef } from 'vue'

export const confirmation = shallowRef<{ message: string; returnFocus: globalThis.HTMLElement | null; resolve: (accepted: boolean) => void } | null>(null)

export function requestConfirmation(message: string, returnFocusId?: string): Promise<boolean> {
  // A second action must not inherit the answer to a different confirmation.
  if (confirmation.value) return Promise.resolve(false)
  const returnFocus = returnFocusId ? globalThis.document.getElementById(returnFocusId) : null
  return new Promise(resolve => { confirmation.value = { message, returnFocus, resolve } })
}

export function settleConfirmation(accepted: boolean) {
  const pending = confirmation.value
  confirmation.value = null
  pending?.resolve(accepted)
}
