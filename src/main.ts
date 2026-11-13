import { mount } from "svelte";
// Fonts are bundled with the app (no internet needed): the blackletter display faces of the
// built-in themes, MedievalSharp for custom themes, and Inter for everything else. Only the
// Latin subsets are imported to keep the app small.
import "@fontsource/unifrakturmaguntia/latin-400.css";
import "@fontsource/grenze-gotisch/latin-400.css";
import "@fontsource/grenze-gotisch/latin-600.css";
import "@fontsource/pirata-one/latin-400.css";
import "@fontsource/medievalsharp/latin-400.css";
import "@fontsource-variable/inter/index.css";
import "./app.css";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app")! });

export default app;
