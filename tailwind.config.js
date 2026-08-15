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
        'fade-in': 'fadeIn 0.4s ease-out both',
        'slide-up': 'slideUp 0.45s cubic-bezier(0.16, 1, 0.3, 1) both',
        'float-in': 'floatIn 0.5s cubic-bezier(0.16, 1, 0.3, 1) both',
      },
      keyframes: {
        blink: {
          '0%, 100%': { opacity: 1.0 },
          '50%': { opacity: 0.7 },
        },
        fadeIn: {
          '0%': { opacity: 0, transform: 'translateY(8px)' },
          '100%': { opacity: 1, transform: 'translateY(0)' },
        },
        slideUp: {
          '0%': { opacity: 0, transform: 'translateY(24px) scale(0.98)' },
          '100%': { opacity: 1, transform: 'translateY(0) scale(1)' },
        },
        floatIn: {
          '0%': { opacity: 0, transform: 'translateY(16px)' },
          '100%': { opacity: 1, transform: 'translateY(0)' },
        },
      },
    },
  },
  plugins: [],
}
