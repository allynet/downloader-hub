import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { useAuthStore } from "@/stores/auth-store";
import { Shell } from "@/pages/Shell";
import { LoginPage } from "@/pages/LoginPage";
import { DashboardPage } from "@/pages/DashboardPage";
import { RequestsPage } from "@/pages/RequestsPage";
import { NodesPage } from "@/pages/NodesPage";
import { TokensPage } from "@/pages/TokensPage";
import { AccountsPage } from "@/pages/AccountsPage";
import { MetricsPage } from "@/pages/MetricsPage";
import { RestrictionsPage } from "@/pages/RestrictionsPage";
import { LoggingPage } from "@/pages/LoggingPage";
import { SecretsPage } from "@/pages/SecretsPage";

const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  beforeLoad: () => {
    if (useAuthStore.getState().me) {
      throw redirect({ to: "/" });
    }
  },
  component: LoginPage,
});

const authedLayout = createRoute({
  getParentRoute: () => rootRoute,
  id: "_authed",
  beforeLoad: () => {
    if (!useAuthStore.getState().me) {
      throw redirect({ to: "/login" });
    }
  },
  component: Shell,
});

const dashboardRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/",
  component: DashboardPage,
});
const requestsRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/requests",
  component: RequestsPage,
});
const nodesRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/nodes",
  component: NodesPage,
});
const tokensRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/tokens",
  component: TokensPage,
});
const accountsRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/accounts",
  component: AccountsPage,
});
const restrictionsRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/restrictions",
  component: RestrictionsPage,
});
const metricsRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/metrics",
  component: MetricsPage,
});
const loggingRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/logging",
  component: LoggingPage,
});
const secretsRoute = createRoute({
  getParentRoute: () => authedLayout,
  path: "/secrets",
  component: SecretsPage,
});

const routeTree = rootRoute.addChildren([
  loginRoute,
  authedLayout.addChildren([
    dashboardRoute,
    requestsRoute,
    nodesRoute,
    tokensRoute,
    accountsRoute,
    restrictionsRoute,
    loggingRoute,
    secretsRoute,
    metricsRoute,
  ]),
]);

export const router = createRouter({
  routeTree,
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
