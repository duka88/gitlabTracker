//! GraphQL JSON → `ListItem` (research R5, verification results). Pure.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::config::avatar_proxy_url;
use crate::model::{
    Approvals, Children, Health, Label, LinkedTicket, ListItem, ListKind, MergeStatusTag, MrFields,
    Person, PipelineStatus, READY_TO_MERGE_STATE, Reviewer, WorkItemFields, WorkStatus,
};
use crate::text::preview;

pub const DESCRIPTION_PREVIEW_MAX: usize = 300;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersonRaw {
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Nodes<T> {
    pub nodes: Vec<T>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LabelRaw {
    pub title: String,
    pub color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Titled {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PathRaw {
    pub full_path: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InteractionRaw {
    pub approved: bool,
    pub review_state: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewerRaw {
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub merge_request_interaction: Option<InteractionRaw>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PipelineRaw {
    pub status: Option<String>,
}

/// One `workItemRelations` node of an MR.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RelationRaw {
    /// `CLOSES` or `MENTIONED`.
    pub link_type: String,
    pub work_item: Option<LinkedWorkItemRaw>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LinkedWorkItemRaw {
    pub iid: String,
    pub reference: String,
    pub title: String,
    pub web_url: String,
    pub state: String,
    pub widgets: Vec<Value>,
}

/// A user known only by username (e.g. `approvedBy`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UserRef {
    pub username: String,
}

/// One MR node of the `MrFields` fragment.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MrRaw {
    pub id: String,
    pub iid: String,
    pub reference: String,
    pub title: String,
    pub draft: bool,
    pub state: String,
    pub web_url: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub target_branch: String,
    pub source_branch: String,
    pub diff_head_sha: Option<String>,
    pub description: Option<String>,
    pub user_notes_count: Option<u32>,
    pub upvotes: u32,
    pub downvotes: u32,
    pub conflicts: bool,
    pub detailed_merge_status: Option<String>,
    pub approved: bool,
    pub approvals_required: Option<u32>,
    pub approvals_left: Option<u32>,
    pub approved_by: Option<Nodes<UserRef>>,
    pub resolvable_discussions_count: Option<u32>,
    pub resolved_discussions_count: Option<u32>,
    pub work_item_relations: Option<Nodes<RelationRaw>>,
    pub project: Option<PathRaw>,
    pub author: Option<PersonRaw>,
    pub milestone: Option<Titled>,
    pub head_pipeline: Option<PipelineRaw>,
    pub labels: Option<Nodes<LabelRaw>>,
    pub assignees: Option<Nodes<PersonRaw>>,
    pub reviewers: Option<Nodes<ReviewerRaw>>,
}

/// Approvals actually given: the approver list, else `required - left`.
pub fn approvals_given(mr: &MrRaw) -> u32 {
    match &mr.approved_by {
        Some(n) => u32::try_from(n.nodes.len()).unwrap_or(u32::MAX),
        None => mr
            .approvals_required
            .unwrap_or(0)
            .saturating_sub(mr.approvals_left.unwrap_or(0)),
    }
}

/// Approved for real: GitLab's rules are met *and* at least one person approved, even when
/// the project requires no approvals (FR-025a).
pub fn effectively_approved(mr: &MrRaw) -> bool {
    mr.approved && approvals_given(mr) >= mr.approvals_required.unwrap_or(0).max(1)
}

/// Resolvable threads that are still open (FR-025b).
pub fn unresolved_threads(mr: &MrRaw) -> u32 {
    mr.resolvable_discussions_count
        .unwrap_or(0)
        .saturating_sub(mr.resolved_discussions_count.unwrap_or(0))
}

/// The ticket an MR resolves (FR-036): a `CLOSES` link wins; otherwise the linked item whose
/// iid is the leading number of the source branch (`19368-email-…` → `#19368`).
pub fn pick_ticket(mr: &MrRaw) -> Option<LinkedTicket> {
    let rels = mr.work_item_relations.as_ref()?;
    let items = || {
        rels.nodes
            .iter()
            .filter_map(|r| r.work_item.as_ref().map(|w| (r, w)))
    };
    let branch_iid = branch_ticket_iid(&mr.source_branch);
    let (_, w) = items().find(|(r, _)| r.link_type == "CLOSES").or_else(|| {
        branch_iid
            .as_deref()
            .and_then(|iid| items().find(|(_, w)| w.iid == iid))
    })?;
    Some(ticket_from(w))
}

/// Leading digits of a branch name (`19368-email-…` → `19368`), the ticket-number convention.
pub fn branch_ticket_iid(source_branch: &str) -> Option<String> {
    let iid: String = source_branch
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    (!iid.is_empty()).then_some(iid)
}

/// Builds the ticket chip data from a work item node (relation or direct lookup).
pub fn ticket_from(w: &LinkedWorkItemRaw) -> LinkedTicket {
    let labels = widget(&w.widgets, "WorkItemWidgetLabels")
        .and_then(|x| x.pointer("/labels/nodes"))
        .and_then(|n| serde_json::from_value::<Vec<LabelRaw>>(n.clone()).ok())
        .unwrap_or_default();
    let state_label = labels.iter().map(label).find(|l| {
        l.scope
            .as_deref()
            .is_some_and(|s| s.eq_ignore_ascii_case("state"))
    });
    let status = widget(&w.widgets, "WorkItemWidgetStatus")
        .and_then(|x| x.get("status"))
        .filter(|x| !x.is_null())
        .and_then(|s| {
            Some(WorkStatus {
                name: s.get("name")?.as_str()?.to_string(),
                color: s.get("color").and_then(Value::as_str).map(str::to_string),
            })
        });
    LinkedTicket {
        reference: if w.reference.is_empty() {
            format!("#{}", w.iid)
        } else {
            w.reference.clone()
        },
        title: w.title.clone(),
        web_url: w.web_url.clone(),
        open: !w.state.eq_ignore_ascii_case("CLOSED"),
        state_label,
        status,
    }
}

/// True when the ticket's workflow state blocks merging (FR-036): it has a `state::` label
/// other than "Ready to Merge". No ticket or no state label doesn't block.
pub fn ticket_blocks_merge(t: Option<&LinkedTicket>) -> bool {
    t.and_then(|t| t.state_label.as_ref())
        .is_some_and(|l| !l.value.trim().eq_ignore_ascii_case(READY_TO_MERGE_STATE))
}

/// Merge status tag, first match wins (research R5 table, FR-025a, FR-036).
pub fn merge_status(mr: &MrRaw) -> Option<MergeStatusTag> {
    let detailed = mr.detailed_merge_status.as_deref().unwrap_or("");
    let pipeline = mr.head_pipeline.as_ref().and_then(|p| p.status.as_deref());
    if mr.state != "opened" || mr.draft {
        return None;
    }
    if mr.conflicts || detailed == "CONFLICT" {
        return Some(MergeStatusTag::Conflicts);
    }
    if pipeline == Some("FAILED") {
        return Some(MergeStatusTag::CiFailed);
    }
    let changes_requested = mr.reviewers.as_ref().is_some_and(|r| {
        r.nodes.iter().any(|n| {
            n.merge_request_interaction
                .as_ref()
                .and_then(|i| i.review_state.as_deref())
                == Some("REQUESTED_CHANGES")
        })
    });
    if changes_requested || detailed == "REQUESTED_CHANGES" {
        return Some(MergeStatusTag::ChangesRequested);
    }
    if detailed == "DISCUSSIONS_NOT_RESOLVED" || unresolved_threads(mr) > 0 {
        return Some(MergeStatusTag::UnresolvedThreads);
    }
    if !effectively_approved(mr) || detailed == "NOT_APPROVED" {
        return Some(MergeStatusTag::NeedsApproval);
    }
    if detailed == "MERGEABLE" {
        if ticket_blocks_merge(pick_ticket(mr).as_ref()) {
            return Some(MergeStatusTag::Approved);
        }
        return Some(MergeStatusTag::ReadyToMerge);
    }
    None
}

fn person(p: &PersonRaw) -> Person {
    Person {
        username: p.username.clone(),
        name: p.name.clone(),
        avatar_url: p.avatar_url.as_deref().and_then(avatar_proxy_url),
    }
}

fn label(l: &LabelRaw) -> Label {
    Label::new(
        &l.title,
        l.color.as_deref().unwrap_or("#428bca"),
        l.text_color.as_deref().unwrap_or(""),
    )
}

fn nodes<T: Clone>(n: Option<&Nodes<T>>) -> Vec<T> {
    n.map_or_default(|n| n.nodes.clone())
}

pub fn mr_to_item(mr: &MrRaw) -> ListItem {
    let required = mr.approvals_required.unwrap_or(0);
    let pipeline = mr
        .head_pipeline
        .as_ref()
        .and_then(|p| p.status.as_deref())
        .map(PipelineStatus::from_graphql);
    let reviewers = nodes(mr.reviewers.as_ref())
        .iter()
        .map(|r| {
            let i = r.merge_request_interaction.clone().unwrap_or_default();
            Reviewer {
                person: Person {
                    username: r.username.clone(),
                    name: r.name.clone(),
                    avatar_url: r.avatar_url.as_deref().and_then(avatar_proxy_url),
                },
                approved: i.approved,
                review_state: i.review_state,
            }
        })
        .collect();
    let created = mr.created_at.unwrap_or_default();
    ListItem {
        gid: mr.id.clone(),
        kind: ListKind::MergeRequests,
        iid: mr.iid.clone(),
        reference: if mr.reference.is_empty() {
            format!("!{}", mr.iid)
        } else {
            mr.reference.clone()
        },
        title: mr.title.clone(),
        web_url: mr.web_url.clone(),
        project_path: mr.project.as_ref().map_or_default(|p| p.full_path.clone()),
        author: mr.author.as_ref().map(person),
        created_at: created,
        updated_at: mr.updated_at.unwrap_or(created),
        closed_at: None,
        labels: nodes(mr.labels.as_ref()).iter().map(label).collect(),
        assignees: nodes(mr.assignees.as_ref()).iter().map(person).collect(),
        milestone: mr.milestone.as_ref().and_then(|m| m.title.clone()),
        comment_count: mr.user_notes_count,
        upvotes: mr.upvotes,
        downvotes: mr.downvotes,
        description_preview: preview(mr.description.as_deref(), DESCRIPTION_PREVIEW_MAX),
        mr: Some(MrFields {
            draft: mr.draft,
            target_branch: mr.target_branch.clone(),
            state: mr.state.clone(),
            pipeline,
            approvals: Approvals {
                approved: effectively_approved(mr),
                required,
                given: approvals_given(mr),
            },
            unresolved_threads: unresolved_threads(mr),
            source_branch: mr.source_branch.clone(),
            head_sha: mr.diff_head_sha.clone().unwrap_or_default(),
            on_branch: None,
            ticket: pick_ticket(mr),
            reviewers,
            merge_status: merge_status(mr),
        }),
        work_item: None,
        envs: Vec::new(),
    }
}

/// One work-item node of the `WiPage` fragment. Widgets stay as JSON and are read by type.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkItemRaw {
    pub id: String,
    pub iid: String,
    pub reference: String,
    pub title: String,
    pub confidential: bool,
    pub state: String,
    pub web_url: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub user_discussions_count: Option<u32>,
    pub namespace: Option<PathRaw>,
    pub work_item_type: Option<TypeRaw>,
    pub author: Option<PersonRaw>,
    pub widgets: Vec<Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TypeRaw {
    pub name: String,
    pub icon_name: Option<String>,
}

fn widget<'a>(w: &'a [Value], type_name: &str) -> Option<&'a Value> {
    w.iter()
        .find(|v| v.get("__typename").and_then(Value::as_str) == Some(type_name))
}

fn parse_at<T: for<'de> Deserialize<'de>>(v: Option<&Value>) -> Option<T> {
    v.filter(|x| !x.is_null())
        .and_then(|x| serde_json::from_value(x.clone()).ok())
}

pub fn work_item_to_item(wi: &WorkItemRaw) -> ListItem {
    let w = &wi.widgets;
    let assignees: Vec<PersonRaw> = parse_at::<Nodes<PersonRaw>>(
        widget(w, "WorkItemWidgetAssignees").and_then(|x| x.get("assignees")),
    )
    .map_or_default(|n| n.nodes);
    let labels: Vec<LabelRaw> = parse_at::<Nodes<LabelRaw>>(
        widget(w, "WorkItemWidgetLabels").and_then(|x| x.get("labels")),
    )
    .map_or_default(|n| n.nodes);
    let milestone =
        parse_at::<Titled>(widget(w, "WorkItemWidgetMilestone").and_then(|x| x.get("milestone")))
            .and_then(|m| m.title);
    let iteration = widget(w, "WorkItemWidgetIteration")
        .and_then(|x| x.get("iteration"))
        .filter(|x| !x.is_null())
        .and_then(iteration_label);
    let due_date = widget(w, "WorkItemWidgetStartAndDueDate")
        .and_then(|x| x.get("dueDate"))
        .and_then(Value::as_str)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let weight = widget(w, "WorkItemWidgetWeight")
        .and_then(|x| x.get("weight"))
        .and_then(Value::as_u64)
        .map(|v| v as u32);
    let status = widget(w, "WorkItemWidgetStatus")
        .and_then(|x| x.get("status"))
        .filter(|x| !x.is_null())
        .and_then(|s| {
            Some(WorkStatus {
                name: s.get("name")?.as_str()?.to_string(),
                color: s.get("color").and_then(Value::as_str).map(str::to_string),
            })
        });
    let health = widget(w, "WorkItemWidgetHealthStatus")
        .and_then(|x| x.get("healthStatus"))
        .and_then(Value::as_str)
        .and_then(|h| match h {
            "onTrack" => Some(Health::OnTrack),
            "needsAttention" => Some(Health::NeedsAttention),
            "atRisk" => Some(Health::AtRisk),
            _ => None,
        });
    let (upvotes, downvotes) = widget(w, "WorkItemWidgetAwardEmoji").map_or((0, 0), |x| {
        (
            x.get("upvotes").and_then(Value::as_u64).unwrap_or(0) as u32,
            x.get("downvotes").and_then(Value::as_u64).unwrap_or(0) as u32,
        )
    });
    let description = widget(w, "WorkItemWidgetDescription")
        .and_then(|x| x.get("description"))
        .and_then(Value::as_str);
    let children = widget(w, "WorkItemWidgetHierarchy")
        .and_then(|x| x.get("rolledUpCountsByType"))
        .and_then(Value::as_array)
        .map(|types| {
            types.iter().fold(Children { done: 0, total: 0 }, |acc, t| {
                let c = t.get("countsByState");
                let get = |k| {
                    c.and_then(|c| c.get(k))
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32
                };
                Children {
                    done: acc.done + get("closed"),
                    total: acc.total + get("all"),
                }
            })
        })
        .filter(|c| c.total > 0);
    let created = wi.created_at.unwrap_or_default();
    let type_raw = wi.work_item_type.clone().unwrap_or_default();
    ListItem {
        gid: wi.id.clone(),
        kind: ListKind::WorkItems,
        iid: wi.iid.clone(),
        reference: if wi.reference.is_empty() {
            format!("#{}", wi.iid)
        } else {
            wi.reference.clone()
        },
        title: wi.title.clone(),
        web_url: wi.web_url.clone(),
        project_path: wi
            .namespace
            .as_ref()
            .map_or_default(|p| p.full_path.clone()),
        author: wi.author.as_ref().map(person),
        created_at: created,
        updated_at: wi.updated_at.unwrap_or(created),
        closed_at: wi.closed_at,
        labels: labels.iter().map(label).collect(),
        assignees: assignees.iter().map(person).collect(),
        milestone,
        comment_count: wi.user_discussions_count,
        upvotes,
        downvotes,
        description_preview: preview(description, DESCRIPTION_PREVIEW_MAX),
        mr: None,
        work_item: Some(WorkItemFields {
            type_name: if type_raw.name.is_empty() {
                "Issue".into()
            } else {
                type_raw.name
            },
            type_icon: type_raw.icon_name,
            confidential: wi.confidential,
            iteration,
            due_date,
            weight,
            status,
            health,
            children,
        }),
        envs: Vec::new(),
    }
}

/// "Sprint 12" or, for untitled iterations, "<cadence> · Nov 3 – Nov 16".
fn iteration_label(it: &Value) -> Option<String> {
    if let Some(t) = it
        .get("title")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        return Some(t.to_string());
    }
    let date = |k: &str| {
        it.get(k)
            .and_then(Value::as_str)
            .and_then(|s| s.get(..10))
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| d.format("%b %-d").to_string())
    };
    let range = match (date("startDate"), date("dueDate")) {
        (Some(a), Some(b)) => format!("{a} – {b}"),
        _ => return None,
    };
    match it
        .get("iterationCadence")
        .and_then(|c| c.get("title"))
        .and_then(Value::as_str)
    {
        Some(c) if !c.is_empty() => Some(format!("{c} · {range}")),
        _ => Some(range),
    }
}

/// Parses a `...Page` connection value (`{count, pageInfo, nodes}`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Page<T> {
    pub count: Option<u32>,
    pub page_info: PageInfo,
    pub nodes: Vec<T>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

pub fn parse_mr_page(conn: &Value) -> Result<(Vec<ListItem>, PageInfo, Option<u32>), String> {
    let page: Page<MrRaw> = serde_json::from_value(conn.clone()).map_err(|e| e.to_string())?;
    Ok((
        page.nodes.iter().map(mr_to_item).collect(),
        page.page_info,
        page.count,
    ))
}

pub fn parse_work_item_page(
    conn: &Value,
) -> Result<(Vec<ListItem>, PageInfo, Option<u32>), String> {
    let page: Page<WorkItemRaw> =
        serde_json::from_value(conn.clone()).map_err(|e| e.to_string())?;
    Ok((
        page.nodes.iter().map(work_item_to_item).collect(),
        page.page_info,
        page.count,
    ))
}
