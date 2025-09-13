import React, { useState, useEffect } from 'react'
import { Music, Upload, Link, FileText, Download, AlertCircle, CheckCircle } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

interface SpotifyImportProps {
  onImportComplete: (tracks: any[]) => void
}

const SpotifyImport: React.FC<SpotifyImportProps> = ({ onImportComplete }) => {
  const [activeTab, setActiveTab] = useState<'url' | 'csv'>('url')
  const [spotifyUrl, setSpotifyUrl] = useState('')
  const [csvFile, setCsvFile] = useState<string | null>(null)
  const [isImporting, setIsImporting] = useState(false)
  const [importedTracks, setImportedTracks] = useState<any[]>([])
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [credentials, setCredentials] = useState<{clientId: string, clientSecret: string} | null>(null)
  const [credentialsError, setCredentialsError] = useState<string | null>(null)

  // Load credentials from settings on component mount
  useEffect(() => {
    const loadCredentials = async () => {
      try {
        const config = await invoke('get_settings')
        console.log('Loaded config from get_settings:', config)
        const configData = config as any
        console.log('Config data structure:', configData)
        console.log('API keys:', configData.api_keys)
        console.log('Spotify client ID:', configData.api_keys?.spotify_client_id)
        console.log('Spotify client secret:', configData.api_keys?.spotify_client_secret)
        
        if (configData.api_keys?.spotify_client_id && configData.api_keys?.spotify_client_secret) {
          setCredentials({
            clientId: configData.api_keys.spotify_client_id,
            clientSecret: configData.api_keys.spotify_client_secret
          })
          console.log('Credentials set successfully')
        } else {
          setCredentialsError('Spotify credentials not found in settings. Please configure them in Settings &gt; API Keys.')
          console.log('Credentials not found in config')
        }
      } catch (err) {
        setCredentialsError('Failed to load settings. Please check your configuration.')
        console.error('Error loading settings:', err)
      }
    }
    loadCredentials()
  }, [])

  const handleUrlImport = async () => {
    if (!spotifyUrl.trim()) {
      setError('Please enter a Spotify URL')
      return
    }

    if (!credentials) {
      setError('Spotify credentials not configured. Please set them in Settings > API Keys.')
      return
    }

    setIsImporting(true)
    setError(null)
    setSuccess(null)

    try {
      const result = await invoke('import_spotify_url', {
        url: spotifyUrl.trim(),
        clientId: credentials.clientId,
        clientSecret: credentials.clientSecret
      })

      const data = result as any
      
      // Handle different response types
      if (data.tracks && data.tracks.length > 0) {
        // Playlist or album with tracks
        console.log('Spotify URL import result (tracks):', data)
        setImportedTracks(data.tracks)
        setSuccess(`Successfully imported ${data.type}: ${data.data.name || data.data.title} (${data.tracks.length} tracks)`)
        // Add tracks to downloads queue
        console.log('Calling onImportComplete with tracks:', data.tracks)
        onImportComplete(data.tracks)
      } else if (data.data) {
        // Single track
        console.log('Spotify URL import result (single track):', data)
        setImportedTracks([data.data])
        setSuccess(`Successfully imported ${data.type}: ${data.data.name || data.data.title}`)
        // Add tracks to downloads queue
        console.log('Calling onImportComplete with single track:', [data.data])
        onImportComplete([data.data])
      } else {
        setError('No tracks found in the response')
      }
    } catch (err) {
      setError(`Import failed: ${err}`)
    } finally {
      setIsImporting(false)
    }
  }

  const handleCsvImport = async () => {
    if (!csvFile) {
      setError('Please select a CSV file')
      return
    }

    setIsImporting(true)
    setError(null)
    setSuccess(null)

    try {
      const result = await invoke('import_csv_playlist', {
        filePath: csvFile
      })

      const data = result as any
      console.log('CSV import result:', data)
      setImportedTracks(data.tracks)
      setSuccess(`Successfully imported ${data.total} tracks from CSV`)
      // Add tracks to downloads queue
      console.log('Calling onImportComplete with tracks:', data.tracks)
      onImportComplete(data.tracks)
    } catch (err) {
      setError(`CSV import failed: ${err}`)
    } finally {
      setIsImporting(false)
    }
  }

  const handleFileSelect = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'CSV Files',
          extensions: ['csv']
        }]
      })

      if (selected) {
        setCsvFile(selected as string)
        setError(null)
      }
    } catch (err) {
      setError(`File selection failed: ${err}`)
    }
  }

  const handleBulkDownload = async () => {
    if (importedTracks.length === 0) {
      setError('No tracks to download')
      return
    }

    setIsImporting(true)
    setError(null)

    try {
      const result = await invoke('bulk_download_spotify_tracks', {
        tracks: importedTracks
      })

      const data = result as any
      setSuccess(`Started downloading ${data.total} tracks`)
      // Tracks are already in the queue, no need to add them again
    } catch (err) {
      setError(`Bulk download failed: ${err}`)
    } finally {
      setIsImporting(false)
    }
  }

  const formatDuration = (seconds?: number) => {
    if (!seconds) return 'Unknown'
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, '0')}`
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="text-center">
        <h2 className="text-3xl font-bold gradient-text mb-2">
          Import from Spotify
        </h2>
        <p className="text-glass-400">
          Import playlists, albums, or tracks using URLs or CSV files
        </p>
      </div>

      {/* Tab Navigation */}
      <div className="flex space-x-1 bg-glass-200/20 rounded-lg p-1">
        <button
          onClick={() => setActiveTab('url')}
          className={`flex-1 flex items-center justify-center space-x-2 py-2 px-4 rounded-md transition-all ${
            activeTab === 'url'
              ? 'bg-purple-600 text-white'
              : 'text-glass-400 hover:text-white'
          }`}
        >
          <Link className="w-4 h-4" />
          <span>Spotify URL</span>
        </button>
        <button
          onClick={() => setActiveTab('csv')}
          className={`flex-1 flex items-center justify-center space-x-2 py-2 px-4 rounded-md transition-all ${
            activeTab === 'csv'
              ? 'bg-purple-600 text-white'
              : 'text-glass-400 hover:text-white'
          }`}
        >
          <FileText className="w-4 h-4" />
          <span>CSV File</span>
        </button>
      </div>

      {/* URL Import Tab */}
      {activeTab === 'url' && (
        <div className="glass-card space-y-4">
          <h3 className="text-xl font-semibold text-white mb-4">Import from Spotify URL</h3>
          
          <div className="space-y-4">
            {/* Credentials Status - Only show errors */}
            {credentialsError && (
              <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-3">
                <div className="flex items-center space-x-2 text-red-400">
                  <AlertCircle className="w-5 h-5" />
                  <span>{credentialsError}</span>
                </div>
                <p className="text-red-300 text-sm mt-2">
                  Please configure your Spotify credentials in Settings &gt; API Keys first.
                </p>
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-glass-300 mb-2">
                Spotify URL
              </label>
              <input
                type="url"
                value={spotifyUrl}
                onChange={(e) => setSpotifyUrl(e.target.value)}
                placeholder="https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"
                className="w-full px-4 py-3 bg-glass-200/30 border border-glass-300/30 rounded-lg text-white placeholder-glass-400 focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <button
              onClick={handleUrlImport}
              disabled={isImporting || !credentials}
              className="w-full gradient-button flex items-center justify-center space-x-2 py-3 disabled:opacity-50"
            >
              {isImporting ? (
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
              ) : (
                <Upload className="w-4 h-4" />
              )}
              <span>{isImporting ? 'Importing...' : 'Import from URL'}</span>
            </button>
          </div>
        </div>
      )}

      {/* CSV Import Tab */}
      {activeTab === 'csv' && (
        <div className="glass-card space-y-4">
          <h3 className="text-xl font-semibold text-white mb-4">Import from CSV File</h3>
          
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-glass-300 mb-2">
                CSV File (from Exportify)
              </label>
              <div className="flex space-x-4">
                <button
                  onClick={handleFileSelect}
                  className="flex-1 gradient-button flex items-center justify-center space-x-2 py-3"
                >
                  <FileText className="w-4 h-4" />
                  <span>Select CSV File</span>
                </button>
                {csvFile && (
                  <div className="flex-1 px-4 py-3 bg-glass-200/30 border border-glass-300/30 rounded-lg text-white truncate">
                    {csvFile.split('\\').pop()}
                  </div>
                )}
              </div>
            </div>

            <button
              onClick={handleCsvImport}
              disabled={isImporting || !csvFile}
              className="w-full gradient-button flex items-center justify-center space-x-2 py-3 disabled:opacity-50"
            >
              {isImporting ? (
                <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
              ) : (
                <Upload className="w-4 h-4" />
              )}
              <span>{isImporting ? 'Importing...' : 'Import from CSV'}</span>
            </button>
          </div>
        </div>
      )}

      {/* Status Messages */}
      {error && (
        <div className="glass-card bg-red-500/10 border border-red-500/30">
          <div className="flex items-center space-x-2 text-red-400">
            <AlertCircle className="w-5 h-5" />
            <span>{error}</span>
          </div>
        </div>
      )}

      {success && (
        <div className="glass-card bg-green-500/10 border border-green-500/30">
          <div className="flex items-center space-x-2 text-green-400">
            <CheckCircle className="w-5 h-5" />
            <span>{success}</span>
          </div>
        </div>
      )}

      {/* Imported Tracks Preview */}
      {importedTracks.length > 0 && (
        <div className="glass-card">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-xl font-semibold text-white">
              Imported Tracks ({importedTracks.length})
            </h3>
            <button
              onClick={handleBulkDownload}
              disabled={isImporting}
              className="gradient-button flex items-center space-x-2 px-4 py-2 disabled:opacity-50"
            >
              <Download className="w-4 h-4" />
              <span>Download All</span>
            </button>
          </div>

          <div className="space-y-2 max-h-64 overflow-y-auto">
            {importedTracks.map((track, index) => (
              <div
                key={index}
                className="flex items-center space-x-4 p-3 bg-glass-200/20 rounded-lg"
              >
                <div className="flex items-center space-x-2">
                  <div className="w-6 h-6 bg-purple-600 text-white text-xs rounded-full flex items-center justify-center font-medium">
                    {track.track_number || index + 1}
                  </div>
                  <Music className="w-5 h-5 text-purple-400" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-white font-medium truncate">
                    {track.title || track.name}
                  </div>
                  <div className="text-glass-400 text-sm truncate">
                    {track.artist || track.artists?.[0]?.name} • {track.album || track.album?.name}
                  </div>
                </div>
                {track.duration && (
                  <div className="text-glass-400 text-sm">
                    {formatDuration(track.duration)}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

export default SpotifyImport
