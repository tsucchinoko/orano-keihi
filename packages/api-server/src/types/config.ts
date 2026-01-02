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
