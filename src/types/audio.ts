export type AudioJobStatus =
  | 'created'
  | 'waiting_login'
  | 'preparing'
  | 'generating'
  | 'downloading'
  | 'importing'
  | 'succeeded'
  | 'waiting_user'
  | 'paused'
  | 'failed'
  | 'cancelled'

export type AudioFormat = 'deep_dive' | 'brief'

export interface AudioJob {
  id: string
  projectId: string
  projectTitle: string
  writingVersionId: string | null
  status: AudioJobStatus
  stage: string
  runId: number
  errorCode: string | null
  errorMessage: string | null
  notebookUrl: string | null
  remoteAudioRef: string | null
  outputSize: number | null
  outputSha256: string | null
  assetId: string | null
  format: AudioFormat
  sourceLabel: string
  targetDurationSeconds: number
  submittedAt: string | null
  lastCheckedAt: string | null
  createdAt: string
  updatedAt: string
}

export interface AudioBridgeStatus {
  supported: boolean
  connected: boolean
  lastSeenSecondsAgo: number | null
  hostName: string
  message: string | null
}

export interface AudioJobView {
  job: AudioJob | null
  activeJob: AudioJob | null
  bridge: AudioBridgeStatus
}

export type AudioSourceInput = { kind: 'asset', assetId: string } | { kind: 'text' }

export interface StartAudioJobInput {
  projectId: string
  writingVersionId: string
  source: AudioSourceInput
  format: AudioFormat
}

export type AudioJobAction = 'pause' | 'resume' | 'cancel' | 'open_browser'
