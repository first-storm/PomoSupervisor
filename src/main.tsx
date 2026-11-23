import React from "react";
import ReactDOM from "react-dom/client";
import { windowLabel } from "./lib/api";
import Popover from "./popover/Popover";
import ReviewWindow from "./windows/Review";
import SettingsWindow from "./windows/Settings";
import "./styles.css";

// One bundle, three windows: the menu-bar popover ("main"), Review and Settings.
const label = windowLabel();
const Root = label === "review" ? ReviewWindow : label === "settings" ? SettingsWindow : Popover;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
