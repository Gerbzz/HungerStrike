import { tauriApi } from "./api";
import { mountApp } from "./app";
import "./styles.css";

mountApp(document.querySelector<HTMLElement>("#app")!, tauriApi).catch((error) => {
  document.querySelector<HTMLElement>("#app")!.textContent = `Could not open local data: ${error}`;
});
