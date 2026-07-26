import CiGlyph from '@components/ci/CiGlyph'
import { GearIcon, MoonIcon, SunIcon } from '../../icons/TitlebarIcons'
import { useTheme } from '../../useTheme'
import type { ConnectionStatus } from '@transport/types'

type Props = {
  status: ConnectionStatus
  onOpenSettings: () => void
}

const Titlebar = ({ status, onOpenSettings }: Props): React.JSX.Element => {
  const { theme, toggle: toggleTheme } = useTheme()
  return (
    <header className="titlebar" data-test="titlebar">
      <strong
        className={`titlebar-app titlebar-app--${status}`}
        data-status={status}
        data-test="titlebar-app"
        aria-label={`bpmux ${status === 'connected' ? 'online' : status === 'connecting' ? 'reconnecting' : 'offline'}`}
      >
        bpmux
        <small className="titlebar-app-state" data-test="titlebar-app-state">
          {status === 'connected' ? 'online' : status === 'connecting' ? 'reconnecting' : 'offline'}
        </small>
      </strong>
      <button
        type="button"
        className="titlebar-settings"
        data-test="titlebar-settings"
        onClick={onOpenSettings}
        aria-label="Open settings"
        title="Settings"
      >
        <GearIcon />
      </button>
      <button
        type="button"
        className="titlebar-theme-toggle"
        onClick={toggleTheme}
        aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
      >
        {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
      </button>
      <output data-test="titlebar-ci">
        <CiGlyph />
      </output>
    </header>
  )
}

export default Titlebar
