export function revealInScroller(element: globalThis.HTMLElement | null | undefined) {
  const scroller = element?.parentElement
  if (!element || !scroller) return
  const item = element.getBoundingClientRect()
  const bounds = scroller.getBoundingClientRect()
  if (item.top < bounds.top) scroller.scrollTop += item.top - bounds.top
  else if (item.bottom > bounds.bottom) scroller.scrollTop += item.bottom - bounds.bottom
}
