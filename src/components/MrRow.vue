<script setup lang="ts">
// MR row (FR-025, Story 1 scenario 5).
import { computed } from "vue";
import type { ListItem, MergeStatusTag, TabView } from "../types";
import RowShell from "./RowShell.vue";
import LabelChip from "./LabelChip.vue";
import AvatarStack from "./AvatarStack.vue";
import PipelineIcon from "./PipelineIcon.vue";
import Icon from "./Icon.vue";
import { formatDateTime, relativeTime } from "../util/time";

const props = defineProps<{ item: ListItem; tabView: TabView; now: number }>();
const mr = computed(() => props.item.mr!);

const STATUS: Record<MergeStatusTag, { text: string; cls: string }> = {
  readyToMerge: { text: "Ready to merge", cls: "ok" },
  needsApproval: { text: "Needs approval", cls: "neutral" },
  ciFailed: { text: "CI failed", cls: "bad" },
  conflicts: { text: "Conflicts", cls: "bad" },
  changesRequested: { text: "Changes requested", cls: "warn" },
  unresolvedThreads: { text: "Unresolved threads", cls: "warn" },
};
const status = computed(() => (mr.value?.mergeStatus ? STATUS[mr.value.mergeStatus] : null));
</script>

<template>
  <RowShell v-if="mr" :item="item" :tab-view="tabView" v-slot="{ isNew, newComments, changeText }">
    <div class="grid">
      <div class="main">
        <div class="title-line">
          <span v-if="isNew" class="tag new">NEW</span>
          <span v-if="mr.draft" class="tag draft">Draft</span>
          <span class="title">{{ item.title }}</span>
        </div>
        <div class="meta">
          <span>{{ item.reference }}</span>
          <span class="sep" />
          <span :title="formatDateTime(item.createdAt)">created {{ relativeTime(item.createdAt, now) }}</span>
          <span v-if="item.author">by {{ item.author.name }}</span>
          <span class="sep" />
          <span class="meta-item" :title="`Target branch: ${mr.targetBranch}`"><Icon name="branch" :size="12" />{{ mr.targetBranch }}</span>
          <template v-if="item.milestone">
            <span class="sep" />
            <span class="meta-item" title="Milestone"><Icon name="milestone" :size="12" />{{ item.milestone }}</span>
          </template>
        </div>
        <div v-if="item.labels.length" class="labels">
          <LabelChip v-for="l in item.labels" :key="l.title" :label="l" />
        </div>
      </div>
      <div class="side">
        <div class="side-line">
          <span v-if="changeText" class="tag changed" :title="`Changed: ${changeText}`">{{ changeText }}</span>
          <span v-if="status" class="mstatus" :class="status.cls">{{ status.text }}</span>
        </div>
        <div class="side-line">
          <PipelineIcon v-if="mr.pipeline" :status="mr.pipeline" />
          <span
            v-if="mr.approvals.required > 0 || mr.approvals.given > 0"
            class="count"
            :class="{ approved: mr.approvals.approved }"
            :title="mr.approvals.required > 0 ? `${mr.approvals.given} of ${mr.approvals.required} required approvals` : `${mr.approvals.given} approvals`"
          ><Icon name="check" :size="12" />{{ mr.approvals.given }}<template v-if="mr.approvals.required > 0">/{{ mr.approvals.required }}</template></span>
          <AvatarStack :people="item.assignees" label="Assignees" />
          <AvatarStack v-if="mr.reviewers.length" :people="mr.reviewers" label="Reviewers" />
        </div>
        <div class="side-line">
          <span v-if="item.commentCount !== null" class="count" :class="{ hot: newComments > 0 }" :title="`${item.commentCount} comments`">
            <Icon name="comment" :size="12" />{{ item.commentCount }}<template v-if="newComments > 0"> (+{{ newComments }})</template>
          </span>
          <span
            v-if="mr.unresolvedThreads > 0"
            class="count unresolved"
            :title="`${mr.unresolvedThreads} unresolved thread${mr.unresolvedThreads === 1 ? '' : 's'} to address`"
          ><Icon name="comment" :size="12" />{{ mr.unresolvedThreads }} to resolve</span>
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
.count.approved {
  color: var(--green);
}
.mstatus {
  font-size: var(--fs-xs);
  font-weight: 600;
  padding: 0 6px;
  line-height: 16px;
  border-radius: var(--radius-pill);
  white-space: nowrap;
}
.mstatus.ok {
  background: var(--green-bg);
  color: var(--green);
}
.mstatus.bad {
  background: var(--red-bg);
  color: var(--red);
}
.mstatus.warn {
  background: var(--orange-bg);
  color: var(--orange);
}
.mstatus.neutral {
  background: var(--bg-strong);
  color: var(--text-muted);
}
</style>
