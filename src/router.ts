import { createRouter, createWebHashHistory } from "vue-router";

/**
 * Top-level views. Hash history keeps routing independent of how Tauri serves
 * `index.html` (no server-side fallback needed in production builds).
 */
export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", redirect: "/workstreams" },
    {
      path: "/workstreams",
      name: "workstreams",
      component: () => import("@/pages/WorkstreamsPage.vue"),
    },
    {
      path: "/workstreams/:id(\\d+)",
      name: "workstream",
      component: () => import("@/pages/WorkstreamPage.vue"),
      props: (route) => ({ id: Number(route.params.id) }),
    },
    { path: "/jira", name: "jira", component: () => import("@/pages/JiraIssuesPage.vue") },
    {
      path: "/pull-requests",
      name: "pull-requests",
      component: () => import("@/pages/PullRequestsPage.vue"),
    },
    { path: "/todos", name: "todos", component: () => import("@/pages/TodosPage.vue") },
    { path: "/settings", name: "settings", component: () => import("@/pages/SettingsPage.vue") },
    { path: "/:pathMatch(.*)*", redirect: "/workstreams" },
  ],
});
