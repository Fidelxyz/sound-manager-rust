import { createApp } from "vue";
import App from "./App.vue";
import PrimeVue from "primevue/config";
import { Preset } from "./style";

const app = createApp(App);
app.use(PrimeVue, {
  theme: {
    preset: Preset,
    options: {
      darkModeSelector: ".dark",
    },
  },
});
app.mount("#app");
