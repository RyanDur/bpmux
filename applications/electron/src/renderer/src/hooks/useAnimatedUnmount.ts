import { useEffect, useRef, useState } from 'react'

/**
 * Keep a component mounted briefly after a caller sets `open=false`
 * so the CSS exit transition can play before React unmounts the
 * element. Open → `{ mounted: true, visible: true }`. Close →
 * `visible` flips to false immediately (triggers CSS transition),
 * `mounted` flips to false after the duration elapses.
 *
 * Usage:
 *   const { mounted, visible } = useAnimatedUnmount(open, 250)
 *   return mounted ? <Overlay visible={visible} /> : null
 *
 * The Overlay's root element reads `[data-visible]` (or a
 * .is-open / .is-closing class) to apply the enter/exit styles.
 *
 * Under prefers-reduced-motion, callers should still pass the full
 * duration here — the :root token zeroing means the CSS transition
 * resolves instantly, but the hook's timer still waits. That's OK:
 * 250ms of extra-mounted time is invisible to the user and avoids
 * a media-query branch in the hook itself. If it ever matters,
 * reading matchMedia here would short-circuit the timeout.
 */
export function useAnimatedUnmount(
  open: boolean,
  durationMs: number
): { mounted: boolean; visible: boolean } {
  const [mounted, setMounted] = useState<boolean>(open)
  const [visible, setVisible] = useState<boolean>(open)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
    if (open) {
      // Mount first, then flip visible on the next frame so the CSS
      // @starting-style / .is-open toggle can interpolate from the
      // closed state. Double requestAnimationFrame ensures the
      // browser has laid out the mounted element at closed styles
      // before we flip the class.
      setMounted(true)
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          setVisible(true)
        })
      })
    } else {
      // Flip visible off first so CSS exits; after the duration,
      // unmount. The timer is the authoritative source of unmount
      // timing — we don't listen to transitionend because cascaded
      // transitions (opacity + transform with different durations)
      // can fire multiple events unpredictably.
      setVisible(false)
      timerRef.current = setTimeout(() => {
        setMounted(false)
        timerRef.current = null
      }, durationMs)
    }
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current)
        timerRef.current = null
      }
    }
  }, [open, durationMs])

  return { mounted, visible }
}
