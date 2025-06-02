import { ConfirmationService, ToastService } from "primevue";
import PrimeVue from "primevue/config";
import { createApp } from "vue";
import App from "./App.vue";
import { Preset } from "./style";

createApp(App)
  .use(PrimeVue, {
    theme: {
      preset: Preset,
      options: {
        darkModeSelector: ".dark",
      },
    },
  })
  .use(ToastService)
  .use(ConfirmationService)
  .mount("#app");
