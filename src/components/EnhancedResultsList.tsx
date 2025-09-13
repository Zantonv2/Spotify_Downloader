import React, { useState, useMemo } from 'react'
import { 
  Plus, 
  Clock, 
  Music, 
  User, 
  Calendar, 
  Star, 
  Filter, 
  SortAsc,
  Search,
  X,
  Play,
  Heart
} from 'lucide-react'
import { TrackInfo } from '../types'

interface EnhancedResultsListProps {
  results: TrackInfo[]
  onDownload: (track: TrackInfo) => void
  isSearching: boolean
}

type SortOption = 'relevance' | 'title' | 'artist' | 'year' | 'duration'
type FilterOption = 'all' | 'high_quality' | 'recent' | 'popular'

const EnhancedResultsList: React.FC<EnhancedResultsListProps> = ({ 
  results, 
  onDownload, 
  isSearching 
}) => {
  const [sortBy, setSortBy] = useState<SortOption>('relevance')
  const [filterBy, setFilterBy] = useState<FilterOption>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [showFilters, setShowFilters] = useState(false)
  const [favorites, setFavorites] = useState<Set<string>>(new Set())

  const formatDuration = (seconds?: number) => {
    if (!seconds) return 'Unknown'
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  const formatYear = (year?: number) => {
    return year ? year.toString() : 'Unknown'
  }

  const getRelevanceColor = (score?: number) => {
    if (!score) return 'text-glass-400'
    if (score >= 80) return 'text-green-400'
    if (score >= 60) return 'text-yellow-400'
    if (score >= 40) return 'text-orange-400'
    return 'text-red-400'
  }


  const getQualityBadge = (track: TrackInfo) => {
    const score = (track as any).relevance_score
    if (!score) return null
    
    if (score >= 80) {
      return <span className="bg-green-500/20 text-green-300 px-2 py-1 rounded-full text-xs font-medium">High Quality</span>
    } else if (score >= 60) {
      return <span className="bg-yellow-500/20 text-yellow-300 px-2 py-1 rounded-full text-xs font-medium">Good Quality</span>
    } else if (score >= 40) {
      return <span className="bg-orange-500/20 text-orange-300 px-2 py-1 rounded-full text-xs font-medium">Fair Quality</span>
    }
    return <span className="bg-red-500/20 text-red-300 px-2 py-1 rounded-full text-xs font-medium">Low Quality</span>
  }

  const filteredAndSortedResults = useMemo(() => {
    let filtered = results

    // Apply search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase()
      filtered = filtered.filter(track => 
        track.title.toLowerCase().includes(query) ||
        track.artist.toLowerCase().includes(query) ||
        track.album?.toLowerCase().includes(query) ||
        track.genre?.toLowerCase().includes(query)
      )
    }

    // Apply quality filter
    if (filterBy === 'high_quality') {
      filtered = filtered.filter(track => {
        const score = (track as any).relevance_score
        return score && score >= 70
      })
    } else if (filterBy === 'recent') {
      filtered = filtered.filter(track => track.year && track.year >= 2020)
    } else if (filterBy === 'popular') {
      // This would need to be implemented based on actual popularity data
      filtered = filtered.filter(track => {
        const score = (track as any).relevance_score
        return score && score >= 60
      })
    }

    // Apply sorting
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'relevance':
          const scoreA = (a as any).relevance_score || 0
          const scoreB = (b as any).relevance_score || 0
          return scoreB - scoreA
        case 'title':
          return a.title.localeCompare(b.title)
        case 'artist':
          return a.artist.localeCompare(b.artist)
        case 'year':
          return (b.year || 0) - (a.year || 0)
        case 'duration':
          return (b.duration || 0) - (a.duration || 0)
        default:
          return 0
      }
    })

    return filtered
  }, [results, searchQuery, filterBy, sortBy])

  const toggleFavorite = (trackId: string) => {
    setFavorites(prev => {
      const newFavorites = new Set(prev)
      if (newFavorites.has(trackId)) {
        newFavorites.delete(trackId)
      } else {
        newFavorites.add(trackId)
      }
      return newFavorites
    })
  }

  if (isSearching) {
    return (
      <div className="glass-card animate-pulse-slow">
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="flex items-center space-x-4 p-4">
              <div className="w-16 h-16 bg-glass-200/30 rounded-lg animate-pulse" />
              <div className="flex-1 space-y-2">
                <div className="h-4 bg-glass-200/30 rounded w-3/4 animate-pulse" />
                <div className="h-3 bg-glass-200/30 rounded w-1/2 animate-pulse" />
              </div>
              <div className="w-24 h-10 bg-glass-200/30 rounded-lg animate-pulse" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  if (results.length === 0) {
    return (
      <div className="glass-card text-center py-12">
        <Music className="w-16 h-16 text-glass-400 mx-auto mb-4" />
        <h3 className="text-xl font-semibold text-glass-300 mb-2">No results found</h3>
        <p className="text-glass-400">
          Try searching for a different song, artist, or album
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header with Controls */}
      <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between space-y-4 lg:space-y-0">
        <div className="flex items-center space-x-4">
          <h2 className="text-2xl font-bold gradient-text">
            Search Results
          </h2>
          <span className="text-glass-400">
            {filteredAndSortedResults.length} of {results.length} tracks
          </span>
        </div>

        {/* Search and Filter Controls */}
        <div className="flex items-center space-x-3">
          {/* Search Input */}
          <div className="relative">
            <Search className="w-4 h-4 absolute left-3 top-1/2 transform -translate-y-1/2 text-glass-400" />
            <input
              type="text"
              placeholder="Filter results..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10 pr-4 py-2 bg-glass-200/20 border border-glass-300/20 rounded-lg text-white placeholder-glass-400 focus:outline-none focus:ring-2 focus:ring-purple-500/50 focus:border-transparent"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                className="absolute right-3 top-1/2 transform -translate-y-1/2 text-glass-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* Filter Toggle */}
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={`glass-button flex items-center space-x-2 ${showFilters ? 'bg-purple-500/20' : ''}`}
          >
            <Filter className="w-4 h-4" />
            <span>Filter</span>
          </button>
        </div>
      </div>

      {/* Filter and Sort Options */}
      {showFilters && (
        <div className="glass-card p-4 space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Sort Options */}
            <div>
              <label className="block text-sm font-medium text-glass-300 mb-2">Sort by</label>
              <div className="flex flex-wrap gap-2">
                {[
                  { value: 'relevance', label: 'Relevance', icon: Star },
                  { value: 'title', label: 'Title', icon: SortAsc },
                  { value: 'artist', label: 'Artist', icon: User },
                  { value: 'year', label: 'Year', icon: Calendar },
                  { value: 'duration', label: 'Duration', icon: Clock }
                ].map(({ value, label, icon: Icon }) => (
                  <button
                    key={value}
                    onClick={() => setSortBy(value as SortOption)}
                    className={`flex items-center space-x-1 px-3 py-2 rounded-lg text-sm transition-all ${
                      sortBy === value
                        ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30'
                        : 'bg-glass-200/10 text-glass-400 hover:bg-glass-200/20 hover:text-white'
                    }`}
                  >
                    <Icon className="w-4 h-4" />
                    <span>{label}</span>
                  </button>
                ))}
              </div>
            </div>

            {/* Filter Options */}
            <div>
              <label className="block text-sm font-medium text-glass-300 mb-2">Filter by</label>
              <div className="flex flex-wrap gap-2">
                {[
                  { value: 'all', label: 'All' },
                  { value: 'high_quality', label: 'High Quality' },
                  { value: 'recent', label: 'Recent (2020+)' },
                  { value: 'popular', label: 'Popular' }
                ].map(({ value, label }) => (
                  <button
                    key={value}
                    onClick={() => setFilterBy(value as FilterOption)}
                    className={`px-3 py-2 rounded-lg text-sm transition-all ${
                      filterBy === value
                        ? 'bg-purple-500/20 text-purple-300 border border-purple-500/30'
                        : 'bg-glass-200/10 text-glass-400 hover:bg-glass-200/20 hover:text-white'
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Results List */}
      <div className="space-y-3">
        {filteredAndSortedResults.map((track, index) => {
          const relevanceScore = (track as any).relevance_score
          const isBestMatch = index === 0 && relevanceScore && relevanceScore >= 60
          const isGoodMatch = relevanceScore && relevanceScore >= 60
          const isFavorite = favorites.has(track.id)
          
          return (
            <div
              key={`${track.id}-${index}`}
              className={`glass-card hover:glass-strong transition-all duration-200 group ${
                isBestMatch ? 'ring-2 ring-green-400/50 bg-green-400/5' : 
                isGoodMatch ? 'ring-1 ring-yellow-400/30' : ''
              }`}
            >
              <div className="flex items-center space-x-4">
                {/* Thumbnail */}
                <div className="w-16 h-16 bg-gradient-to-br from-purple-500 to-pink-500 rounded-lg flex items-center justify-center overflow-hidden relative group">
                  {track.thumbnail_url ? (
                    <img
                      src={track.thumbnail_url}
                      alt={track.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <Music className="w-8 h-8 text-white" />
                  )}
                  {/* Play overlay on hover */}
                  <div className="absolute inset-0 bg-black/50 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                    <Play className="w-6 h-6 text-white" />
                  </div>
                </div>

                {/* Track Info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <h3 className="text-lg font-semibold text-white truncate group-hover:text-purple-300 transition-colors">
                        {track.title}
                      </h3>
                      <div className="flex items-center space-x-4 text-sm text-glass-400 mt-1">
                        <div className="flex items-center space-x-1">
                          <User className="w-4 h-4" />
                          <span className="truncate">{track.artist}</span>
                        </div>
                        {track.album && (
                          <div className="flex items-center space-x-1">
                            <Music className="w-4 h-4" />
                            <span className="truncate">{track.album}</span>
                          </div>
                        )}
                        {track.year && (
                          <div className="flex items-center space-x-1">
                            <Calendar className="w-4 h-4" />
                            <span>{formatYear(track.year)}</span>
                          </div>
                        )}
                        {track.duration && (
                          <div className="flex items-center space-x-1">
                            <Clock className="w-4 h-4" />
                            <span>{formatDuration(track.duration)}</span>
                          </div>
                        )}
                      </div>
                      
                      {/* Quality and Genre */}
                      <div className="flex items-center space-x-2 mt-2">
                        {getQualityBadge(track)}
                        {track.genre && (
                          <span className="inline-block bg-purple-600/20 text-purple-300 px-2 py-1 rounded-full text-xs">
                            {track.genre}
                          </span>
                        )}
                        {relevanceScore && (
                          <span className={`text-xs font-medium ${getRelevanceColor(relevanceScore)}`}>
                            {Math.round(relevanceScore)}% match
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Action Buttons */}
                    <div className="flex items-center space-x-2 ml-4">
                      <button
                        onClick={() => toggleFavorite(track.id)}
                        className={`p-2 rounded-lg transition-all ${
                          isFavorite 
                            ? 'text-red-400 bg-red-400/20' 
                            : 'text-glass-400 hover:text-red-400 hover:bg-red-400/20'
                        }`}
                        title={isFavorite ? 'Remove from favorites' : 'Add to favorites'}
                      >
                        <Heart className={`w-4 h-4 ${isFavorite ? 'fill-current' : ''}`} />
                      </button>
                      
                      <button
                        onClick={() => onDownload(track)}
                        className="gradient-button flex items-center space-x-2 px-4 py-2 text-sm"
                      >
                        <Plus className="w-4 h-4" />
                        <span>Add to Queue</span>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )
        })}
      </div>

      {/* No filtered results */}
      {filteredAndSortedResults.length === 0 && searchQuery && (
        <div className="glass-card text-center py-8">
          <Search className="w-12 h-12 text-glass-400 mx-auto mb-4" />
          <h3 className="text-lg font-semibold text-glass-300 mb-2">No matches found</h3>
          <p className="text-glass-400">
            No tracks match your search query "{searchQuery}"
          </p>
        </div>
      )}
    </div>
  )
}

export default EnhancedResultsList
