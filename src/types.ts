export interface TrackInfo {
  id: string
  title: string
  artist: string
  album?: string
  duration?: number
  year?: number
  genre?: string
  thumbnail_url?: string
  source: string
  url: string
  isrc?: string
  album_artist?: string
  track_number?: number
  disc_number?: number
  composer?: string
}

export interface DownloadTask {
  id: string
  track_info: TrackInfo
  output_path: string
  status: DownloadStatus
  progress: number
  error?: string
  created_at: string
  started_at?: string
  completed_at?: string
  order: number // Track order in playlist/queue
}

export type DownloadStatus = 
  | 'pending'
  | 'downloading'
  | 'processing'
  | 'completed'
  | 'failed'
  | 'paused'
  | 'cancelled'

export interface DownloadProgress {
  task_id: string
  status: DownloadStatus
  progress: number
  current_speed?: number
  estimated_time_remaining?: number
  downloaded_bytes?: number
  total_bytes?: number
}

export interface AppConfig {
  proxy: any
  download_path: string
  max_concurrent_downloads: number
  preferred_quality: string
  preferred_format: string
  enable_metadata: boolean
  enable_lyrics: boolean
  enable_cover_art: boolean
  api_keys: {
    spotify_client_id?: string
    spotify_client_secret?: string
    musicbrainz_user_agent?: string
    musixmatch_client_id?: string
    musixmatch_client_secret?: string
    genius_client_id?: string
    genius_client_secret?: string
    deezer_api_key?: string
  }
  ui: {
    theme: string
    show_notifications: boolean
    auto_start_downloads: boolean
    minimize_to_tray: boolean
  }
}

export interface SearchResult {
  tracks: TrackInfo[]
  total: number
  sources_used: string[]
  deduplicated: boolean
}
