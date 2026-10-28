export interface SteamInstallation { kind: string; path: string }
export interface SteamLibrary { path: string; steamAppsPath: string; installKind: string }
export interface Game {
  appId: string;
  name: string;
  libraryPath: string;
  installPath: string;
  installPathExists: boolean;
  manifestPath: string;
  prefixPath: string;
  prefixExists: boolean;
  buildId: string | null;
  configuredTool?: string | null;
}
export interface CompatibilityTool { name: string; path: string; source: string }
export interface ScanReport {
  installations: SteamInstallation[];
  libraries: SteamLibrary[];
  games: Game[];
  compatibilityTools: CompatibilityTool[];
  warnings: string[];
}
export type Severity = 'error' | 'warning' | 'info';
export interface Finding {
  code: string;
  severity: Severity;
  title: string;
  description: string;
  recommendation: string;
  evidence: string[];
}
export type DiagnosticStatus = 'no-log' | 'review' | 'no-signatures' | 'unreadable';
export interface GameDiagnostics {
  appId: string;
  libraryPath: string;
  logPath: string | null;
  logSize: number | null;
  modifiedUnixSeconds: number | null;
  truncated: boolean;
  status: DiagnosticStatus;
  findings: Finding[];
  notes: string[];
}
export interface SystemInfo {
  os: string;
  kernel: string;
  arch: string;
  gpuDevices: string[];
  vulkaninfoPath: string | null;
  icdManifests: string[];
  notes: string[];
}
export interface DiagnosticsReport { games: GameDiagnostics[]; system: SystemInfo; warnings: string[] }
export interface VulkanProbe {
  status: 'available' | 'unavailable' | 'timeout' | 'failed';
  exitCode: number | null;
  output: string;
  detail: string;
}
export type Panel = 'overview' | 'games' | 'diagnostics' | 'proton' | 'libraries' | 'warnings' | 'about';

export type RepairAction = 'shader-cache' | 'proton-prefix';
export interface RepairPlan {
  appId: string;
  libraryPath: string;
  gameName: string;
  action: RepairAction;
  title: string;
  explanation: string;
  sourcePath: string;
  backupRoot: string;
  risk: 'moderate' | 'high';
  canApply: boolean;
  blocker: string | null;
}
export interface RepairBackup {
  name: string;
  appId: string;
  libraryPath: string;
  action: RepairAction;
  backupPath: string;
  originalPath: string;
  restoredTargetExists: boolean;
}
export interface RepairOutcome {
  message: string;
  backup: RepairBackup;
  displacedBackup: RepairBackup | null;
}

export interface VersionAdvice {
  appId: string;
  libraryPath: string;
  gameName: string;
  status: 'keep-current' | 'steam-default' | 'try-version' | 'need-proton' | 'check-system' | 'check-install' | 'need-information';
  suggestedTool: string | null;
  configuredTool: string | null;
  reason: string;
  nextStep: string;
  confidence: 'limited' | 'tentative';
  alternatives: string[];
  evidence: string[];
}
export interface AdvisorReport { games: VersionAdvice[]; note: string }

export interface AutoSelection {
  gameName: string;
  appId: string;
  toolName: string;
  toolId: string;
  message: string;
  backupPath: string;
  changed: boolean;
}

export interface LaunchResult {
  appId: string;
  libraryPath: string;
  gameName: string;
  status: 'log-error' | 'log-warning' | 'launcher-error' | 'log-unreadable' | 'log-no-known-errors' | 'no-fresh-log';
  detail: string;
  observedSeconds: number;
  launcherExitCode: number | null;
  freshLog: boolean;
  findingCodes: string[];
  manuallyConfirmed: boolean;
}
