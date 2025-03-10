import { definePreset } from "@primeuix/themes";
import Aura from "@primeuix/themes/aura";

export const Preset = definePreset(Aura, {
  semantic: {
    formField: {
      paddingX: "0.65rem",
      paddingY: "0.3rem",
    },
    datatable: {
      bodyCell: {
        padding: "0.5rem 1rem !important",
      },
    },
    list: {
      option: {
        padding: "0.3rem 0.65rem",
      }
    },
    transitionDuration: "0.1s",
    colorScheme: {
      dark: {
        primary: {
          0: "hsl(0, 0%, 100%)",
          50: "hsl(0, 0%, 98%)",
          100: "hsl(0, 0%, 95%)",
          200: "hsl(0, 0%, 90%)",
          300: "hsl(0, 0%, 85%)",
          400: "hsl(0, 0%, 60%)",
          500: "hsl(0, 0%, 45%)",
          600: "hsl(0, 0%, 35%)",
          700: "hsl(0, 0%, 25%)",
          800: "hsl(0, 0%, 15%)",
          900: "hsl(0, 0%, 10%)",
          950: "hsl(0, 0%, 5%)",
        },
        surface: {
          0: "hsl(0, 0%, 90%)",
          50: "hsl(0, 0%, 85%)",
          100: "hsl(0, 0%, 80%)",
          200: "hsl(0, 0%, 75%)",
          300: "hsl(0, 0%, 65%)",
          400: "hsl(0, 0%, 50%)",
          500: "hsl(0, 0%, 30%)",
          600: "hsl(0, 0%, 20%)",
          700: "hsl(0, 0%, 18%)",
          800: "hsl(0, 0%, 15%)",
          900: "hsl(0, 0%, 12%)",
          950: "hsl(0, 0%, 11%)",
        },
      },
    },
  },
});
