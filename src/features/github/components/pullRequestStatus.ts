import type { PullRequest } from "@/types";

export type PullRequestBadgeColor = "neutral" | "info" | "success" | "warning" | "error" | "primary";

export interface PullRequestStatus {
  label: string;
  color: PullRequestBadgeColor;
  icon: string;
}

/** Merged/closed first, then draft, then the review decision of an open PR. */
export function pullRequestStatus(pr: PullRequest): PullRequestStatus {
  if (pr.state === "merged") return { label: "Merged", color: "primary", icon: "i-lucide-git-merge" };
  if (pr.state === "closed") {
    return { label: "Closed", color: "error", icon: "i-lucide-git-pull-request-closed" };
  }
  if (pr.isDraft) {
    return { label: "Draft", color: "neutral", icon: "i-lucide-git-pull-request-draft" };
  }
  switch (pr.reviewDecision) {
    case "approved":
      return { label: "Approved", color: "success", icon: "i-lucide-git-pull-request" };
    case "changes_requested":
      return { label: "Changes requested", color: "warning", icon: "i-lucide-git-pull-request" };
    default:
      return { label: "In review", color: "info", icon: "i-lucide-git-pull-request" };
  }
}
