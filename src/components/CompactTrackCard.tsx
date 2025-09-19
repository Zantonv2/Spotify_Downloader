import React from 'react'
import { 
  X,
  RotateCcw
} from 'lucide-react'
import { DownloadTask } from '../types'

interface CompactTrackCardProps {
  task: DownloadTask
  onRemove: (taskId: string) => void
  isSelected?: boolean
  onSelect?: (taskId: string, event: React.MouseEvent) => void
  onRetry?: (taskId: string) => void
}

const CompactTrackCard: React.FC<CompactTrackCardProps> = ({
  task,
  onRemove,
  isSelected = false,
  onSelect,
  onRetry,
}) => {
  // Helper to format duration from seconds to MM:SS
  const formatDuration = (seconds?: number) => {
    if (seconds === undefined || isNaN(seconds)) return '0:00'
    const minutes = Math.floor(seconds / 60)
    const remainingSeconds = Math.floor(seconds % 60)
    return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`
  }

  return (
    <div 
      className={`glass-card p-4 relative rounded-lg shadow-lg cursor-pointer transition-all duration-300 animate-fade-in ${
        isSelected 
          ? 'ring-2 ring-purple-500 bg-purple-500/10' 
          : 'hover:bg-glass-200/10'
      }`}
      onClick={(e) => onSelect?.(task.id, e)}
    >
      {/* Action Buttons - Top Right */}
      <div className="absolute top-2 right-2 flex items-center space-x-1 z-10">
        {task.status === 'failed' && onRetry && (
          <button
            onClick={(e) => {
              e.stopPropagation()
              onRetry(task.id)
            }}
            className="p-1 rounded-full text-gray-600 hover:bg-orange-500/20 hover:text-orange-400 transition-colors duration-200"
            title="Retry download"
          >
            <RotateCcw className="w-4 h-4" />
          </button>
        )}
        <button
          onClick={(e) => {
            e.stopPropagation()
            onRemove(task.id)
          }}
          className="p-1 rounded-full text-gray-600 hover:bg-red-500/20 hover:text-red-400 transition-colors duration-200"
          title="Remove track"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Top Section: Album Art, Title, Artist/Album, Duration */}
      <div className="flex items-start space-x-3 mb-3">
        {/* Album Art - Solid Background with Track Number */}
        <div className="flex-shrink-0 w-12 h-12 rounded-md bg-purple-500 flex items-center justify-center">
          <span className="text-white text-lg font-bold">
            {task.track_info.track_number ? `№${task.track_info.track_number}` : '♪'}
          </span>
        </div>

        {/* Track Info */}
        <div className="flex-1 min-w-0">
          <h3 className="text-lg font-bold text-white truncate">
            {task.track_info.title}
          </h3>
          <p className="text-gray-300 text-sm truncate">
            {task.track_info.artist}
            {task.track_info.album && ` • ${task.track_info.album}`}
          </p>
        </div>
      </div>

      {/* Metadata Section (Details) - Year, Genre */}
      <div className="flex items-center space-x-4 text-sm text-gray-200 mb-3">
        {task.track_info.year && (
          <span className="flex items-center space-x-1">
            <span>📅</span>
            <span>{task.track_info.year}</span>
          </span>
        )}
        {task.track_info.genre && (
          <span className="flex items-center space-x-1">
            <span>🏷️</span>
            <span>{task.track_info.genre.replace(/,/g, ', ')}</span>
          </span>
        )}
      </div>

      {/* Status Badge */}
      <div className="bg-blue-500 text-white text-xs font-medium py-1 px-2 rounded mb-3 w-fit">
        {task.status === 'downloading' ? 'Downloading...' :
         task.status === 'completed' ? 'Completed' :
         task.status === 'failed' ? 'Failed' :
         task.status === 'paused' ? 'Paused' :
         'Pending'}
      </div>

      {/* Progress Bar with 0% text */}
      <div className="relative w-full h-3 bg-gray-200 rounded-full mb-3">
        <div
          className="absolute top-0 left-0 h-full bg-purple-500 rounded-full transition-all duration-300"
          style={{ width: `${task.progress || 0}%` }}
        ></div>
        <span className={`absolute inset-0 flex items-center justify-center text-xs font-medium ${
          (task.progress || 0) > 20 ? 'text-white' : 'text-gray-600'
        }`}>
          {Math.round(task.progress || 0)}%
        </span>
      </div>

      {/* Duration - Bottom Right */}
      {task.track_info.duration && (
        <div className="flex justify-end">
          <span className="text-gray-300 text-sm">
            {formatDuration(task.track_info.duration)}
          </span>
        </div>
      )}

    </div>
  )
}

export default CompactTrackCard