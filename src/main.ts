import { mount } from "svelte";

import "./app.css";
import Root from "./Root.svelte";

const target = document.getElementById("app");

if (!target) {
  throw new Error("Elemento #app não encontrado.");
}

mount(Root, { target });
