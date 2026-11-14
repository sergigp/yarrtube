import { Navigate, Route } from 'react-router-dom'
import { AiSettingsView } from './AiSettingsView'
import { PlexSettingsView } from './PlexSettingsView'
import { PreferencesView } from './PreferencesView'
import { SettingsView } from './SettingsView'

/**
 * The Settings page's routes: one per section under `/settings`, plus the
 * old `/preferences` and `/ai` pages redirecting to theirs.
 */
export function settingsRoutes() {
  return [
    <Route key="settings" path="/settings" element={<SettingsView />}>
      <Route index element={<Navigate to="general" replace />} />
      <Route path="general" element={<PreferencesView />} />
      <Route path="ai" element={<AiSettingsView />} />
      <Route path="plex" element={<PlexSettingsView />} />
    </Route>,
    <Route
      key="preferences"
      path="/preferences"
      element={<Navigate to="/settings/general" replace />}
    />,
    <Route key="ai" path="/ai" element={<Navigate to="/settings/ai" replace />} />,
  ]
}
