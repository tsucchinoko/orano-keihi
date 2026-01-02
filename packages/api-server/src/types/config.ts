/**
 * 設定関連の型定義
 */

export interface R2Config {
  endpoint: string;
  accessKeyId: string;
  secretAccessKey: string;
  bucketName: string;
  region: string;
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
  id: number;
  googleId: string;
  email: string;
  name: string;
  pictureUrl?: string;
  createdAt: string;
  updatedAt: string;
}

export interface Session {
  id: string;
  userId: number;
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
