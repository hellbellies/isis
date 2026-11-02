import { createApp } from "vue";
import { createPinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";
import { router } from "./router";
import "./style.css";

createApp(App).use(createPinia()).use(router).use(ui).mount("#app");
