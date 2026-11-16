// Restaurant's own interface at /restaurant/: the floor (tables and open orders) and the kitchen's
// tickets by station, with every record in the records frame's lists and forms.

import { useEffect } from "react";
import { Link, useRouteError } from "react-router";
import { shell, t } from "@ferrum/client";
import { EmptyState, ErrorState } from "@ferrum/kit";
import { mountApp, RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound() {
  return (
    <EmptyState title={t("Page not found")}>
      <Link to="/">{t("Go to the floor")}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

setFrameMode("app", ["restaurant"]);

mountApp({
  route: "restaurant",
  catalogs: { ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") },
  routes: [
    {
      path: "/",
      element: (
        <>
          <Started />
          <RequireAuth />
        </>
      ),
      errorElement: <RouteError />,
      children: [
        { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
        { path: "kitchen", lazy: () => import("./Kitchen").then((m) => ({ Component: m.Kitchen })) },
        { path: "e/:entity", lazy: page("ListView") },
        { path: "e/:entity/:id", lazy: page("FormView") },
        { path: "r/:name", lazy: page("ReportView") },
        { path: "*", element: <NotFound /> },
      ],
    },
  ],
});
