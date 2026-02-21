/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: "class",
  content: [
    "./pages/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}",
    "./src/**/*.{ts,tsx}",
    "./index.html",
  ],

  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      boxShadow: {
        onbSidebar: "-5px 0px 12px rgba(17,18,30,.7)",
        onbButton: "0px 8px 12px rgba(0,0,0,0.7)",
      },
      maxWidth: {
        onbSidebar: "600px",
        onbParagraph: "350px",
      },
      height: {
        imgCarousel: "400px",
        onbImgHero: "380px",
      },
      width: {
        featureDisplay: "85%",
        onbImgHero: "280px",
      },
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        brandPurple: "rgb(var(--brandPurple) / <alpha-value>)",
        brandPurpleBg: "rgb(var(--brandPurpleBg) / <alpha-value>)",
        brandPurpleBgForeground:
          "rgb(var(--brandPurpleBgForeground) / <alpha-value>)",
        brandPurpleText: "rgb(var(--brandPurpleText) / <alpha-value>)",
        brandPurpleHighlight:
          "rgb(var(--brandPurpleHighlight) / <alpha-value>)",
        brandPurpleBorder: "rgb(var(--brandPurpleBorder) / <alpha-value>)",
        //Onboarding And Dark Mode Color Palette
        customBlue: "rgb(var(--blue) / <alpha-value>)",
        blueShadow: "rgb(var(--blueShadow) / <alpha-value>)",
        blueHighlight: "rgb(var(--blueHighlight) / <alpha-value>)",
        bg: "rgb(var(--bg) / <alpha-value>)",
        bgShadow: "rgb(var(--bgShadow) / <alpha-value>)",
        darkBg: "rgb(var(--darkBg) / <alpha-value>)",
        darkBgMid: "rgb(var(--darkBgMid) / <alpha-value>)",
        darkBgHighlight: "rgb(var(--darkBgHighlight) / <alpha-value>)",
        customWhite: "rgb(var(--white) / <alpha-value>)",
        customGray: "rgb(var(--grey) / <alpha-value>)",
        customGreen: "rgb(var(--green)/ <alpha-value>)",
        greenHighlight: "rgb(var(--greenHighlight)/ <alpha-value>)",
        greenShadow: "rgb(var(--greenShadow)/ <alpha-value>)",
        customPurple: "rgb(var(--purple) / <alpha-value>)",
        customRed: "rgb(var(--red) / <alpha-value>)",
        redShadow: "rgb(var(--redShadow) / <alpha-value>)",
        redHighlight: "rgb(var(--redHighlight) / <alpha-value>)",
        customYellow: "rgb(var(--yellow) / <alpha-value>)",
        yellowShadow: "rgb(var(--yellowShadow) / <alpha-value>)",
        yellowHighlight: "rgb(var(--yellowHighlight) / <alpha-value>)",
        text: "rgb(var(--text))",
        //
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      keyframes: {
        scroll: {
          "0%": { transform: "translateX(0)" },
          "100%": { transform: "translateX(-50%)" },
        },
        scrollToPerms: {
          "0%": { transform: "translateX(0)" },
          "100%": { transform: "translateX(-33.3333%)" },
        },
        scrollToComplete: {
          "0%": { transform: "translateX(-33.3333%)" },
          "100%": { transform: "translateX(-66.6666%)" },
        },
        floatIn: {
          "0%": {
            transform: "translateY(70px)",
            opacity: "0",
          },
          "100%": {
            transform: "translateY(0)",
            opacity: "1",
          },
        },
        fadeOut: {
          "0%": {
            opacity: "1",
          },
          "100%": {
            opacity: "0",
          },
        },
      },
      animation: {
        scroll: "scroll 30s linear infinite",
        scrollSlow: "scroll 70s linear infinite",
        toPerms: "scrollToPerms .3s ease-in-out forwards",
        toComplete: "scrollToComplete .3s ease-in-out forwards",
        floatIn: "floatIn 1s ease forwards",
        fadeOut: "fadeOut 3s ease-in forwards",
      },
      fontFamily: {
        jockey: ['"Jockey One"', "sans-serif"],
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};
