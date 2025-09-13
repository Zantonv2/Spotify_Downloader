import React from 'react'
import { Plus, Clock, Music, User, Calendar, Star } from 'lucide-react'
import { TrackInfo } from '../types'

interface ResultsListProps {
  results: TrackInfo[]
  onDownload: (track: TrackInfo) => void
  isSearching: boolean
}

const ResultsList: React.FC<ResultsListProps> = ({ results, onDownload, isSearching }) => {
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

  const getRelevanceLabel = (score?: number) => {
    if (!score) return 'Unknown'
    if (score >= 80) return 'Excellent'
    if (score >= 60) return 'Good'
    if (score >= 40) return 'Fair'
    return 'Poor'
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
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold gradient-text">
          Search Results
        </h2>
        <span className="text-glass-400">
          {results.length} track{results.length !== 1 ? 's' : ''} found
        </span>
      </div>

      <div className="space-y-3">
        {results.map((track, index) => {
          const relevanceScore = (track as any).relevance_score
          const isBestMatch = index === 0 && relevanceScore && relevanceScore >= 60
          const isGoodMatch = relevanceScore && relevanceScore >= 60
          
          return (
            <div
              key={`${track.id}-${index}`}
              className={`glass-card hover:glass-strong transition-all duration-200 animate-slide-up ${
                isBestMatch ? 'ring-2 ring-green-400/50 bg-green-400/5' : 
                isGoodMatch ? 'ring-1 ring-yellow-400/30' : ''
              }`}
              style={{ animationDelay: `${index * 0.1}s` }}
            >
            <div className="flex items-center space-x-4">
              {/* Thumbnail */}
              <div className="w-16 h-16 bg-gradient-to-br from-purple-500 to-pink-500 rounded-lg flex items-center justify-center overflow-hidden">
                {track.thumbnail_url ? (
                  <img
                    src={track.thumbnail_url}
                    alt={track.title}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <Music className="w-8 h-8 text-white" />
                )}
              </div>

              {/* Track Info */}
              <div className="flex-1 min-w-0">
                <h3 className="text-lg font-semibold text-white truncate">
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
                  {/* Relevance Score */}
                  {(track as any).relevance_score && (
                    <div className="flex items-center space-x-1">
                      <Star className="w-4 h-4" />
                      <span className={`font-medium ${getRelevanceColor((track as any).relevance_score)}`}>
                        {Math.round((track as any).relevance_score)}% - {getRelevanceLabel((track as any).relevance_score)}
                      </span>
                    </div>
                  )}
                </div>
                {track.genre && (
                  <div className="mt-2">
                    <span className="inline-block bg-purple-600/20 text-purple-300 px-2 py-1 rounded-full text-xs">
                      {track.genre}
                    </span>
                  </div>
                )}
              </div>

              {/* Add to Queue Button */}
              <button
                onClick={() => onDownload(track)}
                className="gradient-button flex items-center space-x-2 px-6 py-3"
              >
                <Plus className="w-4 h-4" />
                <span>Add to Queue</span>
              </button>
            </div>
          </div>
          )
        })}
      </div>
    </div>
  )
}

export default ResultsList
