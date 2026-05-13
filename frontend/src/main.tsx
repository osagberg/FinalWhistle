/* @refresh reload */
import { render } from "solid-js/web";
import { Router } from "@solidjs/router";
import "./styles.css";
import App from "./App";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element #root not found in index.html");
}

// <Router>'s children are <Route> definitions. App() returns a <Route>
// tree; invoke it to get JSX (a plain function reference here would type
// as `() => Element`, which @solidjs/router 0.15 won't accept as
// children — needs the materialized route nodes).
render(() => <Router>{App()}</Router>, root);
