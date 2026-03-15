import React from 'react'
import ReactDOM from 'react-dom/client'
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from './App'
import './styles.css'  // or whatever your CSS file is named
import { TooltipProvider } from "./components/ui/tooltip"

// Enable react-scan only when REACT_SCAN env variable is set
if (import.meta.env.DEV && import.meta.env.VITE_REACT_SCAN === 'true') {
  import('react-scan').then(({ scan }) => scan());
}

try {
  if (getCurrentWebviewWindow().label === "quick") {
    document.documentElement.classList.add("quick-window");
    document.body.classList.add("quick-window");
  }
} catch {
  // Browser fallback (non-tauri).
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <TooltipProvider>
      <App />
    </TooltipProvider>
  </React.StrictMode>,
) 
