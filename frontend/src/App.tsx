import { Route, type RouteSectionProps } from "@solidjs/router";
import { lazy, type JSX } from "solid-js";
import Layout from "./components/Layout";

// Routes lazy-load to keep first-paint small. PixiJS + ECharts are heavy; the
// Match page paying their cost only when reached is a meaningful win.
const Home = lazy(() => import("./routes/Home"));
const Squad = lazy(() => import("./routes/Squad"));
const Player = lazy(() => import("./routes/Player"));
const Tactics = lazy(() => import("./routes/Tactics"));
const Transfers = lazy(() => import("./routes/Transfers"));
const League = lazy(() => import("./routes/League"));
const Match = lazy(() => import("./routes/Match"));
const Career = lazy(() => import("./routes/Career"));
const Stats = lazy(() => import("./routes/Stats"));
const Settings = lazy(() => import("./routes/Settings"));
// Dev-only board — T1-2a tactical board with frame scrubber + window.fwDev.
const DevBoard = lazy(() => import("./routes/Dev/TacticalBoard"));
// Dev-only harness — browser-previewable mount of the production
// `<TacticalBoard>` against a committed dump_frames fixture (T4-1 preview).
const DevBoardPreview = lazy(() => import("./routes/Dev/BoardPreview"));

// Root component receives nested route output via `props.children`. Pattern
// lifted from the @solidjs/router v0.15 root-layout recipe — it's the only
// shape that survives Tauri's strict CSP (no inline scripts, no eval).
const Root = (props: RouteSectionProps): JSX.Element => {
  return <Layout>{props.children}</Layout>;
};

// Exported as the default to keep main.tsx tiny.
export default function App(): JSX.Element {
  return (
    <Route path="/" component={Root}>
      <Route path="/" component={Home} />
      <Route path="/squad" component={Squad} />
      <Route path="/player/:id" component={Player} />
      <Route path="/tactics" component={Tactics} />
      <Route path="/transfers" component={Transfers} />
      <Route path="/league" component={League} />
      <Route path="/match" component={Match} />
      <Route path="/career" component={Career} />
      <Route path="/stats" component={Stats} />
      <Route path="/settings" component={Settings} />
      <Route path="/dev/board" component={DevBoard} />
      <Route path="/dev/board-preview" component={DevBoardPreview} />
    </Route>
  );
}
