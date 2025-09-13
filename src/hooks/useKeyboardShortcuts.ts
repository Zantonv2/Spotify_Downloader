import { useEffect, useCallback } from 'react'

interface KeyboardShortcut {
  key: string
  ctrlKey?: boolean
  shiftKey?: boolean
  altKey?: boolean
  metaKey?: boolean
  action: () => void
  description: string
}

interface UseKeyboardShortcutsProps {
  shortcuts: KeyboardShortcut[]
  enabled?: boolean
}

export const useKeyboardShortcuts = ({ 
  shortcuts, 
  enabled = true 
}: UseKeyboardShortcutsProps) => {
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return

    const pressedKey = event.key.toLowerCase()
    const isCtrl = event.ctrlKey || event.metaKey
    const isShift = event.shiftKey
    const isAlt = event.altKey

    // Find matching shortcut
    const matchingShortcut = shortcuts.find(shortcut => {
      const keyMatch = shortcut.key.toLowerCase() === pressedKey
      const ctrlMatch = !!shortcut.ctrlKey === isCtrl
      const shiftMatch = !!shortcut.shiftKey === isShift
      const altMatch = !!shortcut.altKey === isAlt

      return keyMatch && ctrlMatch && shiftMatch && altMatch
    })

    if (matchingShortcut) {
      event.preventDefault()
      event.stopPropagation()
      matchingShortcut.action()
    }
  }, [shortcuts, enabled])

  useEffect(() => {
    if (enabled) {
      document.addEventListener('keydown', handleKeyDown)
      return () => {
        document.removeEventListener('keydown', handleKeyDown)
      }
    }
  }, [handleKeyDown, enabled])
}

// Common shortcut definitions
export const createAppShortcuts = (actions: {
  onSearch?: () => void
  onDownloadAll?: () => void
  onPauseAll?: () => void
  onResumeAll?: () => void
  onSettings?: () => void
  onRefresh?: () => void
  onClearQueue?: () => void
  onSelectAll?: () => void
  onClearSelection?: () => void
  onTabSearch?: () => void
  onTabQueue?: () => void
  onTabImport?: () => void
}) => {
  const shortcuts: KeyboardShortcut[] = []

  if (actions.onSearch) {
    shortcuts.push({
      key: 'k',
      ctrlKey: true,
      action: actions.onSearch,
      description: 'Focus search (Ctrl+K)'
    })
  }

  if (actions.onDownloadAll) {
    shortcuts.push({
      key: 'd',
      ctrlKey: true,
      action: actions.onDownloadAll,
      description: 'Download all (Ctrl+D)'
    })
  }

  if (actions.onPauseAll) {
    shortcuts.push({
      key: 'p',
      ctrlKey: true,
      action: actions.onPauseAll,
      description: 'Pause all (Ctrl+P)'
    })
  }

  if (actions.onResumeAll) {
    shortcuts.push({
      key: 'r',
      ctrlKey: true,
      action: actions.onResumeAll,
      description: 'Resume all (Ctrl+R)'
    })
  }

  if (actions.onSettings) {
    shortcuts.push({
      key: ',',
      ctrlKey: true,
      action: actions.onSettings,
      description: 'Open settings (Ctrl+,)'
    })
  }

  if (actions.onRefresh) {
    shortcuts.push({
      key: 'f5',
      action: actions.onRefresh,
      description: 'Refresh (F5)'
    })
  }

  if (actions.onClearQueue) {
    shortcuts.push({
      key: 'x',
      ctrlKey: true,
      shiftKey: true,
      action: actions.onClearQueue,
      description: 'Clear queue (Ctrl+Shift+X)'
    })
  }

  if (actions.onSelectAll) {
    shortcuts.push({
      key: 'a',
      ctrlKey: true,
      action: actions.onSelectAll,
      description: 'Select all (Ctrl+A)'
    })
  }

  if (actions.onClearSelection) {
    shortcuts.push({
      key: 'escape',
      action: actions.onClearSelection,
      description: 'Clear selection (Escape)'
    })
  }

  if (actions.onTabSearch) {
    shortcuts.push({
      key: '1',
      ctrlKey: true,
      action: actions.onTabSearch,
      description: 'Search tab (Ctrl+1)'
    })
  }

  if (actions.onTabQueue) {
    shortcuts.push({
      key: '2',
      ctrlKey: true,
      action: actions.onTabQueue,
      description: 'Queue tab (Ctrl+2)'
    })
  }

  if (actions.onTabImport) {
    shortcuts.push({
      key: '3',
      ctrlKey: true,
      action: actions.onTabImport,
      description: 'Import tab (Ctrl+3)'
    })
  }

  return shortcuts
}

export default useKeyboardShortcuts
