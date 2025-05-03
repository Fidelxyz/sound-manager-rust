import { ConfirmationService, ToastService } from "primevue";
import PrimeVue from "primevue/config";
import { createApp } from "vue";
import App from "./App.vue";
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
app.use(ToastService);
app.use(ConfirmationService);
app.mount("#app");
