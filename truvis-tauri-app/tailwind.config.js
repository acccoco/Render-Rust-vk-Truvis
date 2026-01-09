/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        editor: {
          bg: '#1e1e1e',
          panel: '#252526',
          toolbar: '#2d2d2d',
          border: '#3c3c3c',
          hover: '#2a2d2e',
          accent: '#007acc',
          text: {
            primary: '#cccccc',
            secondary: '#bbbbbb',
            muted: '#888888',
            white: '#ffffff',
          }
        }
      }
    },
  },
  plugins: [],
}
