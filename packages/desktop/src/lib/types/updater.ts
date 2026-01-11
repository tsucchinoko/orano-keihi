/**
 * アップデート情報の型定義
 */
export interface UpdateInfo {
  /** 利用可能なアップデートがあるかどうか */
  available: boolean;
  /** 現在のバージョン */
  current_version: string;
  /** 最新バージョン */
  latest_version?: string;
  /** アップデートの詳細情報 */
  release_notes?: string;
  /** アップデートのサイズ（バイト） */
  content_length?: number;
  /** 最後にチェックした時刻（Unix timestamp） */
  last_checked: number;
}

/**
 * アップデーター設定の型定義
 */
export interface UpdaterConfig {
  /** 自動アップデートチェックの有効/無効 */
  auto_check_enabled: boolean;
  /** アップデートチェックの頻度（時間単位） */
  check_interval_hours: number;
  /** ベータ版アップデートの受信可否 */
  include_prereleases: boolean;
  /** スキップされたバージョンのリスト */
  skipped_versions: string[];
  /** 最後にチェックした時刻（Unix timestamp） */
  last_check_time?: number;
}

/**
 * アップデート通知の状態
 */
export interface UpdateNotificationState {
  /** 通知を表示するかどうか */
  show: boolean;
  /** アップデート情報 */
  updateInfo?: UpdateInfo;
  /** ダウンロード中かどうか */
  downloading: boolean;
  /** ダウンロード進捗（0-100） */
  progress: number;
  /** エラーメッセージ */
  error?: string;
}
