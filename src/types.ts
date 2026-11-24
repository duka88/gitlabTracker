// Mirrors the Rust view types in src-tauri/src/model (serde camelCase).

export type ListKind = "mergeRequests" | "workItems";

/** Row order in the list view; sorted in the UI. `gitlab` keeps the URL's order. */
export type TabSort = "gitlab" | "label" | "updated" | "created" | "title" | "priority" | "stage" | "approval";

export interface Tab {
  id: string;
  name: string;
  url: string;
  kind: ListKind;
  notifyNewItems: boolean;
  notifyNewComments: boolean;
  notifyChanges: boolean;
  /** MR tabs: only MRs merged into this branch (FR-034). */
  testBranch?: string | null;
  /** Work-item tabs: group rows by month and week (FR-038). */
  groupByWeek?: boolean;
  /** The tab's default row order. */
  sort?: TabSort;
}

export interface UserInfo {
  id: string;
  username: string;
  name: string;
  avatarUrl: string | null;
}

export interface Person {
  username: string;
  name: string;
  /** `avatar://localhost/...` proxy URL; never a GitLab URL. */
  avatarUrl: string | null;
}

export interface Label {
  title: string;
  color: string;
  textColor: string;
  scope: string | null;
  value: string;
}

export type PipelineStatus =
  | "success"
  | "failed"
  | "running"
  | "pending"
  | "canceled"
  | "skipped"
  | "manual"
  | "created"
  | "scheduled"
  | "other";

export type MergeStatusTag =
  | "readyToMerge"
  | "approved"
  | "needsApproval"
  | "ciFailed"
  | "conflicts"
  | "changesRequested"
  | "unresolvedThreads";

export interface Approvals {
  approved: boolean;
  required: number;
  given: number;
}

export interface Reviewer extends Person {
  approved: boolean;
  reviewState: string | null;
}

export interface LinkedTicket {
  reference: string;
  title: string;
  webUrl: string;
  open: boolean;
  stateLabel: Label | null;
  status: { name: string; color: string | null } | null;
}

export interface OnBranch {
  branch: string;
  upToDate: boolean;
  mergedAt: string | null;
}

/** A row is deployed on a tracked env (FR-040). */
export interface EnvBadge {
  env: string;
  /** False when the MR has commits newer than what's deployed. */
  upToDate: boolean;
}

export interface TrackedEnv {
  project: string;
  name: string;
}

export interface EnvDeployment {
  id: number;
  ref: string;
  sha: string;
  finishedAt: string | null;
  user: string | null;
  jobUrl: string | null;
}

export interface EnvStatus {
  env: TrackedEnv;
  webUrl: string | null;
  state: string | null;
  deployment: EnvDeployment | null;
  error: string | null;
  checkedAt: string | null;
}

export interface MrFields {
  draft: boolean;
  targetBranch: string;
  state: string;
  pipeline: PipelineStatus | null;
  approvals: Approvals;
  unresolvedThreads: number;
  sourceBranch: string;
  headSha: string;
  onBranch: OnBranch | null;
  ticket: LinkedTicket | null;
  reviewers: Reviewer[];
  mergeStatus: MergeStatusTag | null;
}

export type Health = "onTrack" | "needsAttention" | "atRisk";

export interface WorkItemFields {
  typeName: string;
  typeIcon: string | null;
  confidential: boolean;
  iteration: string | null;
  /** `YYYY-MM-DD` */
  dueDate: string | null;
  weight: number | null;
  status: { name: string; color: string | null } | null;
  health: Health | null;
  children: { done: number; total: number } | null;
}

export interface ListItem {
  gid: string;
  kind: ListKind;
  iid: string;
  reference: string;
  title: string;
  webUrl: string;
  projectPath: string;
  author: Person | null;
  createdAt: string;
  updatedAt: string;
  /** Work items: when closed (FR-038). */
  closedAt?: string | null;
  labels: Label[];
  assignees: Person[];
  milestone: string | null;
  commentCount: number | null;
  upvotes: number;
  downvotes: number;
  descriptionPreview: string;
  mr?: MrFields;
  workItem?: WorkItemFields;
  /** Tracked envs this item is deployed on (FR-040). */
  envs?: EnvBadge[];
}

export type ChangeKind =
  | { type: "pipelineFailed" }
  | { type: "approved" }
  | { type: "changesRequested"; byUsername: string; byName: string }
  | { type: "merged" }
  | { type: "statusChanged"; from: string | null; to: string | null }
  | { type: "labelsChanged"; added: string[]; removed: string[] };

/** Also the wire form of `AppError`. */
export interface Issue {
  kind: string;
  message: string;
}
export type AppError = Issue;

export interface TabView {
  tab: Tab;
  items: ListItem[];
  itemCount: number;
  newCount: number;
  newItems: string[];
  newComments: Record<string, number>;
  changes: Record<string, ChangeKind[]>;
  error: Issue | null;
  warning: Issue | null;
  truncatedTotal: number | null;
  lastUpdated: string | null;
  loaded: boolean;
}

export interface AppView {
  signedIn: boolean;
  user: UserInfo | null;
  intervalMinutes: number;
  notifyMentions: boolean;
  testBranch: string | null;
  trackedEnvs: TrackedEnv[];
  /** One per tracked env, in Settings order. */
  envs: EnvStatus[];
  polling: boolean;
  lastPollAt: string | null;
  activeTabId: string | null;
  tabs: TabView[];
  menuCount: number;
  bellUnread: number;
  globalError: Issue | null;
}

export type NotificationKind = "newItem" | "newItems" | "comment" | "change" | "mention" | "deploy";

export interface NotificationEntry {
  id: string;
  kind: NotificationKind;
  tabId: string | null;
  itemGid: string | null;
  webUrl: string | null;
  text: string;
  createdAt: string;
  read: boolean;
}

export interface UrlCheck {
  valid: boolean;
  kind: ListKind | null;
  scopeLabel: string | null;
  filtersSummary: string[];
  ignoredParams: string[];
  error: string | null;
}

export type OpenTarget =
  | { type: "row"; tabId: string; gid: string }
  | { type: "tab"; tabId: string }
  | { type: "browser" }
  | { type: "none" };

export interface RowTarget {
  tabId: string;
  gid: string;
}
