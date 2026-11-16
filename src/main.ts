import { mount } from "svelte";

import "./app.css";
import { startTheme } from "./lib/theme.svelte";
import Root from "./Root.svelte";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Elemento #app não encontrado.");
}

startTheme();
mount(Root, { target });
