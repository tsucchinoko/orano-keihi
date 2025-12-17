// セキュリティ関連のユーティリティ関数

import { invoke } from "@tauri-apps/api/core";
import type {
	SystemDiagnosticInfo,
	EnvironmentInfo,
	R2DiagnosticInfo,
	SecurityEvent,
} from "../types";

/**
 * システム診断情報を取得
 */
export async function getSystemDiagnosticInfo(): Promise<SystemDiagnosticInfo> {
	try {
		const result = await invoke<Record<string, string>>(
			"get_system_diagnostic_info",
		);
		return result as unknown as SystemDiagnosticInfo;
	} catch (error) {
		console.error("システム診断情報の取得に失敗しました:", error);
		throw error;
	}
}

/**
 * セキュリティ設定の検証
 */
export async function validateSecurityConfiguration(): Promise<boolean> {
	try {
		const result = await invoke<boolean>("validate_security_configuration");
		return result;
	} catch (error) {
		console.error("セキュリティ設定の検証に失敗しました:", error);
		throw error;
	}
}

/**
 * セキュアなR2接続テスト
 */
export async function testR2ConnectionSecure(): Promise<boolean> {
	try {
		const result = await invoke<boolean>("test_r2_connection_secure");
		return result;
	} catch (error) {
		console.error("R2接続テストに失敗しました:", error);
		throw error;
	}
}

/**
 * 環境情報を取得
 */
export async function getEnvironmentInfo(): Promise<EnvironmentInfo> {
	try {
		const result = await invoke<Record<string, string>>("get_environment_info");
		return result as unknown as EnvironmentInfo;
	} catch (error) {
		console.error("環境情報の取得に失敗しました:", error);
		throw error;
	}
}

/**
 * セキュリティイベントをログに記録
 */
export async function logSecurityEvent(
	eventType: string,
	details: string,
): Promise<void> {
	try {
		await invoke("log_security_event", {
			eventType,
			details,
		});
	} catch (error) {
		console.error("セキュリティイベントのログ記録に失敗しました:", error);
		throw error;
	}
}

/**
 * R2診断情報を取得
 */
export async function getR2DiagnosticInfo(): Promise<R2DiagnosticInfo> {
	try {
		const result = await invoke<Record<string, string>>(
			"get_r2_diagnostic_info",
		);
		return result as unknown as R2DiagnosticInfo;
	} catch (error) {
		console.error("R2診断情報の取得に失敗しました:", error);
		throw error;
	}
}

/**
 * 認証情報をマスク（フロントエンド用）
 */
export function maskCredential(credential: string): string {
	if (!credential || credential.length <= 8) {
		return "****";
	}

	return `${credential.slice(0, 4)}****${credential.slice(-4)}`;
}

/**
 * セキュリティ状態を確認
 */
export async function checkSecurityStatus(): Promise<{
	isValid: boolean;
	environment: string;
	isProduction: boolean;
	diagnosticInfo: SystemDiagnosticInfo;
}> {
	try {
		const [isValid, envInfo, diagnosticInfo] = await Promise.all([
			validateSecurityConfiguration(),
			getEnvironmentInfo(),
			getSystemDiagnosticInfo(),
		]);

		return {
			isValid,
			environment: envInfo.environment,
			isProduction: envInfo.is_production === "true",
			diagnosticInfo,
		};
	} catch (error) {
		console.error("セキュリティ状態の確認に失敗しました:", error);
		throw error;
	}
}

/**
 * R2接続状態を確認
 */
export async function checkR2Status(): Promise<{
	isConnected: boolean;
	diagnosticInfo?: R2DiagnosticInfo;
	error?: string;
}> {
	try {
		const [isConnected, diagnosticInfo] = await Promise.all([
			testR2ConnectionSecure(),
			getR2DiagnosticInfo(),
		]);

		return {
			isConnected,
			diagnosticInfo,
		};
	} catch (error) {
		console.error("R2接続状態の確認に失敗しました:", error);
		return {
			isConnected: false,
			error: error instanceof Error ? error.message : String(error),
		};
	}
}

/**
 * デバッグ情報を安全に表示（認証情報をマスク）
 */
export function formatDebugInfo(
	info: Record<string, string>,
): Record<string, string> {
	const masked: Record<string, string> = {};

	for (const [key, value] of Object.entries(info)) {
		// 認証情報と思われるキーをマスク
		if (
			key.toLowerCase().includes("key") ||
			key.toLowerCase().includes("secret") ||
			key.toLowerCase().includes("token") ||
			key.toLowerCase().includes("password")
		) {
			masked[key] = maskCredential(value);
		} else {
			masked[key] = value;
		}
	}

	return masked;
}

/**
 * セキュリティイベントのヘルパー関数
 */
export const SecurityEvents = {
	/**
	 * ページアクセスをログ記録
	 */
	async logPageAccess(pageName: string): Promise<void> {
		await logSecurityEvent("page_access", `ページアクセス: ${pageName}`);
	},

	/**
	 * ファイルアップロード開始をログ記録
	 */
	async logUploadStart(fileName: string, fileSize: number): Promise<void> {
		await logSecurityEvent(
			"upload_start",
			`ファイルアップロード開始: ${fileName} (${fileSize} bytes)`,
		);
	},

	/**
	 * ファイルアップロード完了をログ記録
	 */
	async logUploadComplete(fileName: string, receiptUrl: string): Promise<void> {
		await logSecurityEvent(
			"upload_complete",
			`ファイルアップロード完了: ${fileName} -> ${receiptUrl}`,
		);
	},

	/**
	 * エラーをログ記録
	 */
	async logError(errorType: string, errorMessage: string): Promise<void> {
		await logSecurityEvent("error", `${errorType}: ${errorMessage}`);
	},

	/**
	 * 設定変更をログ記録
	 */
	async logConfigChange(configType: string, details: string): Promise<void> {
		await logSecurityEvent("config_change", `${configType}: ${details}`);
	},
};
