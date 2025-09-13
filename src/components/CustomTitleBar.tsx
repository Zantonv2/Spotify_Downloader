import React, { useState, useEffect } from 'react'
import { Minus, Square, X, Maximize2 } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface CustomTitleBarProps {
  title?: string
}

const CustomTitleBar: React.FC<CustomTitleBarProps> = ({ title = "Spotify Downloader" }) => {
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    const checkMaximized = async () => {
      try {
        const window = getCurrentWindow()
        const maximized = await window.isMaximized()
        setIsMaximized(maximized)
      } catch (error) {
        console.error('Failed to check window state:', error)
      }
    }
    
    checkMaximized()

    // Listen for window resize events to update maximize state
    const handleResize = () => {
      checkMaximized()
    }

    window.addEventListener('resize', handleResize)
    
    return () => {
      window.removeEventListener('resize', handleResize)
    }
  }, [])

  const handleMinimize = async () => {
    try {
      const window = getCurrentWindow()
      await window.minimize()
    } catch (error) {
      console.error('Failed to minimize window:', error)
    }
  }

  const handleMaximize = async () => {
    try {
      const window = getCurrentWindow()
      if (isMaximized) {
        await window.unmaximize()
      } else {
        await window.maximize()
      }
      // Update state after a short delay to ensure the window state has changed
      setTimeout(async () => {
        try {
          const maximized = await window.isMaximized()
          setIsMaximized(maximized)
        } catch (error) {
          console.error('Failed to check maximize state:', error)
        }
      }, 100)
    } catch (error) {
      console.error('Failed to toggle maximize:', error)
    }
  }

  const handleClose = async () => {
    try {
      const window = getCurrentWindow()
      await window.close()
    } catch (error) {
      console.error('Failed to close window:', error)
    }
  }

  return (
    <div 
      className="flex items-center justify-between px-4 py-2 bg-slate-900/90 backdrop-blur-md border-b border-glass-300/20 select-none"
      data-tauri-drag-region
      style={{ 
        WebkitAppRegion: 'drag',
        userSelect: 'none'
      } as any}
    >
      {/* Title */}
      <div className="flex items-center space-x-3" data-tauri-drag-region>
        <div className="w-6 h-6 rounded-lg flex items-center justify-center shadow-lg overflow-hidden">
          {/* Use your custom app icon */}
          <img 
            src="/favicon-32x32.png" 
            alt="App Icon" 
            className="w-full h-full object-cover"
            onLoad={() => console.log('✅ Custom icon loaded successfully!')}
            onError={(e) => {
              console.log('❌ Icon failed to load, using fallback');
              // Fallback to gradient if image fails to load
              const target = e.target as HTMLImageElement;
              target.style.display = 'none';
              const parent = target.parentElement;
              if (parent) {
                parent.innerHTML = '<div class="w-full h-full bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center"><span class="text-white text-xs font-bold">S</span></div>';
              }
            }}
          />
        </div>
        <h1 className="text-sm font-semibold text-white tracking-wide">{title}</h1>
      </div>

      {/* Window Controls */}
      <div className="flex items-center space-x-1" data-tauri-drag-region="false" style={{ WebkitAppRegion: 'no-drag' } as any}>
        <button
          onClick={handleMinimize}
          className="w-8 h-8 flex items-center justify-center text-glass-400 hover:text-white hover:bg-glass-200/20 rounded transition-all duration-200 hover:scale-105"
          title="Minimize"
        >
          <Minus className="w-4 h-4" />
        </button>
        
        <button
          onClick={handleMaximize}
          className="w-8 h-8 flex items-center justify-center text-glass-400 hover:text-white hover:bg-glass-200/20 rounded transition-all duration-200 hover:scale-105"
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? <Square className="w-3 h-3" /> : <Maximize2 className="w-3 h-3" />}
        </button>
        
        <button
          onClick={handleClose}
          className="w-8 h-8 flex items-center justify-center text-glass-400 hover:text-white hover:bg-red-500/80 rounded transition-all duration-200 hover:scale-105"
          title="Close"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </div>
  )
}

export default CustomTitleBar
