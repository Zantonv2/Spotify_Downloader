import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import CustomTitleBar from './components/CustomTitleBar'
import Header from './components/Header'
import SearchBar from './components/SearchBar'
import EnhancedResultsList from './components/EnhancedResultsList'
import DownloadQueue from './components/DownloadQueue'
import SettingsModal from './components/SettingsModal'
import SpotifyImport from './components/SpotifyImport'
import { useKeyboardShortcuts, createAppShortcuts } from './hooks/useKeyboardShortcuts'
import { useToast } from './hooks/useToast'
import ToastContainer from './components/Toast'
import { TrackInfo, DownloadTask } from './types'

function App() {
  const [searchResults, setSearchResults] = useState<TrackInfo[]>([])
  const [downloadQueue, setDownloadQueue] = useState<DownloadTask[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [showSettings, setShowSettings] = useState(false)
  const [activeTab, setActiveTab] = useState<'search' | 'queue' | 'import'>('search')
  const searchInputRef = useRef<HTMLInputElement>(null)
  const { toasts, removeToast, success } = useToast()

  const loadDownloadQueue = async () => {
    try {
      const queue = await invoke<DownloadTask[]>('get_download_queue')
      setDownloadQueue(queue)
    } catch (error) {
      console.error('Failed to load download queue:', error)
    }
  }

  const handleSearch = async (query: string, deepSearch: boolean = false) => {
    if (!query.trim()) return

    setIsSearching(true)
    try {
      let result
      if (deepSearch) {
        result = await invoke<any>('deep_search_tracks', {
          query: query.trim(),
          limit: 20
        })
      } else {
        result = await invoke<any>('search_tracks', {
          request: {
            query: query.trim(),
            limit: 20,
            sources: []
          }
        })
      }
      setSearchResults(result.tracks || [])
    } catch (error) {
      console.error('Search failed:', error)
    } finally {
      setIsSearching(false)
    }
  }

  const handleDownload = async (track: TrackInfo) => {
    try {
      await invoke('download_track', {
        request: {
          track_id: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album,
          url: track.url,
          source: track.source
        }
      })
      // Reload queue to show new download
      loadDownloadQueue()
    } catch (error) {
      console.error('Download failed:', error)
    }
  }


  const handleRemoveDownload = async (taskId: string) => {
    try {
      await invoke('remove_from_queue', { taskId })
      loadDownloadQueue()
    } catch (error) {
      console.error('Failed to remove download:', error)
    }
  }

  const handleDownloadAll = async () => {
    try {
      await invoke('download_all_pending')
      success('Download Started', `Started downloading ${downloadQueue.length} tracks`)
      loadDownloadQueue()
    } catch (error) {
      console.error('Failed to download all:', error)
      console.error('Download Failed: Failed to start downloading all tracks')
    }
  }

  const handlePauseAll = async () => {
    try {
      await invoke('pause_all_downloads')
      loadDownloadQueue()
    } catch (error) {
      console.error('Failed to pause all:', error)
    }
  }

  const handleResumeAll = async () => {
    try {
      await invoke('resume_all_downloads')
      loadDownloadQueue()
    } catch (error) {
      console.error('Failed to resume all:', error)
    }
  }

  const handleClearList = async () => {
    try {
      await invoke('clear_download_queue')
      loadDownloadQueue()
    } catch (error) {
      console.error('Failed to clear list:', error)
    }
  }



  // Load download queue on app start
  useEffect(() => {
    loadDownloadQueue()
  }, [])

  // Keyboard shortcuts
  const shortcuts = createAppShortcuts({
    onSearch: () => {
      if (activeTab === 'search' && searchInputRef.current) {
        searchInputRef.current.focus()
      }
    },
    onDownloadAll: handleDownloadAll,
    onPauseAll: handlePauseAll,
    onResumeAll: handleResumeAll,
    onSettings: () => setShowSettings(true),
    onRefresh: loadDownloadQueue,
    onClearQueue: handleClearList,
    onTabSearch: () => setActiveTab('search'),
    onTabQueue: () => setActiveTab('queue'),
    onTabImport: () => setActiveTab('import')
  })

  useKeyboardShortcuts({ shortcuts, enabled: true })

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      <CustomTitleBar title="Spotify Downloader" />
      <Header 
        onSettingsClick={() => setShowSettings(true)}
        onTabChange={setActiveTab}
        activeTab={activeTab}
        downloadCount={downloadQueue.length}
      />
      
      <main className="container mx-auto px-4 py-8 max-w-6xl">
        {activeTab === 'search' ? (
          <div className="space-y-8">
            <SearchBar 
              onSearch={handleSearch}
              isSearching={isSearching}
            />
            <EnhancedResultsList 
              results={searchResults}
              onDownload={handleDownload}
              isSearching={isSearching}
            />
          </div>
        ) : activeTab === 'import' ? (
          <SpotifyImport 
            onImportComplete={async (tracks) => {
              console.log('onImportComplete called with tracks:', tracks)
              // Add imported tracks directly to download queue
              try {
                console.log('Calling bulk_download_spotify_tracks with', tracks.length, 'tracks')
                const result = await invoke('bulk_download_spotify_tracks', {
                  tracks: tracks
                })
                console.log('bulk_download_spotify_tracks result:', result)
                // Reload queue to show new downloads
                console.log('Reloading download queue...')
                loadDownloadQueue()
                // Switch to downloads tab to show the queued tracks
                console.log('Switching to queue tab...')
                setActiveTab('queue')
              } catch (error) {
                console.error('Failed to add tracks to download queue:', error)
              }
            }}
          />
        ) : (
          <DownloadQueue
            queue={downloadQueue}
            onRemove={handleRemoveDownload}
            onRefresh={loadDownloadQueue}
            onDownloadAll={handleDownloadAll}
            onClearList={handleClearList}
          />
        )}
      </main>

      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
        />
      )}
      
      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onRemove={removeToast} />
    </div>
  )
}

export default App
