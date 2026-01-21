/**
 * 設定関連の型定義
 */

export interface R2Config {
  endpoint: string;
  accessKeyId: string;
  secretAccessKey: string;
  bucketName: string;
  region: string;
  publicDomain?: string;
}

export interface CorsConfig {
  origin: string[];
  methods: string[];
  headers: string[];
}

export interface AuthConfig {
  jwtSecret: string;
  sessionEncryptionKey: string;
  sessionExpirationDays: number;
}

export interface FileUploadConfig {
  maxFileSize: number;
  allowedTypes: string[];
  maxFiles: number;
}

export interface RateLimitConfig {
  windowMs: number;
  maxRequests: number;
}

export interface LoggingConfig {
  level: "error" | "warn" | "info" | "debug";
  file: string;
}

export interface ApiServerConfig {
  port: number;
  host: string;
  nodeEnv: "development" | "production" | "test";
  cors: CorsConfig;
  r2: R2Config;
  auth: AuthConfig;
  fileUpload: FileUploadConfig;
  rateLimit: RateLimitConfig;
  logging: LoggingConfig;
}
/**
 * 認証関連の型定義
 */

export interface User {
  id: string; // nanoIdに変更
  googleId: string;
  email: string;
  name: string;
  pictureUrl?: string;
  createdAt: string;
  updatedAt: string;
}

export interface Session {
  id: string;
  userId: string; // nanoIdに変更
  expiresAt: string;
  createdAt: string;
}

export interface AuthResult {
  success: boolean;
  user?: User;
  error?: string;
}

export interface ValidationResult {
  isValid: boolean;
  user?: User;
  error?: string;
}

/**
 * ファイルアップロード関連の型定義
 */

export interface UploadMetadata {
  expenseId: number;
  userId: string; // nanoIdに変更
  description?: string;
  category?: string;
  type?: "expense" | "subscription"; // アップロードタイプ（経費または定期支払い）
}

export interface UploadRequest {
  file: File;
  expenseId: number;
  userId: string; // nanoIdに変更
  metadata?: {
    description?: string;
    category?: string;
  };
}

export interface UploadResponse {
  success: boolean;
  fileUrl?: string;
  fileKey: string;
  fileSize: number;
  contentType: string;
  uploadedAt: string;
  error?: string;
}

export interface MultipleUploadResponse {
  totalFiles: number;
  successfulUploads: number;
  failedUploads: number;
  results: UploadResponse[];
  totalDurationMs: number;
}

export interface FileValidationResult {
  isValid: boolean;
  error?: string;
  details?: {
    field?: string;
    value?: any;
    constraint?: string;
  };
}

export interface ErrorResponse {
  error: {
    code: string;
    message: string;
    details?: any;
    timestamp: string;
    requestId: string;
  };
}

/**
 * サブスクリプション関連の型定義
 */

export interface Subscription {
  id: number;
  userId: string; // nanoIdに変更
  name: string;
  amount: number;
  billing_cycle: "monthly" | "annual";
  start_date: string;
  category: string;
  is_active: boolean;
  receipt_path?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateSubscriptionDto {
  name: string;
  amount: number;
  billing_cycle: "monthly" | "annual";
  start_date: string;
  category: string;
  receipt_path?: string;
}

export interface UpdateSubscriptionDto {
  name?: string;
  amount?: number;
  billing_cycle?: "monthly" | "annual";
  start_date?: string;
  category?: string;
  receipt_path?: string;
}

export interface SubscriptionListResponse {
  subscriptions: Subscription[];
  total: number;
  activeCount: number;
  monthlyTotal: number;
}

export interface MonthlyTotalResponse {
  monthlyTotal: number;
  activeSubscriptions: number;
}
