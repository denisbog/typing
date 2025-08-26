/** @type {import('tailwindcss').Config} */
module.exports = {

 content: [
        "./src/**/*.rs",
        "./index.html",
    ],
  theme: {
    extend: {
      animation: {
        blink: 'blink 1s infinite',
      },
      keyframes: {
        blink: {
          '0%, 100%': { opacity: 1.0 },
          '50%': { opacity: 0.7 },
        },
      },
    },
  },
  plugins: [],
}

