import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import "@univerjs/preset-sheets-core/lib/index.css";

mount(App, {
  target: document.getElementById("app")!,
});
