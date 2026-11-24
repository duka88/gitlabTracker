<script setup lang="ts">
// Work-item row (FR-026, Story 1 scenario 6). Fields the item doesn't have are left out.
import { computed } from "vue";
import type { Health, ListItem, TabView } from "../types";
import RowShell from "./RowShell.vue";
import LabelChip from "./LabelChip.vue";
import AvatarStack from "./AvatarStack.vue";
import TypeIcon from "./TypeIcon.vue";
import StatusDot from "./StatusDot.vue";
import Icon from "./Icon.vue";
import EnvChips from "./EnvChips.vue";
import { formatDateTime, formatDueDate, isOverdue, relativeTime } from "../util/time";

const props = defineProps<{ item: ListItem; tabView: TabView; now: number }>();
const wi = computed(() => props.item.workItem!);
const today = computed(() => new Date(props.now));

const HEALTH: Record<Health, { text: string; cls: string }> = {
  onTrack: { text: "On track", cls: "ok" },
  needsAttention: { text: "Needs attention", cls: "warn" },
  atRisk: { text: "At risk", cls: "bad" },
};
</script>

<template>
  <RowShell v-if="wi" :item="item" :tab-view="tabView" v-slot="{ isNew, newComments, changeText }">
    <div class="grid">
      <div class="main">
        <div class="title-line">
          <span v-if="isNew" class="tag new">NEW</span>
          <TypeIcon :type-name="wi.typeName" :icon-name="wi.typeIcon" />
          <span class="title">{{ item.title }}</span>
          <span v-if="wi.confidential" class="lock" title="Confidential"><Icon name="lock" :size="12" /></span>
        </div>
        <div class="meta">
          <span>{{ item.reference }}</span>
          <span class="sep" />
          <span :title="formatDateTime(item.createdAt)">created {{ relativeTime(item.createdAt, now) }}</span>
          <span v-if="item.author">by {{ item.author.name }}</span>
          <template v-if="item.milestone">
            <span class="sep" />
            <span class="meta-item" title="Milestone"><Icon name="milestone" :size="12" />{{ item.milestone }}</span>
          </template>
          <template v-if="wi.iteration">
            <span class="sep" />
            <span class="meta-item" title="Iteration"><Icon name="iteration" :size="12" />{{ wi.iteration }}</span>
          </template>
          <template v-if="wi.dueDate">
            <span class="sep" />
            <span
              class="meta-item"
              :class="{ overdue: isOverdue(wi.dueDate, today) }"
              :title="isOverdue(wi.dueDate, today) ? `Overdue (due ${wi.dueDate})` : `Due ${wi.dueDate}`"
            ><Icon name="clock" :size="12" />{{ formatDueDate(wi.dueDate, today) }}</span>
          </template>
          <template v-if="wi.weight !== null">
            <span class="sep" />
            <span class="meta-item" title="Weight"><Icon name="weight" :size="12" />{{ wi.weight }}</span>
          </template>
        </div>
        <div v-if="wi.status || item.labels.length" class="labels">
          <StatusDot v-if="wi.status" :name="wi.status.name" :color="wi.status.color" />
          <LabelChip v-for="l in item.labels" :key="l.title" :label="l" />
        </div>
      </div>
      <div class="side">
        <div v-if="changeText || item.envs?.length" class="side-line">
          <span v-if="changeText" class="tag changed" :title="`Changed: ${changeText}`">{{ changeText }}</span>
          <EnvChips :envs="item.envs" />
        </div>
        <div class="side-line">
          <span v-if="wi.health" class="health" :class="HEALTH[wi.health].cls">{{ HEALTH[wi.health].text }}</span>
          <span v-if="wi.children && wi.children.total > 0" class="count" :title="`${wi.children.done} of ${wi.children.total} child items closed`">
            <Icon name="child" :size="12" />{{ wi.children.done }}/{{ wi.children.total }}
          </span>
          <AvatarStack :people="item.assignees" label="Assignees" />
        </div>
        <div class="side-line">
          <span v-if="item.commentCount !== null" class="count" :class="{ hot: newComments > 0 }" :title="`${item.commentCount} threads`">
            <Icon name="comment" :size="12" />{{ item.commentCount }}<template v-if="newComments > 0"> (+{{ newComments }})</template>
          </span>
          <span v-if="item.upvotes" class="count" title="Thumbs up"><Icon name="thumb-up" :size="12" />{{ item.upvotes }}</span>
          <span v-if="item.downvotes" class="count" title="Thumbs down"><Icon name="thumb-down" :size="12" />{{ item.downvotes }}</span>
          <span class="updated" :title="formatDateTime(item.updatedAt)">updated {{ relativeTime(item.updatedAt, now) }}</span>
        </div>
      </div>
    </div>
  </RowShell>
</template>

<style scoped src="./rowParts.css"></style>
<style scoped>
.lock {
  color: var(--orange);
  display: inline-flex;
}
.health {
  font-size: var(--fs-xs);
  font-weight: 600;
  padding: 0 6px;
  line-height: 16px;
  border-radius: var(--radius-pill);
  white-space: nowrap;
}
.health.ok {
  background: var(--green-bg);
  color: var(--green);
}
.health.warn {
  background: var(--orange-bg);
  color: var(--orange);
}
.health.bad {
  background: var(--red-bg);
  color: var(--red);
}
</style>
