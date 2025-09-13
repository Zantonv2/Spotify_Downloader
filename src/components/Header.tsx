import React from 'react'
import { Settings, Download, Search, Music } from 'lucide-react'

interface HeaderProps {
  onSettingsClick: () => void
  onTabChange: (tab: 'search' | 'queue' | 'import') => void
  activeTab: 'search' | 'queue' | 'import'
  downloadCount: number
}

const Header: React.FC<HeaderProps> = ({ 
  onSettingsClick, 
  onTabChange, 
  activeTab, 
  downloadCount 
}) => {
  return (
    <div className="container mx-auto px-4 py-2 max-w-6xl">
      <header className="bg-white/10 backdrop-blur-md border border-white/20 rounded-xl p-3 shadow-2xl">
        <div className="flex items-center justify-center relative">
        {/* Navigation Tabs - Reorganized: Import | Search | Downloads */}
        <div className="flex items-center space-x-2">
          {/* Import Tab - Left */}
          <button
            onClick={() => onTabChange('import')}
            className={`flex items-center space-x-2 px-4 py-2 rounded-lg transition-all duration-200 ${
              activeTab === 'import'
                ? 'bg-purple-600/30 text-purple-300'
                : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
            }`}
          >
            <Music className="w-4 h-4" />
            <span>Import</span>
          </button>
          
          {/* Search Tab - Center */}
          <button
            onClick={() => onTabChange('search')}
            className={`flex items-center space-x-2 px-4 py-2 rounded-lg transition-all duration-200 ${
              activeTab === 'search'
                ? 'bg-purple-600/30 text-purple-300'
                : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
            }`}
          >
            <Search className="w-4 h-4" />
            <span>Search</span>
          </button>
          
          {/* Downloads Tab - Right */}
          <button
            onClick={() => onTabChange('queue')}
            className={`flex items-center space-x-2 px-4 py-2 rounded-lg transition-all duration-200 relative ${
              activeTab === 'queue'
                ? 'bg-purple-600/30 text-purple-300'
                : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
            }`}
          >
            <Download className="w-4 h-4" />
            <span>Downloads</span>
            {downloadCount > 0 && (
              <span className="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full w-5 h-5 flex items-center justify-center">
                {downloadCount}
              </span>
            )}
          </button>
        </div>

        {/* Settings Button - Positioned absolutely to the right */}
        <button
          onClick={onSettingsClick}
          className="absolute right-0 glass-button flex items-center space-x-2"
        >
          <Settings className="w-4 h-4" />
          <span>Settings</span>
        </button>
        </div>
      </header>
    </div>
  )
}

export default Header
