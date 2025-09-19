import React, { useState, useEffect } from 'react'
import { X, Save, FolderOpen, Key, Music, Download, Settings } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { AppConfig } from '../types'

interface SettingsModalProps {
  onClose: () => void
}

const SettingsModal: React.FC<SettingsModalProps> = ({ onClose }) => {
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [activeTab, setActiveTab] = useState<'general' | 'downloads' | 'api'>('general')
  const [ffmpegAvailable, setFfmpegAvailable] = useState<boolean | null>(null)
  const [ffmpegPath, setFfmpegPath] = useState<string | null>(null)
  const [checkingFfmpeg, setCheckingFfmpeg] = useState(false)

  useEffect(() => {
    loadSettings()
  }, [])

  // Lock body scroll when modal is open
  useEffect(() => {
    const originalOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = originalOverflow
    }
  }, [])

  const loadSettings = async () => {
    try {
      const settings = await invoke<AppConfig>('get_settings')
      setConfig(settings)
      
      // Only check FFmpeg availability if not already checked
      if (ffmpegAvailable === null) {
        await checkFfmpegStatus()
      }
    } catch (error) {
      console.error('Failed to load settings:', error)
    } finally {
      setLoading(false)
    }
  }

  const checkFfmpegStatus = async () => {
    setCheckingFfmpeg(true)
    try {
      const available = await invoke<boolean>('check_ffmpeg_availability')
      setFfmpegAvailable(available)
      
      if (available) {
        const path = await invoke<string | null>('get_ffmpeg_path')
        setFfmpegPath(path)
      }
      // FFmpeg status is now cached in state
    } catch (error) {
      console.error('Failed to check FFmpeg status:', error)
    } finally {
      setCheckingFfmpeg(false)
    }
  }

  const refreshFfmpegStatus = async () => {
    setFfmpegAvailable(null)
    setFfmpegPath(null)
    await checkFfmpegStatus()
  }

  const saveSettings = async () => {
    if (!config) return

    setSaving(true)
    try {
      await invoke('update_settings', {
        update: {
          download_path: config.download_path,
          max_concurrent_downloads: config.max_concurrent_downloads,
          preferred_quality: config.preferred_quality,
          preferred_format: config.preferred_format,
          enable_metadata: config.enable_metadata,
          enable_lyrics: config.enable_lyrics,
          enable_cover_art: config.enable_cover_art,
          spotify_client_id: config.api_keys.spotify_client_id,
          spotify_client_secret: config.api_keys.spotify_client_secret,
          musixmatch_client_id: config.api_keys.musixmatch_client_id,
          musixmatch_client_secret: config.api_keys.musixmatch_client_secret,
          genius_client_id: config.api_keys.genius_client_id,
          genius_client_secret: config.api_keys.genius_client_secret,
          deezer_api_key: config.api_keys.deezer_api_key,
          // UI settings
          theme: config.ui.theme,
          show_notifications: config.ui.show_notifications,
          auto_start_downloads: config.ui.auto_start_downloads,
          minimize_to_tray: config.ui.minimize_to_tray,
          // Network settings
          proxy: config.proxy,
        }
      })
      onClose()
    } catch (error) {
      console.error('Failed to save settings:', error)
    } finally {
      setSaving(false)
    }
  }

  const handleInputChange = (path: string, value: any) => {
    if (!config) return

    const keys = path.split('.')
    const newConfig = { ...config }
    let current: any = newConfig

    for (let i = 0; i < keys.length - 1; i++) {
      current = current[keys[i]]
    }

    current[keys[keys.length - 1]] = value
    setConfig(newConfig)
  }

  const handleBrowseFolder = async () => {
    try {
      const selectedPath = await invoke<string | null>('browse_folder')
      if (selectedPath) {
        handleInputChange('download_path', selectedPath)
      }
    } catch (error) {
      console.error('Failed to browse folder:', error)
    }
  }

  if (loading) {
    return (
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
        <div className="bg-glass-100/20 backdrop-blur-md border border-glass-200/30 rounded-xl p-8 shadow-2xl max-w-md w-full mx-4">
          <div className="text-center">
            {/* Animated Settings Icon */}
            <div className="relative mb-6">
              <div className="w-16 h-16 mx-auto bg-gradient-to-br from-purple-500 to-pink-500 rounded-full flex items-center justify-center animate-pulse">
                <Settings className="w-8 h-8 text-white animate-spin" />
              </div>
              <div className="absolute inset-0 w-16 h-16 mx-auto border-2 border-purple-400/30 rounded-full animate-ping"></div>
            </div>
            
            {/* Loading text */}
            <div className="space-y-2">
              <p className="text-white text-lg font-medium">Loading Settings</p>
              <p className="text-glass-400 text-sm">Preparing your configuration...</p>
              
              {/* Progress dots */}
              <div className="flex justify-center space-x-1 mt-4">
                <div className="w-2 h-2 bg-purple-500 rounded-full animate-bounce"></div>
                <div className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" style={{ animationDelay: '0.1s' }}></div>
                <div className="w-2 h-2 bg-purple-500 rounded-full animate-bounce" style={{ animationDelay: '0.2s' }}></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    )
  }

  if (!config) {
    return (
      <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
        <div className="glass-card max-w-md w-full mx-4">
          <div className="text-center py-8">
            <p className="text-red-400">Failed to load settings</p>
            <button onClick={onClose} className="glass-button mt-4">
              Close
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4 overflow-hidden">
      <div className="glass-card max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-glass-300/30 pb-4 mb-6">
          <h2 className="text-2xl font-bold gradient-text">Settings</h2>
          <button
            onClick={onClose}
            className="glass-button p-2"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex gap-6 flex-1 min-h-0">
          {/* Sidebar */}
          <div className="w-48 space-y-2 flex-shrink-0">
            <button
              onClick={() => setActiveTab('general')}
              className={`w-full text-left px-4 py-3 rounded-lg transition-all ${
                activeTab === 'general'
                  ? 'bg-purple-600/30 text-purple-300'
                  : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
              }`}
            >
              <div className="flex items-center space-x-3">
                <Music className="w-4 h-4" />
                <span>General</span>
              </div>
            </button>
            <button
              onClick={() => setActiveTab('downloads')}
              className={`w-full text-left px-4 py-3 rounded-lg transition-all ${
                activeTab === 'downloads'
                  ? 'bg-purple-600/30 text-purple-300'
                  : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
              }`}
            >
              <div className="flex items-center space-x-3">
                <Download className="w-4 h-4" />
                <span>Downloads</span>
              </div>
            </button>
            <button
              onClick={() => setActiveTab('api')}
              className={`w-full text-left px-4 py-3 rounded-lg transition-all ${
                activeTab === 'api'
                  ? 'bg-purple-600/30 text-purple-300'
                  : 'text-glass-400 hover:text-white hover:bg-glass-200/20'
              }`}
            >
              <div className="flex items-center space-x-3">
                <Key className="w-4 h-4" />
                <span>API Keys</span>
              </div>
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto min-h-0">
            {activeTab === 'general' && (
              <div className="space-y-6">
                <h3 className="text-xl font-semibold text-white">General Settings</h3>
                
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Theme
                    </label>
                    <select
                      value={config.ui.theme}
                      onChange={(e) => handleInputChange('ui.theme', e.target.value)}
                      className="glass-input"
                    >
                      <option value="dark">Dark</option>
                      <option value="light">Light</option>
                    </select>
                  </div>

                  <div className="space-y-3">
                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.ui.show_notifications}
                        onChange={(e) => handleInputChange('ui.show_notifications', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Show notifications</span>
                    </label>

                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.ui.auto_start_downloads}
                        onChange={(e) => handleInputChange('ui.auto_start_downloads', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Auto-start downloads</span>
                    </label>

                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.ui.minimize_to_tray}
                        onChange={(e) => handleInputChange('ui.minimize_to_tray', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Minimize to system tray</span>
                    </label>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Proxy (for Spotify imports)
                    </label>
                    <input
                      type="text"
                      value={config.proxy || ''}
                      onChange={(e) => handleInputChange('proxy', e.target.value)}
                      placeholder="http://127.0.0.1:1080"
                      className="glass-input"
                    />
                    <p className="text-xs text-glass-400 mt-1">
                      Leave empty to disable proxy. Format: http://host:port
                    </p>
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'downloads' && (
              <div className="space-y-6">
                <h3 className="text-xl font-semibold text-white">Download Settings</h3>
                
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Download Path
                    </label>
                    <div className="flex space-x-2">
                      <input
                        type="text"
                        value={config.download_path}
                        onChange={(e) => handleInputChange('download_path', e.target.value)}
                        className="glass-input flex-1"
                      />
                      <button 
                        className="glass-button px-4"
                        onClick={handleBrowseFolder}
                      >
                        <FolderOpen className="w-4 h-4" />
                      </button>
                    </div>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Max Concurrent Downloads
                    </label>
                    <input
                      type="number"
                      min="1"
                      max="10"
                      value={config.max_concurrent_downloads}
                      onChange={(e) => handleInputChange('max_concurrent_downloads', parseInt(e.target.value))}
                      className="glass-input"
                    />
                  </div>

                  {/* Quality settings for different formats */}
                  {config.preferred_format !== 'flac' && config.preferred_format !== 'wav' && (
                    <div>
                      <label className="block text-sm font-medium text-glass-300 mb-2">
                        Preferred Quality
                      </label>
                      <select
                        value={config.preferred_quality}
                        onChange={(e) => handleInputChange('preferred_quality', e.target.value)}
                        className="glass-input text-white"
                      >
                        {config.preferred_format === 'ogg' ? (
                          <>
                            <option value="low" className="text-slate-900">Low (Quality 2 ~128kbps)</option>
                            <option value="medium" className="text-slate-900">Medium (Quality 4 ~192kbps)</option>
                            <option value="high" className="text-slate-900">High (Quality 6 ~256kbps)</option>
                            <option value="best" className="text-slate-900">Best (Quality 8 ~320kbps)</option>
                            <option value="lossless" className="text-slate-900">Lossless (FLAC in OGG)</option>
                          </>
                        ) : config.preferred_format === 'ape' ? (
                          <>
                            <option value="lossless" className="text-slate-900">Lossless (APE)</option>
                          </>
                        ) : (
                          <>
                            <option value="low" className="text-slate-900">Low (128 kbps)</option>
                            <option value="medium" className="text-slate-900">Medium (192 kbps)</option>
                            <option value="high" className="text-slate-900">High (256 kbps)</option>
                            <option value="best" className="text-slate-900">Best (320 kbps)</option>
                            <option value="lossless" className="text-slate-900">Lossless</option>
                          </>
                        )}
                      </select>
                      {config.preferred_format === 'ogg' && (
                        <p className="text-xs text-glass-400 mt-1">
                          OGG Vorbis uses quality settings (0-10) for lossy, FLAC for lossless
                        </p>
                      )}
                      {config.preferred_format === 'opus' && (
                        <p className="text-xs text-glass-400 mt-1">
                          Opus is a highly efficient lossy format, great for streaming
                        </p>
                      )}
                      {config.preferred_format === 'ape' && (
                        <p className="text-xs text-glass-400 mt-1">
                          APE (Monkey's Audio) is a lossless format with high compression
                        </p>
                      )}
                    </div>
                  )}

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Preferred Format
                    </label>
                    <select
                      value={config.preferred_format}
                      onChange={(e) => handleInputChange('preferred_format', e.target.value)}
                      className="glass-input text-white"
                    >
                        <option value="mp3" className="text-slate-900">MP3</option>
                        <option value="m4a" className="text-slate-900">M4A</option>
                        <option value="flac" className="text-slate-900">FLAC</option>
                        <option value="wav" className="text-slate-900">WAV</option>
                        <option value="ogg" className="text-slate-900">OGG</option>
                        <option value="opus" className="text-slate-900">Opus</option>
                        <option value="ape" className="text-slate-900">APE</option>
                    </select>
                  </div>

                  <div className="space-y-3">
                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.enable_metadata}
                        onChange={(e) => handleInputChange('enable_metadata', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Enable metadata embedding</span>
                    </label>

                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.enable_lyrics}
                        onChange={(e) => handleInputChange('enable_lyrics', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Enable lyrics embedding</span>
                    </label>

                    <label className="flex items-center space-x-3">
                      <input
                        type="checkbox"
                        checked={config.enable_cover_art}
                        onChange={(e) => handleInputChange('enable_cover_art', e.target.checked)}
                        className="rounded"
                      />
                      <span className="text-glass-300">Enable cover art download</span>
                    </label>
                  </div>

                  {/* FFmpeg Status */}
                  <div className="mt-6 p-4 rounded-lg border border-glass-300/30">
                    <div className="flex items-center justify-between mb-3">
                      <h4 className="text-lg font-semibold text-white">FFmpeg Status</h4>
                      <button
                        onClick={refreshFfmpegStatus}
                        disabled={checkingFfmpeg}
                        className="glass-button px-3 py-1 text-sm disabled:opacity-50"
                      >
                        {checkingFfmpeg ? (
                          <div className="animate-spin w-4 h-4 border-2 border-white border-t-transparent rounded-full" />
                        ) : (
                          'Refresh'
                        )}
                      </button>
                    </div>
                    <div className="space-y-2">
                      <div className="flex items-center space-x-3">
                        <div className={`w-3 h-3 rounded-full ${
                          ffmpegAvailable === true ? 'bg-green-500' : 
                          ffmpegAvailable === false ? 'bg-red-500' : 'bg-yellow-500'
                        }`} />
                        <span className="text-glass-300">
                          {checkingFfmpeg ? 'Checking FFmpeg...' :
                           ffmpegAvailable === true ? 'FFmpeg is available' : 
                           ffmpegAvailable === false ? 'FFmpeg not found' : 'Not checked yet'}
                        </span>
                      </div>
                      {ffmpegPath && (
                        <div className="text-sm text-glass-400 ml-6">
                          Path: {ffmpegPath}
                        </div>
                      )}
                      {ffmpegAvailable === false && (
                        <div className="text-sm text-red-400 ml-6">
                          Please install FFmpeg and add it to your PATH, or place it in a common location.
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            )}

            {activeTab === 'api' && (
              <div className="space-y-6">
                <h3 className="text-xl font-semibold text-white">API Keys</h3>
                
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Spotify Client ID
                    </label>
                    <input
                      type="text"
                      value={config.api_keys.spotify_client_id || ''}
                      onChange={(e) => handleInputChange('api_keys.spotify_client_id', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Spotify Client ID"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Spotify Client Secret
                    </label>
                    <input
                      type="password"
                      value={config.api_keys.spotify_client_secret || ''}
                      onChange={(e) => handleInputChange('api_keys.spotify_client_secret', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Spotify Client Secret"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Musixmatch Client ID
                    </label>
                    <input
                      type="text"
                      value={config.api_keys.musixmatch_client_id || ''}
                      onChange={(e) => handleInputChange('api_keys.musixmatch_client_id', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Musixmatch Client ID"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Musixmatch Client Secret
                    </label>
                    <input
                      type="password"
                      value={config.api_keys.musixmatch_client_secret || ''}
                      onChange={(e) => handleInputChange('api_keys.musixmatch_client_secret', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Musixmatch Client Secret"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Genius Client ID
                    </label>
                    <input
                      type="text"
                      value={config.api_keys.genius_client_id || ''}
                      onChange={(e) => handleInputChange('api_keys.genius_client_id', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Genius Client ID"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Genius Client Secret
                    </label>
                    <input
                      type="password"
                      value={config.api_keys.genius_client_secret || ''}
                      onChange={(e) => handleInputChange('api_keys.genius_client_secret', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Genius Client Secret"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-glass-300 mb-2">
                      Deezer API Key
                    </label>
                    <input
                      type="password"
                      value={config.api_keys.deezer_api_key || ''}
                      onChange={(e) => handleInputChange('api_keys.deezer_api_key', e.target.value)}
                      className="glass-input"
                      placeholder="Enter your Deezer API key"
                    />
                  </div>

                  <div className="bg-blue-500/20 border border-blue-500/30 rounded-lg p-4">
                    <h4 className="text-blue-300 font-semibold mb-2">How to get API keys:</h4>
                    <div className="space-y-3">
                      <div>
                        <h5 className="text-blue-200 font-medium">Spotify:</h5>
                        <ol className="text-sm text-blue-200 space-y-1 list-decimal list-inside ml-2">
                          <li>Go to <a href="https://developer.spotify.com/dashboard" target="_blank" rel="noopener noreferrer" className="underline">Spotify Developer Dashboard</a></li>
                          <li>Log in with your Spotify account</li>
                          <li>Click "Create an App"</li>
                          <li>Fill in the app details and accept the terms</li>
                          <li>Copy the Client ID and Client Secret</li>
                        </ol>
                      </div>
                      <div>
                        <h5 className="text-blue-200 font-medium">Musixmatch:</h5>
                        <ol className="text-sm text-blue-200 space-y-1 list-decimal list-inside ml-2">
                          <li>Go to <a href="https://developer.musixmatch.com/" target="_blank" rel="noopener noreferrer" className="underline">Musixmatch Developer Portal</a></li>
                          <li>Sign up for a developer account</li>
                          <li>Create a new app and get your Client ID and Client Secret</li>
                        </ol>
                      </div>
                      <div>
                        <h5 className="text-blue-200 font-medium">Genius:</h5>
                        <ol className="text-sm text-blue-200 space-y-1 list-decimal list-inside ml-2">
                          <li>Go to <a href="https://genius.com/api-clients" target="_blank" rel="noopener noreferrer" className="underline">Genius API Clients</a></li>
                          <li>Sign up and create a new client</li>
                          <li>Copy your Client ID and Client Secret</li>
                        </ol>
                      </div>
                      <div>
                        <h5 className="text-blue-200 font-medium">Deezer:</h5>
                        <ol className="text-sm text-blue-200 space-y-1 list-decimal list-inside ml-2">
                          <li>Go to <a href="https://developers.deezer.com/" target="_blank" rel="noopener noreferrer" className="underline">Deezer Developers</a></li>
                          <li>Register your application</li>
                          <li>Get your app ID and secret</li>
                        </ol>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end space-x-4 pt-6 border-t border-glass-300/30 mt-6">
          <button
            onClick={onClose}
            className="glass-button"
          >
            Cancel
          </button>
          <button
            onClick={saveSettings}
            disabled={saving}
            className="gradient-button flex items-center space-x-2 disabled:opacity-50"
          >
            {saving ? (
              <div className="animate-spin w-4 h-4 border-2 border-white border-t-transparent rounded-full" />
            ) : (
              <Save className="w-4 h-4" />
            )}
            <span>{saving ? 'Saving...' : 'Save Settings'}</span>
          </button>
        </div>
      </div>
    </div>
  )
}

export default SettingsModal
