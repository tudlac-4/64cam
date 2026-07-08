import { Navigate, Route, Routes } from 'react-router-dom'
import { Login }     from './pages/Login'
import { Dashboard } from './pages/Dashboard'
import { Playback }  from './pages/Playback'
import { useAuthStore } from './store/auth'

export function App() {
  const token = useAuthStore((s) => s.token)

  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route
        path="/"
        element={token ? <Dashboard /> : <Navigate to="/login" replace />}
      />
      <Route
        path="/playback/:cameraId"
        element={token ? <Playback /> : <Navigate to="/login" replace />}
      />
      {/* Catch-all for deep links */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
