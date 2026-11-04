<script setup lang="ts">
// Avatars via the avatar:// scheme only (research R16). Falls back to an initials circle.
import { computed, reactive } from "vue";
import type { Person } from "../types";
import { hashHue, initials } from "../util/text";

type P = Person & { approved?: boolean };
const props = withDefaults(defineProps<{ people: P[]; max?: number; label?: string; size?: number }>(), {
  max: 4,
  label: "",
  size: 20,
});

const failed = reactive(new Set<string>());
const shown = computed(() => props.people.slice(0, props.max));
const extra = computed(() => props.people.length - shown.value.length);
const tooltip = computed(() => {
  const names = props.people.map((p) => (p.approved ? `${p.name} ✓ approved` : p.name)).join(", ");
  return props.label ? `${props.label}: ${names}` : names;
});
</script>

<template>
  <span v-if="people.length" class="stack" :title="tooltip">
    <span v-for="(p, i) in shown" :key="`${p.username}-${i}`" class="av" :style="{ width: size + 'px', height: size + 'px' }">
      <img
        v-if="p.avatarUrl && !failed.has(p.username)"
        :src="p.avatarUrl"
        :alt="p.name"
        loading="lazy"
        draggable="false"
        @error="failed.add(p.username)"
      />
      <span v-else class="initials" :style="{ background: `hsl(${hashHue(p.username)} 45% 55%)`, fontSize: size * 0.42 + 'px' }">
        {{ initials(p.name, p.username[0]?.toUpperCase()) }}
      </span>
      <span v-if="p.approved" class="ok" aria-label="approved">✓</span>
    </span>
    <span v-if="extra > 0" class="more">+{{ extra }}</span>
  </span>
</template>

<style scoped>
.stack {
  display: inline-flex;
  align-items: center;
}
.av {
  position: relative;
  display: inline-block;
  border-radius: 50%;
  margin-left: -5px;
  box-shadow: 0 0 0 1.5px var(--bg);
  background: var(--bg-strong);
}
.av:first-child {
  margin-left: 0;
}
.av img,
.initials {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  object-fit: cover;
  color: #fff;
  font-weight: 600;
}
.ok {
  position: absolute;
  right: -3px;
  bottom: -3px;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: var(--green);
  color: #fff;
  font-size: 8px;
  line-height: 11px;
  text-align: center;
  box-shadow: 0 0 0 1px var(--bg);
}
.more {
  margin-left: 3px;
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
