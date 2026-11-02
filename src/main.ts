import { createApp } from "vue";
import { createPinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";
import "./style.css";

createApp(App).use(createPinia()).use(ui).mount("#app");
