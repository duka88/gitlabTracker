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
import { useAppState } from "../store";

const props = defineProps<{ item: ListItem; tabView: TabView; now: number }>();
const { openUrl } = useAppState();
const mr = computed(() => props.item.mr!);

const STATUS: Record<MergeStatusTag, { text: string; cls: string }> = {
  readyToMerge: { text: "Ready to merge", cls: "ok" },
  approved: { text: "Approved", cls: "approved" },
  needsApproval: { text: "Needs approval", cls: "neutral" },
  ciFailed: { text: "CI failed", cls: "bad" },
  conflicts: { text: "Conflicts", cls: "bad" },
  changesRequested: { text: "Changes requested", cls: "warn" },
  unresolvedThreads: { text: "Unresolved threads", cls: "warn" },
};
const status = computed(() => (mr.value?.mergeStatus ? STATUS[mr.value.mergeStatus] : null));

// FR-036: the ticket this MR resolves, with its workflow state (`state::` label).
const ticket = computed(() => mr.value?.ticket ?? null);
const ticketState = computed(() => ticket.value?.stateLabel?.value ?? ticket.value?.status?.name ?? null);
function openTicket(e: Event) {
  e.stopPropagation();
  if (ticket.value) void openUrl(ticket.value.webUrl);
}

// FR-034: in a tab with a test branch, show the source branch and whether it's current there.
const onBranch = computed(() => {
  const ob = mr.value?.onBranch;
  if (!ob) return null;
  const when = ob.mergedAt ? ` (last merged ${formatDateTime(ob.mergedAt)})` : "";
  return ob.upToDate
    ? { text: `on ${ob.branch}`, cls: "ok", title: `Latest commit is on ${ob.branch}${when}` }
    : { text: `outdated on ${ob.branch}`, cls: "warn", title: `Commits were pushed after it was merged into ${ob.branch}${when}` };
});
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
          <template v-if="ticket">
            <span class="sep" />
            <button
              class="ticket"
              :class="{ ready: ticketState?.toLowerCase() === 'ready to merge', closed: !ticket.open }"
              :style="ticket.stateLabel ? { borderColor: ticket.stateLabel.color } : undefined"
              :title="`${ticket.reference} ${ticket.title}${ticketState ? ` — ${ticketState}` : ''} (open ticket)`"
              @click="openTicket"
            >{{ ticket.reference }}</button>
          </template>
          <span class="sep" />
          <span :title="formatDateTime(item.createdAt)">created {{ relativeTime(item.createdAt, now) }}</span>
          <span v-if="item.author">by {{ item.author.name }}</span>
          <span class="sep" />
          <span v-if="mr.onBranch" class="meta-item src-branch" :title="`${mr.sourceBranch} → ${mr.targetBranch}`"><Icon name="branch" :size="12" /><span class="ellipsis">{{ mr.sourceBranch }}</span></span>
          <span v-else class="meta-item" :title="`Target branch: ${mr.targetBranch}`"><Icon name="branch" :size="12" />{{ mr.targetBranch }}</span>
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
          <button
            v-if="ticket && ticketState"
            class="ticket-state"
            :class="{ closed: !ticket.open }"
            :title="`Ticket ${ticket.reference} ${ticket.title} — ${ticketState} (click to open)`"
            @click="openTicket"
          >
            <LabelChip v-if="ticket.stateLabel" :label="ticket.stateLabel" />
            <span v-else class="mstatus neutral">{{ ticketState }}</span>
          </button>
          <span v-if="onBranch" class="mstatus" :class="onBranch.cls" :title="onBranch.title">{{ onBranch.text }}</span>
          <span v-if="status" class="mstatus" :class="status.cls">{{ status.text }}</span>
        </div>
        <div class="side-line">
          <PipelineIcon v-if="mr.pipeline" :status="mr.pipeline" />
          <span
            class="count"
            :class="{ approved: mr.approvals.approved }"
            :title="mr.approvals.required > 0 ? `${mr.approvals.given} of ${mr.approvals.required} required approvals` : `${mr.approvals.given} approvals`"
          ><Icon name="check" :size="12" />{{ mr.approvals.approved ? "approved" : "not approved" }}<template v-if="mr.approvals.required > 0 || mr.approvals.given > 0"> {{ mr.approvals.given }}<template v-if="mr.approvals.required > 0">/{{ mr.approvals.required }}</template></template></span>
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
.src-branch {
  max-width: 100%;
  min-width: 0;
}
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.ticket {
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: transparent;
  padding: 0 6px;
  line-height: 15px;
  font-size: var(--fs-xs);
  color: var(--text);
  white-space: nowrap;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ticket:hover:not(:disabled) {
  background: var(--bg-hover);
}
.ticket.ready {
  color: var(--green);
  font-weight: 600;
}
.ticket.closed {
  text-decoration: line-through;
  color: var(--text-muted);
}
.ticket-state {
  border: none;
  background: transparent;
  padding: 0;
  display: inline-flex;
  line-height: 0;
}
.ticket-state:hover:not(:disabled) {
  background: transparent;
  filter: brightness(0.95);
}
.ticket-state.closed {
  opacity: 0.55;
}
.mstatus.approved {
  color: var(--green);
  box-shadow: inset 0 0 0 1px var(--green);
}
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
