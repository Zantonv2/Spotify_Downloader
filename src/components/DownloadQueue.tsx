import React, { useState, useEffect } from 'react'
import { 
  Download, 
  DownloadCloud,
  Search,
  ChevronDown,
  ChevronRight,
  Trash2
} from 'lucide-react'
import { DownloadTask } from '../types'
import CompactTrackCard from './CompactTrackCard'

interface DownloadQueueProps {
  queue: DownloadTask[]
  onRemove: (taskId: string) => void
  onRefresh: () => void
  onDownloadAll?: () => void
  onDownloadSelected?: (taskIds: string[]) => void
  onClearList?: () => void
  onRetryDownload?: (taskId: string) => void
  onRetryAllFailed?: () => void
}

const DownloadQueue: React.FC<DownloadQueueProps> = ({
  queue,
  onRemove,
  onRefresh,
  onDownloadAll,
  onDownloadSelected,
  onClearList,
  onRetryDownload,
  onRetryAllFailed,
}) => {
  const [autoRefresh] = useState(true)
  const [activeFilter, setActiveFilter] = useState<'all' | 'downloading' | 'completed' | 'failed'>('all')
  const [searchQuery, setSearchQuery] = useState('')
  const [collapsedSections, setCollapsedSections] = useState<Set<string>>(new Set())
  const [selectedTasks, setSelectedTasks] = useState<Set<string>>(new Set())

  // Auto-refresh every 5 seconds
  useEffect(() => {
    if (!autoRefresh) return

    const interval = setInterval(() => {
      onRefresh()
    }, 5000)

    return () => clearInterval(interval)
  }, [autoRefresh, onRefresh])

  // Search and filter logic
  const searchAndFilterQueue = (tasks: DownloadTask[]) => {
    let filtered = tasks

    // Apply status filter
    if (activeFilter !== 'all') {
      filtered = filtered.filter(task => {
        const status = task.status.toLowerCase()
        switch (activeFilter) {
          case 'downloading':
            return status === 'downloading' || status === 'processing'
          case 'completed':
            return status === 'completed'
          case 'failed':
            return status === 'failed'
          default:
            return true
        }
      })
    }

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      filtered = filtered.filter(task => 
        task.track_info.title.toLowerCase().includes(query) ||
        task.track_info.artist.toLowerCase().includes(query) ||
        (task.track_info.album && task.track_info.album.toLowerCase().includes(query))
      )
    }

    return filtered
  }

  const filteredQueue = searchAndFilterQueue(queue)

  // Calculate overall progress and statistics
  const calculateOverallProgress = () => {
    const total = queue.length
    const completed = queue.filter(task => task.status === 'completed').length
    const downloading = queue.filter(task => task.status === 'downloading' || task.status === 'processing').length
    const failed = queue.filter(task => task.status === 'failed').length
    
    const progress = total > 0 ? (completed / total) * 100 : 0
    
    // Calculate download speed (tracks per minute)
    const activeTasks = queue.filter(task => task.status === 'downloading' || task.status === 'processing')
    const downloadSpeed = activeTasks.length > 0 ? (completed / Math.max(1, activeTasks.length)) * 60 : 0 // tracks per minute
    
    // Calculate ETA (estimated time remaining)
    const remainingTasks = total - completed - failed
    const eta = downloadSpeed > 0 && remainingTasks > 0 ? Math.ceil(remainingTasks / downloadSpeed) : 0 // minutes
    
    return {
      total,
      completed,
      downloading,
      failed,
      pending: total - completed - downloading - failed,
      progress,
      downloadSpeed: Math.round(downloadSpeed * 10) / 10, // Round to 1 decimal
      eta
    }
  }

  const progressStats = calculateOverallProgress()

  // Group tasks by status for auto-collapse (case-insensitive)
  const groupedTasks = {
    downloading: filteredQueue.filter(task => task.status.toLowerCase() === 'downloading' || task.status.toLowerCase() === 'processing'),
    completed: filteredQueue.filter(task => task.status.toLowerCase() === 'completed'),
    failed: filteredQueue.filter(task => task.status.toLowerCase() === 'failed'),
    pending: filteredQueue.filter(task => task.status.toLowerCase() === 'pending' || task.status.toLowerCase() === 'paused')
  }

  const toggleSection = (section: string) => {
    const newCollapsed = new Set(collapsedSections)
    if (newCollapsed.has(section)) {
      newCollapsed.delete(section)
    } else {
      newCollapsed.add(section)
    }
    setCollapsedSections(newCollapsed)
  }

  // Multi-select handlers
  const handleTaskSelect = (taskId: string, event: React.MouseEvent) => {
    const newSelected = new Set(selectedTasks)
    
    if (event.ctrlKey || event.metaKey) {
      // Ctrl/Cmd+click: toggle selection
      if (newSelected.has(taskId)) {
        newSelected.delete(taskId)
      } else {
        newSelected.add(taskId)
      }
    } else {
      // Regular click: toggle selection (click again to deselect)
      if (newSelected.has(taskId)) {
        newSelected.delete(taskId)
      } else {
        // If no tracks selected, select this one
        // If other tracks selected, replace selection with this one
        newSelected.clear()
        newSelected.add(taskId)
      }
    }
    
    setSelectedTasks(newSelected)
  }

  const clearSelection = () => {
    setSelectedTasks(new Set())
  }

  const selectAllVisible = () => {
    const visibleTaskIds = filteredQueue.map(task => task.id)
    setSelectedTasks(new Set(visibleTaskIds))
  }

  const removeSelectedTasks = () => {
    selectedTasks.forEach(taskId => {
      onRemove(taskId)
    })
    setSelectedTasks(new Set())
  }

  const downloadSelectedTasks = () => {
    if (onDownloadSelected && selectedTasks.size > 0) {
      onDownloadSelected(Array.from(selectedTasks))
      setSelectedTasks(new Set())
    }
  }

  if (queue.length === 0) {
    return (
      <div className="glass-card text-center py-12">
        <Download className="w-16 h-16 text-glass-400 mx-auto mb-4" />
        <h3 className="text-xl font-semibold text-glass-300 mb-2">No downloads</h3>
        <p className="text-glass-400">
          Your download queue is empty. Search for music to get started!
        </p>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Download Queue</h2>
        <div className="flex items-center space-x-4">
          {/* Multi-select controls - Left side */}
          <div className="flex items-center space-x-2">
            {selectedTasks.size > 0 ? (
              <>
                <span className="text-sm text-glass-300">
                  {selectedTasks.size} selected
                </span>
                <button
                  onClick={downloadSelectedTasks}
                  className="bg-gradient-to-r from-green-500 to-emerald-500 hover:from-green-600 hover:to-emerald-600 text-white font-bold py-2 px-4 rounded-lg flex items-center space-x-2 transition-all duration-200 shadow-lg"
                  disabled={!onDownloadSelected}
                >
                  <Download className="w-4 h-4" />
                  <span>Download Selected</span>
                </button>
                <button
                  onClick={removeSelectedTasks}
                  className="text-red-400 hover:text-red-300 hover:bg-red-400/20 px-3 py-2 rounded-lg flex items-center space-x-2 transition-all duration-200"
                >
                  <Trash2 className="w-4 h-4" />
                  <span>Remove Selected</span>
                </button>
                <button
                  onClick={clearSelection}
                  className="text-glass-400 hover:text-white hover:bg-glass-200/20 px-3 py-2 rounded-lg transition-all duration-200"
                >
                  Clear Selection
                </button>
              </>
            ) : (
              <button
                onClick={selectAllVisible}
                className="text-glass-400 hover:text-white hover:bg-glass-200/20 px-3 py-2 rounded-lg transition-all duration-200"
                disabled={filteredQueue.length === 0}
              >
                Select All ({filteredQueue.length})
              </button>
            )}
          </div>

          {/* Action buttons - Right side */}
          <div className="flex items-center space-x-2">
            {progressStats.failed > 0 && onRetryAllFailed && (
              <button
                onClick={onRetryAllFailed}
                className="bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-600 hover:to-red-600 text-white font-bold py-3 px-4 rounded-lg flex items-center space-x-2 transition-all duration-200 shadow-lg"
              >
                <Download className="w-4 h-4" />
                <span>Retry All Failed ({progressStats.failed})</span>
              </button>
            )}
            <button
              onClick={onDownloadAll}
              className="bg-gradient-to-r from-purple-500 to-pink-500 hover:from-purple-600 hover:to-pink-600 text-white font-bold py-3 px-6 rounded-lg flex items-center space-x-2 transition-all duration-200 shadow-lg"
            >
              <DownloadCloud className="w-5 h-5" />
              <span>Download All ({queue.length})</span>
            </button>
          </div>
        </div>
      </div>

      {/* Summary Bar */}
      <div className="glass-card p-4">
        {/* Overall Progress Bar */}
        <div className="w-full h-4 bg-glass-200/20 rounded-full mb-4 relative">
          <div
            className="h-full bg-gradient-to-r from-purple-500 to-pink-500 rounded-full transition-all duration-300"
            style={{ width: `${progressStats.progress}%` }}
          ></div>
          <span className="absolute inset-0 flex items-center justify-center text-sm font-medium text-white">
            {Math.round(progressStats.progress)}%
          </span>
        </div>

        {/* Overall Stats */}
        <div className="flex justify-between items-center text-sm text-glass-400">
          <div className="flex items-center space-x-2">
            <span className="px-3 py-1 rounded-full bg-gray-500/20 text-gray-300">Total: {progressStats.total}</span>
            <span className="px-3 py-1 rounded-full bg-blue-500/20 text-blue-300">Downloading: {progressStats.downloading}</span>
            <span className="px-3 py-1 rounded-full bg-green-500/20 text-green-300">Completed: {progressStats.completed}</span>
            <span className="px-3 py-1 rounded-full bg-red-500/20 text-red-300">Failed: {progressStats.failed}</span>
            <span className="px-3 py-1 rounded-full bg-yellow-500/20 text-yellow-300">Pending: {progressStats.pending}</span>
          </div>
          
          {/* ETA and Download Speed */}
          <div className="flex items-center space-x-4 text-xs text-glass-300">
            {progressStats.downloadSpeed > 0 && (
              <span className="flex items-center space-x-1">
                <Download className="w-3 h-3" />
                <span>{progressStats.downloadSpeed} tracks/min</span>
              </span>
            )}
            {progressStats.eta > 0 && (
              <span className="flex items-center space-x-1">
                <span>⏱️</span>
                <span>{progressStats.eta}m remaining</span>
              </span>
            )}
          </div>
        </div>
      </div>

      {/* Search and Filters */}
      <div className="glass-card p-4">
        <div className="flex items-center space-x-4">
          {/* Search Bar */}
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-glass-400 w-4 h-4" />
            <input
              type="text"
              placeholder="Search tracks, artists, albums..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 bg-glass-200/20 border border-glass-300/20 rounded-lg text-white placeholder-glass-400 focus:outline-none focus:ring-2 focus:ring-purple-500/50 focus:border-transparent"
            />
          </div>

          {/* Filter Tabs */}
          <div className="flex items-center space-x-2">
            <button
              className={`px-4 py-2 rounded-full text-sm font-medium transition-colors duration-200 ${
                activeFilter === 'all' ? 'bg-purple-500/30 text-purple-200' : 'text-glass-400 hover:bg-glass-200/10'
              }`}
              onClick={() => setActiveFilter('all')}
            >
              All ({queue.length})
            </button>
            <button
              className={`px-4 py-2 rounded-full text-sm font-medium transition-colors duration-200 ${
                activeFilter === 'downloading' ? 'bg-blue-500/30 text-blue-200' : 'text-glass-400 hover:bg-glass-200/10'
              }`}
              onClick={() => setActiveFilter('downloading')}
            >
              Downloading ({groupedTasks.downloading.length})
            </button>
            <button
              className={`px-4 py-2 rounded-full text-sm font-medium transition-colors duration-200 ${
                activeFilter === 'completed' ? 'bg-green-500/30 text-green-200' : 'text-glass-400 hover:bg-glass-200/10'
              }`}
              onClick={() => setActiveFilter('completed')}
            >
              Completed ({groupedTasks.completed.length})
            </button>
            <button
              className={`px-4 py-2 rounded-full text-sm font-medium transition-colors duration-200 ${
                activeFilter === 'failed' ? 'bg-red-500/30 text-red-200' : 'text-glass-400 hover:bg-glass-200/10'
              }`}
              onClick={() => setActiveFilter('failed')}
            >
              Failed ({groupedTasks.failed.length})
            </button>
          </div>

          {/* Clear List Button */}
          <button
            onClick={onClearList}
            className="text-red-400 hover:text-red-300 hover:bg-red-400/20 px-3 py-2 rounded-lg flex items-center space-x-2 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={queue.length === 0}
          >
            <Trash2 className="w-4 h-4" />
            <span>Clear List</span>
          </button>
        </div>
      </div>

      {/* Track Cards Grid */}
      <div className="space-y-6 max-h-[60vh] overflow-y-auto smooth-scroll">
        {/* Downloading Section */}
        {groupedTasks.downloading.length > 0 && (
          <div className="glass-card p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white flex items-center space-x-2">
                <div className="w-3 h-3 rounded-full bg-blue-500"></div>
                <span>Downloading ({groupedTasks.downloading.length})</span>
              </h3>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {groupedTasks.downloading.map((task) => (
                <CompactTrackCard
                  key={task.id}
                  task={task}
                  onRemove={onRemove}
                  isSelected={selectedTasks.has(task.id)}
                  onSelect={handleTaskSelect}
                  onRetry={onRetryDownload}
                />
              ))}
            </div>
          </div>
        )}

        {/* Pending Section */}
        {groupedTasks.pending.length > 0 && (
          <div className="glass-card p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white flex items-center space-x-2">
                <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
                <span>Pending ({groupedTasks.pending.length})</span>
              </h3>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {groupedTasks.pending.map((task) => (
                <CompactTrackCard
                  key={task.id}
                  task={task}
                  onRemove={onRemove}
                  isSelected={selectedTasks.has(task.id)}
                  onSelect={handleTaskSelect}
                  onRetry={onRetryDownload}
                />
              ))}
            </div>
          </div>
        )}

        {/* Failed Section */}
        {groupedTasks.failed.length > 0 && (
          <div className="glass-card p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white flex items-center space-x-2">
                <div className="w-3 h-3 rounded-full bg-red-500"></div>
                <span>Failed ({groupedTasks.failed.length})</span>
              </h3>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {groupedTasks.failed.map((task) => (
                <CompactTrackCard
                  key={task.id}
                  task={task}
                  onRemove={onRemove}
                  isSelected={selectedTasks.has(task.id)}
                  onSelect={handleTaskSelect}
                  onRetry={onRetryDownload}
                />
              ))}
            </div>
          </div>
        )}

        {/* Completed Section - Collapsible */}
        {groupedTasks.completed.length > 0 && (
          <div className="glass-card p-4">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-lg font-semibold text-white flex items-center space-x-2">
                <div className="w-3 h-3 rounded-full bg-green-500"></div>
                <span>Completed ({groupedTasks.completed.length})</span>
              </h3>
              <button
                onClick={() => toggleSection('completed')}
                className="text-glass-400 hover:text-white transition-colors"
              >
                {collapsedSections.has('completed') ? (
                  <ChevronRight className="w-5 h-5 text-glass-400" />
                ) : (
                  <ChevronDown className="w-5 h-5 text-glass-400" />
                )}
              </button>
            </div>
            {!collapsedSections.has('completed') && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {groupedTasks.completed.map((task) => (
                  <CompactTrackCard
                    key={task.id}
                    task={task}
                    onRemove={onRemove}
                    onRetry={onRetryDownload}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

export default DownloadQueue