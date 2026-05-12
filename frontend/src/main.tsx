/* @refresh reload */
import { render } from "solid-js/web";
import { Router } from "@solidjs/router";
import "./styles.css";
import App from "./App";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element #root not found in index.html");
}

// <Router>'s children are <Route> definitions. App returns a <Route> tree.
render(() => <Router>{App}</Router>, root);
