import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './styles.css'  // or whatever your CSS file is named
import { TooltipProvider } from "./components/ui/tooltip"

// Enable react-scan only when REACT_SCAN env variable is set
if (import.meta.env.DEV && import.meta.env.VITE_REACT_SCAN === 'true') {
  import('react-scan').then(({ scan }) => scan());
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <TooltipProvider>
      <App />
    </TooltipProvider>
  </React.StrictMode>,
) 