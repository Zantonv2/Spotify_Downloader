import React, { useState } from 'react'
import { Search, Loader2 } from 'lucide-react'

interface SearchBarProps {
  onSearch: (query: string, deepSearch?: boolean) => void
  isSearching: boolean
}

const SearchBar: React.FC<SearchBarProps> = ({ onSearch, isSearching }) => {
  const [query, setQuery] = useState('')
  const [deepSearch, setDeepSearch] = useState(false)

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (query.trim() && !isSearching) {
      onSearch(query, deepSearch)
    }
  }

  return (
    <div className="glass-card">
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="relative">
          <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
            {isSearching ? (
              <Loader2 className="w-5 h-5 text-purple-400 animate-spin" />
            ) : (
              <Search className="w-5 h-5 text-glass-400" />
            )}
          </div>
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search for songs, artists, or albums..."
            className="glass-input pl-12 pr-4 py-4 text-lg"
            disabled={isSearching}
          />
        </div>
        
        <button
          type="submit"
          disabled={!query.trim() || isSearching}
          className="gradient-button w-full py-4 text-lg font-semibold disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
        >
          {isSearching ? (
            <div className="flex items-center justify-center space-x-2">
              <Loader2 className="w-5 h-5 animate-spin" />
              <span>Searching...</span>
            </div>
          ) : (
            'Search Music'
          )}
        </button>
        
        <div className="flex items-center space-x-2">
          <input
            type="checkbox"
            id="deepSearch"
            checked={deepSearch}
            onChange={(e) => setDeepSearch(e.target.checked)}
            className="w-4 h-4 text-purple-600 bg-glass-900 border-glass-600 rounded focus:ring-purple-500 focus:ring-2"
          />
          <label htmlFor="deepSearch" className="text-sm text-glass-300">
            Deep Search (7x more comprehensive)
          </label>
        </div>
      </form>
      
      <div className="mt-4 text-sm text-glass-400">
        <p>Search across YouTube, SoundCloud, Bandcamp, Vimeo and more</p>
      </div>
    </div>
  )
}

export default SearchBar
