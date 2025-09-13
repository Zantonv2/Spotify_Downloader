import React from 'react'
import { Download, Search, Tag, Music, CheckCircle, AlertCircle, Clock } from 'lucide-react'
import { DownloadTask } from '../types'

interface EnhancedProgressBarProps {
  task: DownloadTask
  showDetails?: boolean
}

interface ProgressStage {
  name: string
  icon: React.ReactNode
  progress: number
  status: 'pending' | 'active' | 'completed' | 'failed'
  description: string
}

const EnhancedProgressBar: React.FC<EnhancedProgressBarProps> = ({ 
  task, 
  showDetails = true 
}) => {
  const getStages = (): ProgressStage[] => {
    const baseProgress = task.progress || 0
    
    return [
      {
        name: 'Downloading',
        icon: <Download className="w-3 h-3" />,
        progress: Math.min(baseProgress * 0.4, 100),
        status: baseProgress > 0 ? (baseProgress < 40 ? 'active' : 'completed') : 'pending',
        description: 'Downloading audio file'
      },
      {
        name: 'Searching',
        icon: <Search className="w-3 h-3" />,
        progress: Math.min(Math.max((baseProgress - 40) * 2.5, 0), 100),
        status: baseProgress < 40 ? 'pending' : (baseProgress < 60 ? 'active' : 'completed'),
        description: 'Finding metadata'
      },
      {
        name: 'Embedding',
        icon: <Tag className="w-3 h-3" />,
        progress: Math.min(Math.max((baseProgress - 60) * 2.5, 0), 100),
        status: baseProgress < 60 ? 'pending' : (baseProgress < 80 ? 'active' : 'completed'),
        description: 'Adding metadata & lyrics'
      },
      {
        name: 'Processing',
        icon: <Music className="w-3 h-3" />,
        progress: Math.min(Math.max((baseProgress - 80) * 5, 0), 100),
        status: baseProgress < 80 ? 'pending' : (baseProgress < 100 ? 'active' : 'completed'),
        description: 'Final processing'
      }
    ]
  }

  const getStatusIcon = (status: DownloadTask['status']) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-400" />
      case 'failed':
        return <AlertCircle className="w-4 h-4 text-red-400" />
      case 'paused':
        return <Clock className="w-4 h-4 text-yellow-400" />
      case 'downloading':
      case 'processing':
        return <div className="w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
      default:
        return <Clock className="w-4 h-4 text-glass-400" />
    }
  }

  const getStatusColor = (status: DownloadTask['status']) => {
    switch (status) {
      case 'completed':
        return 'from-green-500 to-emerald-500'
      case 'failed':
        return 'from-red-500 to-rose-500'
      case 'paused':
        return 'from-yellow-500 to-orange-500'
      case 'downloading':
      case 'processing':
        return 'from-blue-500 to-cyan-500'
      default:
        return 'from-purple-500 to-pink-500'
    }
  }

  const getStatusTextColor = (status: DownloadTask['status']) => {
    switch (status) {
      case 'completed':
        return 'text-green-400'
      case 'failed':
        return 'text-red-400'
      case 'paused':
        return 'text-yellow-400'
      case 'downloading':
      case 'processing':
        return 'text-blue-400'
      default:
        return 'text-purple-400'
    }
  }

  const getStatusText = (status: DownloadTask['status']) => {
    switch (status) {
      case 'completed':
        return 'Completed'
      case 'failed':
        return 'Failed'
      case 'paused':
        return 'Paused'
      case 'downloading':
        return 'Downloading'
      case 'processing':
        return 'Processing'
      case 'pending':
        return 'Pending'
      case 'cancelled':
        return 'Cancelled'
      default:
        return 'Unknown'
    }
  }

  const stages = getStages()
  const currentStage = stages.find(stage => stage.status === 'active') || stages[0]

  return (
    <div className="space-y-3">
      {/* Main Progress Bar */}
      <div className="space-y-3">
        <div className="flex items-center justify-between text-sm">
          <div className="flex items-center space-x-2">
            {getStatusIcon(task.status)}
            <span className={`font-medium ${getStatusTextColor(task.status)}`}>
              {getStatusText(task.status)}
            </span>
            {task.status === 'downloading' && (
              <span className="text-xs text-glass-400">
                {currentStage.description}
              </span>
            )}
          </div>
          <div className="flex items-center space-x-2">
            <span className={`font-bold text-lg ${getStatusTextColor(task.status)}`}>
              {Math.round(task.progress || 0)}%
            </span>
            {task.status === 'downloading' && (
              <span className="text-xs text-glass-500">
                {currentStage.name}
              </span>
            )}
          </div>
        </div>
        
        {/* Thicker Progress Bar with Glow Effect */}
        <div className="relative w-full bg-glass-200/20 rounded-full h-6 overflow-hidden shadow-inner">
          <div
            className={`bg-gradient-to-r ${getStatusColor(task.status)} h-6 rounded-full transition-all duration-500 ease-out shadow-lg`}
            style={{ width: `${task.progress || 0}%` }}
          >
            {/* Percentage Overlay */}
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="text-xs font-bold text-white drop-shadow-lg">
                {Math.round(task.progress || 0)}%
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Detailed Stages */}
      {showDetails && task.status === 'downloading' && (
        <div className="space-y-2">
          <div className="text-xs text-glass-400 font-medium mb-2">
            Progress Details:
          </div>
          <div className="grid grid-cols-2 gap-2">
            {stages.map((stage, index) => (
              <div
                key={index}
                className={`flex items-center space-x-2 p-2 rounded-lg transition-all duration-300 ${
                  stage.status === 'active'
                    ? 'bg-blue-500/20 border border-blue-500/30'
                    : stage.status === 'completed'
                    ? 'bg-green-500/20 border border-green-500/30'
                    : 'bg-glass-200/10 border border-glass-300/20'
                }`}
              >
                <div
                  className={`${
                    stage.status === 'active'
                      ? 'text-blue-400'
                      : stage.status === 'completed'
                      ? 'text-green-400'
                      : 'text-glass-500'
                  }`}
                >
                  {stage.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium text-glass-300 truncate">
                    {stage.name}
                  </div>
                  <div className="text-xs text-glass-500 truncate">
                    {stage.description}
                  </div>
                </div>
                <div className="text-xs text-glass-400">
                  {Math.round(stage.progress)}%
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Error Message */}
      {task.status === 'failed' && task.error && (
        <div className="mt-2 p-3 bg-red-500/10 border border-red-500/30 rounded-lg">
          <div className="flex items-start space-x-2">
            <AlertCircle className="w-4 h-4 text-red-400 mt-0.5 flex-shrink-0" />
            <div className="text-sm text-red-300">
              <div className="font-medium">Download Failed</div>
              <div className="text-red-400 text-xs mt-1">{task.error}</div>
            </div>
          </div>
        </div>
      )}

      {/* Time Information */}
      {task.status === 'downloading' && (
        <div className="flex items-center justify-between text-xs text-glass-500">
          <div className="flex items-center space-x-4">
            {task.started_at && (
              <span>
                Started: {new Date(task.started_at).toLocaleTimeString()}
              </span>
            )}
            {task.completed_at && (
              <span>
                Completed: {new Date(task.completed_at).toLocaleTimeString()}
              </span>
            )}
          </div>
          <div className="text-glass-400">
            Order: #{task.order}
          </div>
        </div>
      )}
    </div>
  )
}

export default EnhancedProgressBar
