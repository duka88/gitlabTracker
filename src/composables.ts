// Small shared composables.
import { onBeforeUnmount, onMounted, ref } from "vue";

/** A `Date.now()` that ticks every `ms` (default 30 s), so relative times stay fresh. */
export function useNow(ms = 30_000) {
  const now = ref(Date.now());
  let timer: ReturnType<typeof setInterval> | undefined;
  onMounted(() => {
    timer = setInterval(() => (now.value = Date.now()), ms);
  });
  onBeforeUnmount(() => clearInterval(timer));
  return now;
}
