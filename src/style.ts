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
      },
    },
    floatlabel: {
      fontWeight: "normal !important",
    },
    transitionDuration: "0.1s",
    colorScheme: {
      dark: {
        primary: {
          0: "#FFFFFF",
          50: "{teal.50}",
          100: "{teal.100}",
          200: "{teal.200}",
          300: "{teal.300}",
          400: "{teal.400}",
          500: "{teal.500}",
          600: "{teal.600}",
          700: "{teal.700}",
          800: "{teal.800}",
          900: "{teal.900}",
          950: "{teal.950}",
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
